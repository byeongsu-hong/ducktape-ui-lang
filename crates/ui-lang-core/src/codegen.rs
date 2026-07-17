use crate::Error;
use crate::ast::*;
use crate::check::expr_type;
use std::collections::HashMap;
use std::fmt::Write;

pub fn generate(document: &Document, source_path: &str) -> Result<String, Error> {
    let message = format!("__{}Message", document.app);
    let mut out = String::new();
    writeln!(
        out,
        "const _: &str = include_str!({});",
        rust_string(source_path)
    )
    .unwrap();

    writeln!(out, "#[derive(Debug)]\npub struct {} {{", document.app).unwrap();
    for state in &document.states {
        writeln!(
            out,
            "pub(crate) {}: {},",
            state.name,
            state.ty.rust(&document.structs)
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();

    writeln!(out, "#[derive(Debug, Clone)]\nenum {message} {{").unwrap();
    for handler in &document.handlers {
        if handler.name == "mount" {
            continue;
        }
        let variant = pascal(&handler.name);
        if handler.params.is_empty() {
            writeln!(out, "{variant},").unwrap();
        } else {
            let fields = handler
                .params
                .iter()
                .map(|param| param.ty.rust(&document.structs))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "{variant}({fields}),").unwrap();
        }
    }
    for binding in input_bindings(&document.view) {
        writeln!(out, "{}(::std::string::String),", binding_variant(&binding)).unwrap();
    }
    if needs_extern_noop(document) {
        writeln!(out, "__ExternNoop,").unwrap();
    }
    writeln!(out, "}}").unwrap();

    generate_extern_probes(&mut out, document);
    writeln!(out, "impl {} {{", document.app).unwrap();
    writeln!(out, "pub fn run() -> ::iced::Result {{").unwrap();
    let subscription = if document.subscriptions.is_empty() {
        ""
    } else {
        ".subscription(Self::__subscription)"
    };
    writeln!(out, "::iced::application(Self::__boot, Self::__update, Self::__view){subscription}.theme(Self::__theme).run()").unwrap();
    writeln!(out, "}}").unwrap();

    generate_theme(&mut out, document);
    generate_boot(&mut out, document, &message)?;
    generate_update(&mut out, document, &message)?;
    generate_subscription(&mut out, document, &message)?;
    generate_view(&mut out, document, &message)?;
    writeln!(out, "}}").unwrap();
    Ok(out)
}

fn generate_extern_probes(out: &mut String, document: &Document) {
    for item in &document.structs {
        writeln!(
            out,
            "#[allow(dead_code)] fn __ui_lang_check_{}(value: &{}) {{",
            item.name.to_ascii_lowercase(),
            item.rust_path
        )
        .unwrap();
        for (field, ty) in &item.fields {
            writeln!(
                out,
                "let _: &{} = &value.{field};",
                ty.rust(&document.structs)
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
    }
    for item in &document.functions {
        let params = item
            .params
            .iter()
            .enumerate()
            .map(|(index, (_, ty))| format!("arg{index}: {}", ty.rust(&document.structs)))
            .collect::<Vec<_>>()
            .join(", ");
        let args = (0..item.params.len())
            .map(|index| format!("arg{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let output = item.error.as_ref().map_or_else(
            || item.output.rust(&document.structs),
            |error| {
                format!(
                    "::std::result::Result<{}, {}>",
                    item.output.rust(&document.structs),
                    error.rust(&document.structs)
                )
            },
        );
        match item.kind {
            ExternKind::Future => writeln!(
                out,
                "#[allow(dead_code)] async fn __ui_lang_check_{}({params}) {{ let _: {output} = {}({args}).await; }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Component => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_component_{}({params}) {{ let _: ::iced::Element<'static, {output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Task => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_task_{}({params}) {{ let _: ::iced::Task<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Subscription => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_subscription_{}({params}) {{ let _: ::iced::Subscription<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
        }
    }
}

fn generate_theme(out: &mut String, document: &Document) {
    let color = |name: &str, fallback: &str| {
        color_code(
            document
                .theme
                .get(name)
                .map(String::as_str)
                .unwrap_or(fallback),
            None,
        )
    };
    writeln!(out, "fn __theme(&self) -> ::iced::Theme {{").unwrap();
    writeln!(
        out,
        "::iced::Theme::custom(\"{}\", ::iced::theme::Palette {{",
        document.app
    )
    .unwrap();
    writeln!(out, "background: {},", color("background", "#000000")).unwrap();
    writeln!(out, "text: {},", color("foreground", "#ffffff")).unwrap();
    writeln!(out, "primary: {},", color("primary", "#5865f2")).unwrap();
    writeln!(out, "success: {},", color("primary", "#5865f2")).unwrap();
    writeln!(out, "warning: {},", color("danger", "#c3423f")).unwrap();
    writeln!(out, "danger: {},", color("danger", "#c3423f")).unwrap();
    writeln!(out, "}})\n}}").unwrap();
}

fn generate_boot(out: &mut String, document: &Document, message: &str) -> Result<(), Error> {
    writeln!(
        out,
        "fn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self {{"
    )
    .unwrap();
    for state in &document.states {
        writeln!(
            out,
            "{}: {},",
            state.name,
            initial_code(&state.initial, &state.ty, document)
        )
        .unwrap();
    }
    writeln!(out, "}};").unwrap();
    if let Some(mount) = document
        .handlers
        .iter()
        .find(|handler| handler.name == "mount")
    {
        let env = state_env(document, "state");
        writeln!(out, "let task = (|| {{").unwrap();
        let has_task = generate_statements(
            out,
            &mount.statements,
            document,
            message,
            &env,
            "state",
            false,
        )?;
        if !has_task {
            writeln!(out, "::iced::Task::none()").unwrap();
        }
        writeln!(out, "}})();").unwrap();
    } else {
        writeln!(out, "let task = ::iced::Task::none();").unwrap();
    }
    writeln!(out, "(state, task)\n}}").unwrap();
    Ok(())
}

fn generate_update(out: &mut String, document: &Document, message: &str) -> Result<(), Error> {
    writeln!(
        out,
        "fn __update(&mut self, message: {message}) -> ::iced::Task<{message}> {{\nmatch message {{"
    )
    .unwrap();
    for handler in &document.handlers {
        if handler.name == "mount" {
            continue;
        }
        let variant = pascal(&handler.name);
        let pattern = if handler.params.is_empty() {
            format!("{message}::{variant}")
        } else {
            format!(
                "{message}::{variant}({})",
                handler
                    .params
                    .iter()
                    .map(|param| param.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        writeln!(out, "{pattern} => {{").unwrap();
        let mut env = state_env(document, "self");
        for param in &handler.params {
            env.insert(
                param.name.clone(),
                Binding {
                    code: param.name.clone(),
                    ty: param.ty.clone(),
                    local: true,
                },
            );
        }
        let has_task = generate_statements(
            out,
            &handler.statements,
            document,
            message,
            &env,
            "self",
            true,
        )?;
        if !has_task {
            writeln!(out, "::iced::Task::none()").unwrap();
        }
        writeln!(out, "}}").unwrap();
    }
    for binding in input_bindings(&document.view) {
        let variant = binding_variant(&binding);
        writeln!(
            out,
            "{message}::{variant}(value) => {{ self.{binding} = value; ::iced::Task::none() }}"
        )
        .unwrap();
    }
    if needs_extern_noop(document) {
        writeln!(out, "{message}::__ExternNoop => ::iced::Task::none(),").unwrap();
    }
    writeln!(out, "}}\n}}").unwrap();
    Ok(())
}

fn generate_subscription(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    if document.subscriptions.is_empty() {
        return Ok(());
    }
    let env = state_env(document, "self");
    writeln!(
        out,
        "fn __subscription(&self) -> ::iced::Subscription<{message}> {{"
    )
    .unwrap();
    writeln!(out, "::iced::Subscription::batch([").unwrap();
    for subscription in &document.subscriptions {
        let source = document
            .functions
            .iter()
            .find(|item| {
                item.name == subscription.function && item.kind == ExternKind::Subscription
            })
            .ok_or_else(|| {
                Error::new(
                    "E130",
                    &subscription.span,
                    format!("unknown extern subscription `{}`", subscription.function),
                )
            })?;
        let args = subscription
            .args
            .iter()
            .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        let route = route_code(&subscription.route, "__value", &env, document, message)?;
        writeln!(
            out,
            "{}({args}).map(move |__value| {route}),",
            source.rust_path
        )
        .unwrap();
    }
    writeln!(out, "])\n}}").unwrap();
    Ok(())
}

fn generate_view(out: &mut String, document: &Document, message: &str) -> Result<(), Error> {
    let env = state_env(document, "self");
    let root = render_node(
        &document.view,
        document,
        message,
        &env,
        &rust_string(&document.app),
    )?;
    writeln!(
        out,
        "fn __view(&self) -> ::iced::Element<'_, {message}> {{ {root} }}"
    )
    .unwrap();
    Ok(())
}

fn generate_statements(
    out: &mut String,
    statements: &[Statement],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    state: &str,
    return_task: bool,
) -> Result<bool, Error> {
    let mut has_task = false;
    for statement in statements {
        match statement {
            Statement::Assign { target, value, .. } => {
                let code = expr_code(value, env, document, ValueMode::Owned)?;
                writeln!(out, "{state}.{target} = {code};").unwrap();
            }
            Statement::ReturnIf { condition, .. } => {
                let code = expr_code(condition, env, document, ValueMode::Owned)?;
                writeln!(out, "if {code} {{ return ::iced::Task::none(); }}").unwrap();
            }
            Statement::Run {
                kind,
                function,
                args,
                success,
                error,
                span,
            } => {
                has_task = true;
                let extern_kind = match kind {
                    EffectKind::Future => ExternKind::Future,
                    EffectKind::Task => ExternKind::Task,
                };
                let action = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == extern_kind)
                    .ok_or_else(|| {
                        Error::new("E130", span, format!("unknown extern fn `{function}`"))
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let success_message = route_code(success, "value", env, document, message)?;
                if let (Some(error_route), Some(_)) = (error, &action.error) {
                    let error_message = route_code(error_route, "error", env, document, message)?;
                    match kind {
                        EffectKind::Future => writeln!(out, "{}::iced::Task::perform({}({args}), |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){}", if return_task { "return " } else { "" }, action.rust_path, if return_task { ";" } else { "" }).unwrap(),
                        EffectKind::Task => writeln!(out, "{}{}({args}).map(|result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){}", if return_task { "return " } else { "" }, action.rust_path, if return_task { ";" } else { "" }).unwrap(),
                    }
                } else {
                    match kind {
                        EffectKind::Future => writeln!(
                            out,
                            "{}::iced::Task::perform({}({args}), |value| {success_message}){}",
                            if return_task { "return " } else { "" },
                            action.rust_path,
                            if return_task { ";" } else { "" }
                        )
                        .unwrap(),
                        EffectKind::Task => writeln!(
                            out,
                            "{}{}({args}).map(|value| {success_message}){}",
                            if return_task { "return " } else { "" },
                            action.rust_path,
                            if return_task { ";" } else { "" }
                        )
                        .unwrap(),
                    }
                }
            }
        }
    }
    Ok(has_task)
}

fn render_node(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
) -> Result<String, Error> {
    match node {
        ViewNode::Layout {
            kind,
            options,
            id,
            styles,
            children,
            ..
        } => render_layout(
            *kind, options, id, styles, children, document, message, env, scope,
        ),
        ViewNode::Text { value, styles, .. } => {
            let style = Style::parse(styles, document);
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let mut code = format!("::iced::widget::text({value})");
            if let Some(size) = style.text_size {
                write!(code, ".size({size})").unwrap();
            }
            if style.bold {
                code.push_str(".font(::iced::Font { weight: ::iced::font::Weight::Bold, ..::iced::Font::default() })");
            }
            if let Some(color) = style.text_color {
                write!(code, ".color({})", theme_color(document, &color)).unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Input {
            label,
            id,
            binding,
            hint,
            disabled,
            styles,
            ..
        } => {
            let style = Style::parse(styles, document);
            let variant = binding_variant(binding);
            let mut input = format!(
                "::iced::widget::text_input({}, &self.{binding})",
                rust_string(hint)
            );
            if let Some(id) = id {
                write!(
                    input,
                    ".id(::iced::widget::Id::from({}))",
                    id_code(id, scope, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = style.padding_code() {
                write!(input, ".padding({padding})").unwrap();
            }
            if style.width_fill {
                input.push_str(".width(::iced::Fill)");
            }
            let constructor =
                format!("{message}::{variant} as fn(::std::string::String) -> {message}");
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(
                    input,
                    ".on_input_maybe(if {disabled} {{ None }} else {{ Some({constructor}) }})"
                )
                .unwrap();
            } else {
                write!(input, ".on_input({constructor})").unwrap();
            }
            input.push_str(&input_style_code(&style, document));
            Ok(format!(
                "::iced::widget::column![::iced::widget::text({}), {input}].spacing(6).into()",
                rust_string(label)
            ))
        }
        ViewNode::Button {
            label,
            disabled,
            styles,
            route,
            ..
        } => {
            let style = Style::parse(styles, document);
            let message_code = route_code(route, "", env, document, message)?;
            let mut code = format!(
                "::iced::widget::button(::iced::widget::text({}))",
                rust_string(label)
            );
            if let Some(padding) = style.padding_code() {
                write!(code, ".padding({padding})").unwrap();
            }
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    ".on_press_maybe(if {disabled} {{ None }} else {{ Some({message_code}) }})"
                )
                .unwrap();
            } else {
                write!(code, ".on_press({message_code})").unwrap();
            }
            code.push_str(&button_style_code(&style, document));
            Ok(format!("{code}.into()"))
        }
        ViewNode::Checkbox {
            label,
            checked,
            disabled,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!("::iced::widget::checkbox({checked}).label({label})");
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    ".on_toggle_maybe(if {disabled} {{ None }} else {{ Some(move |__value| {message_code}) }})"
                )
                .unwrap();
            } else {
                write!(code, ".on_toggle(move |__value| {message_code})").unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Toggler {
            label,
            checked,
            disabled,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!("::iced::widget::toggler({checked}).label({label})");
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(code, ".on_toggle_maybe(if {disabled} {{ None }} else {{ Some(move |__value| {message_code}) }})").unwrap();
            } else {
                write!(code, ".on_toggle(move |__value| {message_code})").unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Slider {
            value,
            min,
            max,
            step,
            vertical,
            route,
            release,
            ..
        } => {
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let min = expr_code(min, env, document, ValueMode::Owned)?;
            let max = expr_code(max, env, document, ValueMode::Owned)?;
            let step = expr_code(step, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let helper = if *vertical {
                "vertical_slider"
            } else {
                "slider"
            };
            let mut code = format!(
                "::iced::widget::{helper}(({min})..=({max}), {value}, move |__value| {message_code}).step({step})"
            );
            if let Some(release) = release {
                write!(
                    code,
                    ".on_release({})",
                    route_code(release, "", env, document, message)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Progress {
            value,
            min,
            max,
            vertical,
            ..
        } => {
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let min = expr_code(min, env, document, ValueMode::Owned)?;
            let max = expr_code(max, env, document, ValueMode::Owned)?;
            let vertical = if *vertical { ".vertical()" } else { "" };
            Ok(format!(
                "::iced::widget::progress_bar(({min} as f32)..=({max} as f32), {value} as f32){vertical}.into()"
            ))
        }
        ViewNode::Radio {
            label,
            value,
            selected,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            Ok(format!(
                "::iced::widget::radio({label}, {value}, if {selected} {{ Some({value}) }} else {{ None }}, move |__value| {message_code}).into()"
            ))
        }
        ViewNode::PickList {
            options,
            selected,
            options_config,
            route,
            ..
        } => {
            let options = expr_code(options, env, document, ValueMode::Owned)?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!(
                "::iced::widget::pick_list({options}, {selected}, move |__value| {message_code})"
            );
            if let Some(placeholder) = &options_config.placeholder {
                write!(
                    code,
                    ".placeholder({})",
                    expr_code(placeholder, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(width) = &options_config.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options_config.menu_height {
                write!(
                    code,
                    ".menu_height({})",
                    length_code(height, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = &options_config.padding {
                write!(
                    code,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options_config.text_size {
                write!(
                    code,
                    ".text_size({} as f32)",
                    expr_code(size, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(route) = &options_config.open {
                write!(
                    code,
                    ".on_open({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options_config.close {
                write!(
                    code,
                    ".on_close({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Rule {
            axis, thickness, ..
        } => {
            let thickness = expr_code(thickness, env, document, ValueMode::Owned)?;
            let axis = match axis {
                Axis::Horizontal => "horizontal",
                Axis::Vertical => "vertical",
            };
            Ok(format!(
                "::iced::widget::rule::{axis}({thickness} as f32).into()"
            ))
        }
        ViewNode::Space { width, height, .. } => {
            let mut code = String::from("::iced::widget::space()");
            if let Some(width) = width {
                write!(
                    code,
                    ".width({} as f32)",
                    expr_code(width, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = height {
                write!(
                    code,
                    ".height({} as f32)",
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Component {
            name,
            args,
            id,
            span,
        } => {
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            let mut component_env = HashMap::new();
            for ((param, ty), arg) in component.params.iter().zip(args) {
                component_env.insert(
                    param.clone(),
                    Binding {
                        code: expr_code(arg, env, document, ValueMode::Borrowed)?,
                        ty: ty.clone(),
                        local: false,
                    },
                );
            }
            let component_scope = id.as_ref().map_or_else(
                || format!("format!(\"{{}}/{}\", {scope})", name),
                |id| id_code(id, scope, env, document).unwrap_or_else(|_| scope.into()),
            );
            render_node(
                &component.root,
                document,
                message,
                &component_env,
                &component_scope,
            )
        }
        ViewNode::ExternComponent {
            function,
            args,
            route,
            span,
        } => {
            let component = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Component)
                .ok_or_else(|| {
                    Error::new(
                        "E130",
                        span,
                        format!("unknown extern component `{function}`"),
                    )
                })?;
            let args = args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mapped = if let Some(route) = route {
                route_code(route, "__value", env, document, message)?
            } else {
                format!("{message}::__ExternNoop")
            };
            Ok(format!(
                "{}({args}).map(move |__value| {mapped}).into()",
                component.rust_path
            ))
        }
        ViewNode::Media {
            kind,
            source,
            options,
            ..
        } => {
            let helper = match kind {
                MediaKind::Image => "image",
                MediaKind::Svg => "svg",
            };
            let source = expr_code(source, env, document, ValueMode::Owned)?;
            let mut code = format!("::iced::widget::{helper}({source})");
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            if let Some(fit) = options.fit {
                let fit = match fit {
                    ContentFit::Contain => "Contain",
                    ContentFit::Cover => "Cover",
                    ContentFit::Fill => "Fill",
                    ContentFit::None => "None",
                    ContentFit::ScaleDown => "ScaleDown",
                };
                write!(code, ".content_fit(::iced::ContentFit::{fit})").unwrap();
            }
            if let Some(rotation) = &options.rotation {
                write!(
                    code,
                    ".rotation({} as f32)",
                    expr_code(rotation, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(opacity) = &options.opacity {
                write!(
                    code,
                    ".opacity({} as f32)",
                    expr_code(opacity, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(filter) = options.filter {
                let filter = match filter {
                    ImageFilter::Linear => "Linear",
                    ImageFilter::Nearest => "Nearest",
                };
                write!(
                    code,
                    ".filter_method(::iced::widget::image::FilterMethod::{filter})"
                )
                .unwrap();
            }
            if let Some(scale) = &options.scale {
                write!(
                    code,
                    ".scale({} as f32)",
                    expr_code(scale, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(expand) = &options.expand {
                write!(
                    code,
                    ".expand({})",
                    expr_code(expand, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(radius) = &options.radius {
                write!(
                    code,
                    ".border_radius({} as f32)",
                    expr_code(radius, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Tooltip {
            options,
            content,
            tip,
            ..
        } => {
            let content = render_node(content, document, message, env, scope)?;
            let tip = render_node(tip, document, message, env, scope)?;
            let position = match options.position {
                TooltipPosition::Top => "Top",
                TooltipPosition::Bottom => "Bottom",
                TooltipPosition::Left => "Left",
                TooltipPosition::Right => "Right",
                TooltipPosition::FollowCursor => "FollowCursor",
            };
            let gap = expr_code(&options.gap, env, document, ValueMode::Owned)?;
            let padding = expr_code(&options.padding, env, document, ValueMode::Owned)?;
            let delay = expr_code(&options.delay_ms, env, document, ValueMode::Owned)?;
            let snap = expr_code(&options.snap, env, document, ValueMode::Owned)?;
            Ok(format!(
                "{{ let __tooltip_content: ::iced::Element<'_, {message}> = {content}; let __tooltip_tip: ::iced::Element<'_, {message}> = {tip}; ::iced::widget::tooltip(__tooltip_content, __tooltip_tip, ::iced::widget::tooltip::Position::{position}).gap({gap} as f32).padding({padding} as f32).delay(::std::time::Duration::from_millis({delay} as u64)).snap_within_viewport({snap}).into() }}"
            ))
        }
        ViewNode::MouseArea {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, scope)?;
            let mut code = format!(
                "{{ let __mouse_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::mouse_area(__mouse_content)"
            );
            for (route, method) in [
                (&options.press, "on_press"),
                (&options.release, "on_release"),
                (&options.double_click, "on_double_click"),
                (&options.right_press, "on_right_press"),
                (&options.right_release, "on_right_release"),
                (&options.middle_press, "on_middle_press"),
                (&options.middle_release, "on_middle_release"),
                (&options.enter, "on_enter"),
                (&options.exit, "on_exit"),
            ] {
                if let Some(route) = route {
                    write!(
                        code,
                        ".{method}({})",
                        route_code(route, "", env, document, message)?
                    )
                    .unwrap();
                }
            }
            if let Some(interaction) = options.interaction {
                write!(
                    code,
                    ".interaction(::iced::mouse::Interaction::{})",
                    mouse_interaction_code(interaction)
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::If { span, .. } | ViewNode::For { span, .. } => Err(Error::new(
            "E170",
            span,
            "if and for must be children of a layout node",
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_layout(
    kind: Layout,
    options: &LayoutOptions,
    id: &Option<Id>,
    styles: &[String],
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
) -> Result<String, Error> {
    let style = Style::parse(styles, document);
    if kind == Layout::Scroll {
        let child_scope = id.as_ref().map_or_else(
            || Ok(scope.to_owned()),
            |id| id_code(id, scope, env, document),
        )?;
        let child = render_node(&children[0], document, message, env, &child_scope)?;
        let mut code = String::from("::iced::widget::scrollable(__scroll_content)");
        if let Some(id) = id {
            write!(
                code,
                ".id(::iced::widget::Id::from({}))",
                id_code(id, scope, env, document)?
            )
            .unwrap();
        }
        append_size(&mut code, &style);
        return Ok(format!(
            "{{ let __scroll_content: ::iced::Element<'_, {message}> = {child}; {code}.into() }}"
        ));
    }

    let mut body = String::from("{ let mut __children: ::std::vec::Vec<::iced::Element<'_, ");
    write!(body, "{message}>> = ::std::vec::Vec::new();").unwrap();
    let child_scope = id.as_ref().map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )?;
    render_children(&mut body, children, document, message, env, &child_scope)?;
    let constructor = match kind {
        Layout::Column => "column",
        Layout::Row => "row",
        Layout::Grid => "grid",
        Layout::Stack => "stack",
        Layout::Scroll => unreachable!("scroll returned above"),
    };
    write!(
        body,
        " let __layout = ::iced::widget::{constructor}(__children)"
    )
    .unwrap();
    if let Some(gap) = style.gap {
        write!(body, ".spacing({gap})").unwrap();
    }
    if matches!(kind, Layout::Column | Layout::Row)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    if style.items_center {
        if kind == Layout::Column {
            body.push_str(".align_x(::iced::Center)");
        } else {
            body.push_str(".align_y(::iced::Center)");
        }
    }
    if kind == Layout::Grid
        && let Some(columns) = &options.columns
    {
        write!(
            body,
            ".columns({} as usize)",
            expr_code(columns, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if kind == Layout::Stack {
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        append_size(&mut body, &style);
    }
    body.push(';');
    body.push_str(" let __content = ::iced::widget::container(__layout)");
    if matches!(kind, Layout::Grid | Layout::Stack)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    append_size(&mut body, &style);
    if let Some(max_width) = style.max_width {
        write!(body, ".max_width({max_width})").unwrap();
    }
    body.push_str(&container_style_code(&style, document));
    body.push(';');
    if style.self_center {
        body.push_str(" ::iced::widget::container(__content).width(::iced::Fill).center_x(::iced::Fill).into() }");
    } else {
        body.push_str(" __content.into() }");
    }
    Ok(body)
}

fn render_children(
    out: &mut String,
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
) -> Result<(), Error> {
    for child in children {
        match child {
            ViewNode::If {
                condition,
                children,
                ..
            } => {
                let condition = expr_code(condition, env, document, ValueMode::Owned)?;
                write!(out, " if {condition} {{").unwrap();
                render_children(out, children, document, message, env, scope)?;
                out.push_str(" }");
            }
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                let Type::List(inner) = expr_type(
                    items,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?
                else {
                    return Err(Error::new("E121", span, "for expects a list"));
                };
                let items = expr_code(items, env, document, ValueMode::Borrowed)?;
                write!(out, " for {item} in {items}.iter() {{").unwrap();
                let mut child_env = env.clone();
                child_env.insert(
                    item.clone(),
                    Binding {
                        code: item.clone(),
                        ty: *inner,
                        local: false,
                    },
                );
                render_children(out, children, document, message, &child_env, scope)?;
                out.push_str(" }");
            }
            _ => {
                let child = render_node(child, document, message, env, scope)?;
                write!(out, " __children.push({child});").unwrap();
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
struct Binding {
    code: String,
    ty: Type,
    local: bool,
}

#[derive(Clone, Copy)]
enum ValueMode {
    Owned,
    Borrowed,
}

fn state_env(document: &Document, name: &str) -> HashMap<String, Binding> {
    document
        .states
        .iter()
        .map(|state| {
            (
                state.name.clone(),
                Binding {
                    code: format!("{name}.{}", state.name),
                    ty: state.ty.clone(),
                    local: false,
                },
            )
        })
        .collect()
}

fn expr_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
    mode: ValueMode,
) -> Result<String, Error> {
    Ok(match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::I64(value) => value.to_string(),
        Expr::F64(value) => rust_f64(*value),
        Expr::Str(value) => match mode {
            ValueMode::Owned => format!("{}.to_owned()", rust_string(value)),
            ValueMode::Borrowed => rust_string(value),
        },
        Expr::EmptyList => "::std::vec::Vec::new()".into(),
        Expr::List(values) => format!(
            "::std::vec![{}]",
            values
                .iter()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Expr::None => "::std::option::Option::None".into(),
        Expr::Path(path) => {
            let binding = env.get(&path[0]).ok_or_else(|| {
                Error::new(
                    "E150",
                    &Span::line(1),
                    format!("unknown value `{}`", path[0]),
                )
            })?;
            let mut code = binding.code.clone();
            let mut ty = binding.ty.clone();
            for field in &path[1..] {
                write!(code, ".{field}").unwrap();
                if let Type::Named(name) = ty {
                    ty = document
                        .structs
                        .iter()
                        .find(|item| item.name == name)
                        .and_then(|item| item.fields.iter().find(|(name, _)| name == field))
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(Type::Unknown);
                }
            }
            if matches!(mode, ValueMode::Owned)
                && !matches!(ty, Type::Bool | Type::I64 | Type::F64 | Type::Unit)
                && !(binding.local && path.len() == 1)
            {
                code.push_str(".clone()");
            }
            code
        }
        Expr::Call { name, args } => match name.as_str() {
            "len" => format!(
                "({}).len() as i64",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "empty" => format!(
                "({}).is_empty()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "trim" => format!(
                "({}).trim().to_owned()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "some" => format!(
                "::std::option::Option::Some({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            _ => unreachable!("checker rejects unknown calls"),
        },
        Expr::Unary { op, value } => format!(
            "({}{})",
            match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            },
            expr_code(value, env, document, ValueMode::Owned)?
        ),
        Expr::Binary { left, op, right } => {
            let mode = if matches!(
                op,
                BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
            ) {
                ValueMode::Borrowed
            } else {
                ValueMode::Owned
            };
            format!(
                "({} {} {})",
                expr_code(left, env, document, mode)?,
                match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Eq => "==",
                    BinaryOp::NotEq => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                },
                expr_code(right, env, document, mode)?
            )
        }
    })
}

fn route_code(
    route: &Route,
    payload: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    if route.args.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok(payload.into()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}

fn initial_code(expr: &Expr, ty: &Type, document: &Document) -> String {
    match (expr, ty) {
        (Expr::Str(value), Type::Str) => format!("{}.to_owned()", rust_string(value)),
        (Expr::EmptyList, Type::List(_)) => "::std::vec::Vec::new()".into(),
        (Expr::None, Type::Option(_)) => "::std::option::Option::None".into(),
        (Expr::Bool(value), _) => value.to_string(),
        (Expr::I64(value), _) => value.to_string(),
        (Expr::F64(value), _) => rust_f64(*value),
        _ => expr_code(expr, &HashMap::new(), document, ValueMode::Owned)
            .unwrap_or_else(|_| "::core::default::Default::default()".into()),
    }
}

fn input_bindings(root: &ViewNode) -> Vec<String> {
    fn collect(node: &ViewNode, output: &mut Vec<String>) {
        match node {
            ViewNode::Input { binding, .. } => {
                if !output.contains(binding) {
                    output.push(binding.clone());
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::MouseArea { content, .. } => collect(content, output),
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(root, &mut output);
    output
}

fn needs_extern_noop(document: &Document) -> bool {
    fn contains(node: &ViewNode) -> bool {
        match node {
            ViewNode::ExternComponent { route: None, .. } => true,
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => children.iter().any(contains),
            ViewNode::Tooltip { content, tip, .. } => contains(content) || contains(tip),
            ViewNode::MouseArea { content, .. } => contains(content),
            _ => false,
        }
    }
    contains(&document.view) || document.components.iter().any(|item| contains(&item.root))
}

fn length_code(
    length: &LengthValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match length {
        LengthValue::Fill => "::iced::Fill".into(),
        LengthValue::Shrink => "::iced::Shrink".into(),
        LengthValue::Fixed(value) => format!(
            "{} as f32",
            expr_code(value, env, document, ValueMode::Owned)?
        ),
    })
}

fn mouse_interaction_code(interaction: MouseInteraction) -> &'static str {
    match interaction {
        MouseInteraction::None => "None",
        MouseInteraction::Hidden => "Hidden",
        MouseInteraction::Idle => "Idle",
        MouseInteraction::ContextMenu => "ContextMenu",
        MouseInteraction::Help => "Help",
        MouseInteraction::Pointer => "Pointer",
        MouseInteraction::Progress => "Progress",
        MouseInteraction::Wait => "Wait",
        MouseInteraction::Cell => "Cell",
        MouseInteraction::Crosshair => "Crosshair",
        MouseInteraction::Text => "Text",
        MouseInteraction::Alias => "Alias",
        MouseInteraction::Copy => "Copy",
        MouseInteraction::Move => "Move",
        MouseInteraction::NoDrop => "NoDrop",
        MouseInteraction::NotAllowed => "NotAllowed",
        MouseInteraction::Grab => "Grab",
        MouseInteraction::Grabbing => "Grabbing",
        MouseInteraction::ResizingHorizontally => "ResizingHorizontally",
        MouseInteraction::ResizingVertically => "ResizingVertically",
        MouseInteraction::ResizingDiagonallyUp => "ResizingDiagonallyUp",
        MouseInteraction::ResizingDiagonallyDown => "ResizingDiagonallyDown",
        MouseInteraction::ResizingColumn => "ResizingColumn",
        MouseInteraction::ResizingRow => "ResizingRow",
        MouseInteraction::AllScroll => "AllScroll",
        MouseInteraction::ZoomIn => "ZoomIn",
        MouseInteraction::ZoomOut => "ZoomOut",
    }
}

fn binding_variant(binding: &str) -> String {
    format!("__Bind{}", pascal(binding))
}

fn id_code(
    id: &Id,
    scope: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    if let Some(key) = &id.key {
        Ok(format!(
            "format!(\"{{}}/{}({{}})\", {scope}, {})",
            id.name,
            expr_code(key, env, document, ValueMode::Borrowed)?
        ))
    } else {
        Ok(format!("format!(\"{{}}/{}\", {scope})", id.name))
    }
}

#[derive(Default)]
struct Style {
    width_fill: bool,
    height_fill: bool,
    max_width: Option<u16>,
    padding: [u16; 4],
    gap: Option<u16>,
    items_center: bool,
    self_center: bool,
    text_size: Option<u16>,
    bold: bool,
    text_color: Option<String>,
    background: Option<String>,
    hover_background: Option<String>,
    pressed_background: Option<String>,
    border_color: Option<String>,
    focus_border_color: Option<String>,
    border_width: u16,
    radius: u16,
    disabled_opacity: Option<f32>,
}

impl Style {
    fn parse(tokens: &[String], document: &Document) -> Self {
        let mut style = Self::default();
        for token in tokens {
            let (variant, utility) = token
                .split_once(':')
                .map_or((None, token.as_str()), |(a, b)| (Some(a), b));
            if variant == Some("hover") && utility.starts_with("bg-") {
                style.hover_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("pressed") && utility.starts_with("bg-") {
                style.pressed_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("focus") && utility.starts_with("border-") {
                style.focus_border_color = Some(utility[7..].into());
                continue;
            }
            if variant == Some("disabled") && utility.starts_with("opacity-") {
                style.disabled_opacity =
                    utility[8..].parse::<f32>().ok().map(|value| value / 100.0);
                continue;
            }
            if variant.is_some() {
                continue;
            }
            match utility {
                "w-full" => style.width_fill = true,
                "h-full" => style.height_fill = true,
                "max-w-sm" => style.max_width = Some(384),
                "max-w-md" => style.max_width = Some(448),
                "max-w-lg" => style.max_width = Some(512),
                "max-w-xl" => style.max_width = Some(576),
                "max-w-2xl" => style.max_width = Some(672),
                "items-center" => style.items_center = true,
                "self-center" => style.self_center = true,
                "text-xs" => style.text_size = Some(12),
                "text-sm" => style.text_size = Some(14),
                "text-base" => style.text_size = Some(16),
                "text-lg" => style.text_size = Some(18),
                "text-xl" => style.text_size = Some(20),
                "text-2xl" => style.text_size = Some(24),
                "font-bold" => style.bold = true,
                "border" => style.border_width = 1,
                "border-2" => style.border_width = 2,
                "rounded-sm" => style.radius = 2,
                "rounded" | "rounded-md" => style.radius = 6,
                "rounded-lg" => style.radius = 10,
                "rounded-full" => style.radius = 999,
                _ if utility.starts_with("gap-") => style.gap = spacing(&utility[4..]),
                _ if utility.starts_with("p-") => {
                    if let Some(value) = spacing(&utility[2..]) {
                        style.padding = [value; 4];
                    }
                }
                _ if utility.starts_with("px-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[1] = value;
                        style.padding[3] = value;
                    }
                }
                _ if utility.starts_with("py-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[0] = value;
                        style.padding[2] = value;
                    }
                }
                _ if utility.starts_with("bg-") => style.background = Some(utility[3..].into()),
                _ if utility.starts_with("text-") && document.theme.contains_key(&utility[5..])
                    || matches!(utility, "text-white" | "text-black") =>
                {
                    style.text_color = Some(utility[5..].into())
                }
                _ if utility.starts_with("border-") => {
                    style.border_color = Some(utility[7..].into())
                }
                _ => {}
            }
        }
        style
    }

    fn padding_code(&self) -> Option<String> {
        (self.padding != [0; 4]).then(|| {
            format!(
                "::iced::Padding {{ top: {}.0, right: {}.0, bottom: {}.0, left: {}.0 }}",
                self.padding[0], self.padding[1], self.padding[2], self.padding[3]
            )
        })
    }
}

fn append_size(code: &mut String, style: &Style) {
    if style.width_fill {
        code.push_str(".width(::iced::Fill)");
    }
    if style.height_fill {
        code.push_str(".height(::iced::Fill)");
    }
}

fn container_style_code(style: &Style, document: &Document) -> String {
    if style.background.is_none() && style.border_width == 0 && style.text_color.is_none() {
        return String::new();
    }
    let background = style
        .background
        .as_ref()
        .map(|color| format!("Some({}.into())", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let text = style
        .text_color
        .as_ref()
        .map(|color| format!("Some({})", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let border = style
        .border_color
        .as_ref()
        .map(|color| theme_color(document, color))
        .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
    format!(
        ".style(|_| ::iced::widget::container::Style {{ background: {background}, text_color: {text}, border: ::iced::Border {{ color: {border}, width: {}.0, radius: {}.0.into() }}, ..::iced::widget::container::Style::default() }})",
        style.border_width, style.radius
    )
}

fn button_style_code(style: &Style, document: &Document) -> String {
    if style.background.is_none()
        && style.hover_background.is_none()
        && style.pressed_background.is_none()
        && style.text_color.is_none()
        && style.radius == 0
        && style.disabled_opacity.is_none()
    {
        return String::new();
    }

    let normal = style
        .background
        .as_ref()
        .map(|color| theme_color(document, color));
    let hover = style
        .hover_background
        .as_ref()
        .map(|color| theme_color(document, color))
        .or_else(|| normal.clone());
    let pressed = style
        .pressed_background
        .as_ref()
        .map(|color| theme_color(document, color))
        .or_else(|| hover.clone())
        .or_else(|| normal.clone());
    let option = |color: Option<String>| {
        color.map_or_else(|| "None".into(), |color| format!("Some({color})"))
    };
    let mut code = format!(
        ".style(|theme, status| {{ let mut style = ::iced::widget::button::primary(theme, status); let background: Option<::iced::Color> = match status {{ ::iced::widget::button::Status::Hovered => {}, ::iced::widget::button::Status::Pressed => {}, ::iced::widget::button::Status::Disabled => {}, _ => {} }}; if let Some(background) = background {{ style.background = Some(::iced::Background::Color(background)); }}",
        option(hover),
        option(pressed),
        option(normal.clone()),
        option(normal),
    );
    if let Some(text) = &style.text_color {
        write!(code, " style.text_color = {};", theme_color(document, text)).unwrap();
    }
    if style.radius > 0 {
        write!(code, " style.border.radius = {}.0.into();", style.radius).unwrap();
    }
    if style.background.is_some() || style.text_color.is_some() || style.disabled_opacity.is_some()
    {
        let disabled = style.disabled_opacity.unwrap_or(0.5);
        write!(code, " if matches!(status, ::iced::widget::button::Status::Disabled) {{ style.text_color.a *= {disabled}; if let Some(::iced::Background::Color(mut color)) = style.background {{ color.a *= {disabled}; style.background = Some(::iced::Background::Color(color)); }} }}").unwrap();
    }
    code.push_str(" style })");
    code
}

fn input_style_code(style: &Style, document: &Document) -> String {
    if style.background.is_none()
        && style.border_width == 0
        && style.radius == 0
        && style.focus_border_color.is_none()
    {
        return String::new();
    }
    let background = style
        .background
        .as_ref()
        .map(|color| theme_color(document, color))
        .unwrap_or_else(|| theme_color(document, "background"));
    let border = style
        .border_color
        .as_ref()
        .map(|color| theme_color(document, color))
        .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
    let focus = style
        .focus_border_color
        .as_ref()
        .map(|color| theme_color(document, color))
        .unwrap_or_else(|| border.clone());
    let foreground = theme_color(document, "foreground");
    let muted = theme_color(document, "muted");
    let primary = theme_color(document, "primary");
    format!(
        ".style(|_, status| ::iced::widget::text_input::Style {{ background: {background}.into(), border: ::iced::Border {{ color: if matches!(status, ::iced::widget::text_input::Status::Focused {{ .. }}) {{ {focus} }} else {{ {border} }}, width: {}.0, radius: {}.0.into() }}, icon: {foreground}, placeholder: {muted}, value: {foreground}, selection: {primary} }})",
        style.border_width, style.radius
    )
}

fn theme_color(document: &Document, token: &str) -> String {
    let (name, opacity) = token
        .split_once('/')
        .map_or((token, None), |(name, opacity)| {
            (name, opacity.parse::<u8>().ok())
        });
    let value = match name {
        "white" => "#ffffff",
        "black" => "#000000",
        "transparent" => "#00000000",
        name => document
            .theme
            .get(name)
            .map(String::as_str)
            .unwrap_or("#000000"),
    };
    color_code(value, opacity)
}

fn color_code(value: &str, opacity: Option<u8>) -> String {
    let hex = value.trim_start_matches('#');
    let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).unwrap_or(0);
    let alpha = opacity
        .map(|value| value as f32 / 100.0)
        .or_else(|| (hex.len() == 8).then(|| byte(6..8) as f32 / 255.0))
        .unwrap_or(1.0);
    format!(
        "::iced::Color::from_rgba8({}, {}, {}, {alpha:.6})",
        byte(0..2),
        byte(2..4),
        byte(4..6)
    )
}

fn spacing(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().map(|value| value * 4)
}

fn rust_string(value: &str) -> String {
    format!("{value:?}")
}

fn rust_f64(value: f64) -> String {
    format!("{value:?}")
}

fn pascal(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::compile;

    #[test]
    fn emits_a_probe_for_every_extern_function() {
        let source = r#"app Demo
extern crate::backend
  Item(id:i64)
  AppError(message:str)
  load(id:i64) -> [Item] ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
on mount
  return if false
  run load(1) -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items)
"#;
        let generated = compile(source, "demo.ice").unwrap();
        assert!(generated.contains("async fn __ui_lang_check_load"));
        assert!(generated.contains("crate::backend::load(arg0).await"));
        assert!(generated.contains("let task = (||"));
    }

    #[test]
    fn lowers_complex_native_controls() {
        let source = r#"app Controls
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  enabled = false
  choice = 0
on amount_changed(next)
  amount = next
on released
on enabled_changed(next)
  enabled = next
on choice_changed(next)
  choice = next
view
  grid columns=2 @gap-2
    toggler "Enabled" checked=enabled -> enabled_changed _
    slider amount min=0.0 max=100.0 step=0.5 vertical release=released -> amount_changed _
    progress amount vertical
    radio "First" value=0 selected=(choice == 0) -> choice_changed _
    rule horizontal thickness=2.0
    space width=4.0 height=8.0
    stack clip=true
      text "base"
      text "overlay"
"#;
        let generated = compile(source, "controls.ice").unwrap();
        assert!(
            generated.contains("::iced::widget::grid(__children).spacing(8).columns(2 as usize)")
        );
        assert!(generated.contains("::iced::widget::vertical_slider"));
        assert!(generated.contains("::iced::widget::progress_bar"));
        assert!(generated.contains(".vertical()"));
        assert!(generated.contains("::iced::widget::radio"));
        assert!(generated.contains("::iced::widget::stack(__children).clip(true)"));
    }

    #[test]
    fn lowers_list_literals_options_and_pick_lists() {
        let source = r#"app Selection
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on opened
on closed
view
  pick choices selected placeholder="Choose" width=fill menu-height=120.0 padding=8.0 text-size=14.0 open=opened close=closed -> selected _
"#;
        let generated = compile(source, "selection.ice").unwrap();
        assert!(
            generated.contains("pub(crate) selected: ::std::option::Option<::std::string::String>")
        );
        assert!(generated.contains("::std::vec![\"List\".to_owned(), \"Board\".to_owned()]"));
        assert!(
            generated
                .contains("::iced::widget::pick_list(self.choices.clone(), self.selected.clone()")
        );
        assert!(generated.contains(".on_open(__SelectionMessage::Opened)"));
        assert!(generated.contains("self.selected = ::std::option::Option::Some(next);"));
    }

    #[test]
    fn lowers_typed_iced_extern_boundaries() {
        let source = r#"app Interop
extern crate::backend
  Failure(code:i64)
  component native_meter(value:f64) -> f64
  component passive() -> unit
  task focus_next() -> unit
  task save() -> i64 ! Failure
  subscription events() -> bool
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 1.0
  count = 0
  seen = false
on changed(next)
  amount = next
on focused
on focus
  task focus_next() -> focused
on save
  task save() -> saved _ | failed _
on saved(next)
  count = next
on failed(error)
  count = error.code
on event(next)
  seen = next
subscribe
  events() -> event _
view
  col
    extern native_meter(amount) -> changed _
    extern passive()
    button "Focus" -> focus
    button "Save" -> save
"#;
        let generated = compile(source, "interop.ice").unwrap();
        assert!(generated.contains("::iced::Element<'static, f64>"));
        assert!(generated.contains("::iced::Task<()>"));
        assert!(generated.contains("::iced::Subscription<bool>"));
        assert!(generated.contains(".subscription(Self::__subscription)"));
        assert!(generated.contains("native_meter(self.amount).map"));
        assert!(generated.contains("passive().map(move |__value| __InteropMessage::__ExternNoop)"));
        assert!(generated.contains("focus_next().map(|value| __InteropMessage::Focused)"));
        assert!(generated.contains("save().map(|result| match result"));
        assert!(generated.contains("Result::Err(error) => __InteropMessage::Failed(error)"));
    }

    #[test]
    fn lowers_media_tooltip_and_pointer_events() {
        let source = r#"app Media
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on entered
on exited
on pressed
view
  col
    image "photo.ppm" width=fill height=64.0 fit=cover filter=nearest rotation=0.5 opacity=0.8 scale=1.2 expand=true radius=4.0
    svg "icon.svg" width=48.0 height=shrink fit=scale-down rotation=0.1 opacity=0.9
    tooltip position=cursor gap=2.0 padding=5.0 delay=100 snap=false
      mouse enter=entered exit=exited press=pressed cursor=pointer
        text "Hover"
      text "Tip"
"#;
        let generated = compile(source, "media.ice").unwrap();
        assert!(generated.contains("::iced::widget::image(\"photo.ppm\".to_owned())"));
        assert!(generated.contains(".filter_method(::iced::widget::image::FilterMethod::Nearest)"));
        assert!(generated.contains("::iced::widget::svg(\"icon.svg\".to_owned())"));
        assert!(generated.contains("tooltip::Position::FollowCursor"));
        assert!(generated.contains(".delay(::std::time::Duration::from_millis(100 as u64))"));
        assert!(generated.contains(".on_enter(__MediaMessage::Entered)"));
        assert!(generated.contains(".interaction(::iced::mouse::Interaction::Pointer)"));
    }
}
