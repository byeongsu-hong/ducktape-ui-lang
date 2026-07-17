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
    generate_keyboard_types(&mut out, document);
    generate_system_types(&mut out, document);

    writeln!(out, "#[derive(Debug)]\npub struct {} {{", document.app).unwrap();
    for qr in &document.qr_codes {
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::qr_code::Data,",
            qr.name
        )
        .unwrap();
    }
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
    for binding in editor_bindings(&document.view) {
        writeln!(
            out,
            "{}(::iced::widget::text_editor::Action),",
            editor_variant(&binding)
        )
        .unwrap();
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
    let default_font = document
        .fonts
        .iter()
        .find(|font| font.default)
        .map_or_else(String::new, |font| {
            format!(".default_font({})", font_decl_code(font))
        });
    let title = document
        .settings
        .title
        .as_ref()
        .map_or_else(String::new, |title| {
            format!(".title({})", rust_string(title))
        });
    let settings = app_settings_code(&document.settings);
    let window = window_settings_code(document.settings.window.as_ref());
    let scale_factor = document
        .settings
        .scale_factor
        .map_or_else(String::new, |scale| {
            format!(".scale_factor(|_| {scale} as f32)")
        });
    writeln!(out, "::iced::application(Self::__boot, Self::__update, Self::__view){title}{subscription}.theme(Self::__theme){settings}{default_font}{window}{scale_factor}.run()").unwrap();
    writeln!(out, "}}").unwrap();

    generate_theme(&mut out, document);
    generate_boot(&mut out, document, &message)?;
    generate_update(&mut out, document, &message)?;
    generate_subscription(&mut out, document, &message)?;
    generate_view(&mut out, document, &message)?;
    writeln!(out, "}}").unwrap();
    Ok(out)
}

fn app_settings_code(settings: &AppSettings) -> String {
    let mut fields = String::new();
    if let Some(id) = &settings.id {
        write!(
            fields,
            "id: ::std::option::Option::Some({}.to_owned()),",
            rust_string(id)
        )
        .unwrap();
    }
    if let Some(size) = settings.default_text_size {
        write!(fields, "default_text_size: ::iced::Pixels({size} as f32),").unwrap();
    }
    if let Some(value) = settings.antialiasing {
        write!(fields, "antialiasing: {value},").unwrap();
    }
    if let Some(value) = settings.vsync {
        write!(fields, "vsync: {value},").unwrap();
    }
    if fields.is_empty() {
        String::new()
    } else {
        format!(".settings(::iced::Settings {{ {fields} ..::std::default::Default::default() }})")
    }
}

fn window_settings_code(settings: Option<&WindowSettings>) -> String {
    let Some(settings) = settings else {
        return String::new();
    };
    let mut fields = String::new();
    let size =
        |(width, height): (f64, f64)| format!("::iced::Size::new({width} as f32, {height} as f32)");
    if let Some(value) = settings.size {
        write!(fields, "size: {},", size(value)).unwrap();
    }
    for (name, value) in [
        ("maximized", settings.maximized),
        ("fullscreen", settings.fullscreen),
        ("visible", settings.visible),
        ("resizable", settings.resizable),
        ("closeable", settings.closeable),
        ("minimizable", settings.minimizable),
        ("decorations", settings.decorations),
        ("transparent", settings.transparent),
        ("blur", settings.blur),
        ("exit_on_close_request", settings.exit_on_close_request),
    ] {
        if let Some(value) = value {
            write!(fields, "{name}: {value},").unwrap();
        }
    }
    if let Some(position) = settings.position {
        let position = match position {
            WindowPosition::Default => "::iced::window::Position::Default".into(),
            WindowPosition::Centered => "::iced::window::Position::Centered".into(),
            WindowPosition::Specific(x, y) => format!(
                "::iced::window::Position::Specific(::iced::Point::new({x} as f32, {y} as f32))"
            ),
        };
        write!(fields, "position: {position},").unwrap();
    }
    if let Some(value) = settings.min_size {
        write!(
            fields,
            "min_size: ::std::option::Option::Some({}),",
            size(value)
        )
        .unwrap();
    }
    if let Some(value) = settings.max_size {
        write!(
            fields,
            "max_size: ::std::option::Option::Some({}),",
            size(value)
        )
        .unwrap();
    }
    if let Some(level) = settings.level {
        let level = match level {
            WindowLevel::Normal => "Normal",
            WindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
            WindowLevel::AlwaysOnTop => "AlwaysOnTop",
        };
        write!(fields, "level: ::iced::window::Level::{level},").unwrap();
    }
    format!(".window(::iced::window::Settings {{ {fields} ..::std::default::Default::default() }})")
}

fn generate_keyboard_types(out: &mut String, document: &Document) {
    if !document
        .subscriptions
        .iter()
        .any(|subscription| matches!(&subscription.source, SubscriptionSource::Keyboard(_)))
    {
        return;
    }
    out.push_str(
        r#"#[derive(Debug, Clone, Copy)]
struct __IceKeyModifiers {
    shift: bool,
    control: bool,
    alt: bool,
    logo: bool,
    command: bool,
    jump: bool,
    macos_command: bool,
}
#[derive(Debug, Clone)]
struct __IceKeyPress {
    key: ::std::string::String,
    modified_key: ::std::string::String,
    physical_key: ::std::string::String,
    location: ::std::string::String,
    modifiers: __IceKeyModifiers,
    text: ::std::option::Option<::std::string::String>,
    repeat: bool,
}
#[derive(Debug, Clone)]
struct __IceKeyRelease {
    key: ::std::string::String,
    modified_key: ::std::string::String,
    physical_key: ::std::string::String,
    location: ::std::string::String,
    modifiers: __IceKeyModifiers,
}
fn __ice_key(value: ::iced::keyboard::Key) -> ::std::string::String {
    match value {
        ::iced::keyboard::Key::Named(value) => ::std::format!("{value:?}"),
        ::iced::keyboard::Key::Character(value) => value.to_string(),
        ::iced::keyboard::Key::Unidentified => "Unidentified".to_owned(),
    }
}
fn __ice_key_modifiers(value: ::iced::keyboard::Modifiers) -> __IceKeyModifiers {
    __IceKeyModifiers {
        shift: value.shift(),
        control: value.control(),
        alt: value.alt(),
        logo: value.logo(),
        command: value.command(),
        jump: value.jump(),
        macos_command: value.macos_command(),
    }
}
fn __ice_key_location(value: ::iced::keyboard::Location) -> ::std::string::String {
    match value {
        ::iced::keyboard::Location::Standard => "standard",
        ::iced::keyboard::Location::Left => "left",
        ::iced::keyboard::Location::Right => "right",
        ::iced::keyboard::Location::Numpad => "numpad",
    }.to_owned()
}
"#,
    );
}

fn generate_system_types(out: &mut String, document: &Document) {
    let information = uses_system_task(document, "__ice_system_info");
    let theme = uses_system_task(document, "__ice_system_theme")
        || document
            .subscriptions
            .iter()
            .any(|subscription| matches!(&subscription.source, SubscriptionSource::SystemTheme));
    if information {
        out.push_str(
            r#"#[derive(Debug, Clone)]
struct __IceSystemInfo {
    system_name: ::std::option::Option<::std::string::String>,
    system_kernel: ::std::option::Option<::std::string::String>,
    system_version: ::std::option::Option<::std::string::String>,
    system_short_version: ::std::option::Option<::std::string::String>,
    cpu_brand: ::std::string::String,
    cpu_cores: ::std::option::Option<i64>,
    memory_total: i64,
    memory_used: ::std::option::Option<i64>,
    graphics_backend: ::std::string::String,
    graphics_adapter: ::std::string::String,
}
fn __ice_system_info(value: ::iced::system::Information) -> __IceSystemInfo {
    __IceSystemInfo {
        system_name: value.system_name,
        system_kernel: value.system_kernel,
        system_version: value.system_version,
        system_short_version: value.system_short_version,
        cpu_brand: value.cpu_brand,
        cpu_cores: value.cpu_cores.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        memory_total: i64::try_from(value.memory_total).unwrap_or(i64::MAX),
        memory_used: value.memory_used.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        graphics_backend: value.graphics_backend,
        graphics_adapter: value.graphics_adapter,
    }
}
"#,
        );
    }
    if theme {
        out.push_str(
            r#"fn __ice_system_theme(value: ::iced::theme::Mode) -> ::std::string::String {
    match value {
        ::iced::theme::Mode::None => "none",
        ::iced::theme::Mode::Light => "light",
        ::iced::theme::Mode::Dark => "dark",
    }.to_owned()
}
"#,
        );
    }
}

fn uses_system_task(document: &Document, name: &str) -> bool {
    document.handlers.iter().any(|handler| {
        handler.statements.iter().any(|statement| {
            matches!(statement, Statement::Run { kind: EffectKind::Task, function, .. } if function == name)
        })
    })
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
    for qr in &document.qr_codes {
        writeln!(out, "{}: {},", qr.name, qr_data_code(qr)).unwrap();
    }
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
    for binding in editor_bindings(&document.view) {
        let variant = editor_variant(&binding);
        writeln!(
            out,
            "{message}::{variant}(action) => {{ self.{binding}.perform(action); ::iced::Task::none() }}"
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
        let route = route_code(&subscription.route, "__value", &env, document, message)?;
        match &subscription.source {
            SubscriptionSource::Extern { function, args } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Subscription)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown extern subscription `{function}`"),
                        )
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                writeln!(
                    out,
                    "{}({args}).map(move |__value| {route}),",
                    source.rust_path
                )
                .unwrap();
            }
            SubscriptionSource::Keyboard(event) => {
                let filter = match event {
                    KeyboardEvent::Press => {
                        "match __event { ::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat } => ::std::option::Option::Some(__IceKeyPress { key: __ice_key(key), modified_key: __ice_key(modified_key), physical_key: ::std::format!(\"{physical_key:?}\"), location: __ice_key_location(location), modifiers: __ice_key_modifiers(modifiers), text: text.map(|value| value.to_string()), repeat }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Release => {
                        "match __event { ::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers } => ::std::option::Option::Some(__IceKeyRelease { key: __ice_key(key), modified_key: __ice_key(modified_key), physical_key: ::std::format!(\"{physical_key:?}\"), location: __ice_key_location(location), modifiers: __ice_key_modifiers(modifiers) }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Modifiers => {
                        "match __event { ::iced::keyboard::Event::ModifiersChanged(modifiers) => ::std::option::Option::Some(__ice_key_modifiers(modifiers)), _ => ::std::option::Option::None }"
                    }
                };
                writeln!(out, "::iced::keyboard::listen().filter_map(|__event| {{ {filter} }}).map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::SystemTheme => {
                writeln!(out, "::iced::system::theme_changes().map(__ice_system_theme).map(move |__value| {route}),").unwrap();
            }
        }
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
        None,
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
                if *kind == EffectKind::Task
                    && matches!(
                        function.as_str(),
                        "__ice_system_info"
                            | "__ice_system_theme"
                            | "__ice_clipboard_read"
                            | "__ice_clipboard_read_primary"
                    )
                {
                    let task = match function.as_str() {
                        "__ice_system_info" => {
                            "::iced::system::information().map(__ice_system_info)"
                        }
                        "__ice_system_theme" => "::iced::system::theme().map(__ice_system_theme)",
                        "__ice_clipboard_read" => "::iced::clipboard::read()",
                        "__ice_clipboard_read_primary" => "::iced::clipboard::read_primary()",
                        _ => unreachable!(),
                    };
                    let success_message = route_code(success, "value", env, document, message)?;
                    writeln!(
                        out,
                        "{}{task}.map(move |value| {success_message}){}",
                        if return_task { "return " } else { "" },
                        if return_task { ";" } else { "" }
                    )
                    .unwrap();
                    continue;
                }
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
            Statement::ClipboardWrite { primary, value, .. } => {
                has_task = true;
                let value = expr_code(value, env, document, ValueMode::Owned)?;
                let function = if *primary { "write_primary" } else { "write" };
                writeln!(
                    out,
                    "{}::iced::clipboard::{function}::<{message}>({value}){}",
                    if return_task { "return " } else { "" },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
            }
            Statement::WidgetOperation {
                operation, route, ..
            } => {
                has_task = true;
                let id = |id: &str| {
                    format!(
                        "::iced::widget::Id::new({})",
                        rust_string(&format!("{}/{}", document.app, id))
                    )
                };
                let value = |value: &Expr, cast: &str| {
                    Ok::<_, Error>(format!(
                        "({}) as {cast}",
                        expr_code(value, env, document, ValueMode::Owned)?
                    ))
                };
                let task = match operation {
                    WidgetOperation::FocusPrevious => {
                        format!("::iced::widget::operation::focus_previous::<{message}>()")
                    }
                    WidgetOperation::FocusNext => {
                        format!("::iced::widget::operation::focus_next::<{message}>()")
                    }
                    WidgetOperation::Focus { id: target } => format!(
                        "::iced::widget::operation::focus::<{message}>({})",
                        id(target)
                    ),
                    WidgetOperation::Focused { id: target } => {
                        let route = route.as_ref().expect("checker requires focused route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::widget::operation::is_focused({}).map(move |value| {message_code})",
                            id(target)
                        )
                    }
                    WidgetOperation::CursorFront { id: target } => format!(
                        "::iced::widget::operation::move_cursor_to_front::<{message}>({})",
                        id(target)
                    ),
                    WidgetOperation::CursorEnd { id: target } => format!(
                        "::iced::widget::operation::move_cursor_to_end::<{message}>({})",
                        id(target)
                    ),
                    WidgetOperation::Cursor {
                        id: target,
                        position,
                    } => format!(
                        "::iced::widget::operation::move_cursor_to::<{message}>({}, {})",
                        id(target),
                        value(position, "usize")?
                    ),
                    WidgetOperation::SelectAll { id: target } => format!(
                        "::iced::widget::operation::select_all::<{message}>({})",
                        id(target)
                    ),
                    WidgetOperation::Select {
                        id: target,
                        start,
                        end,
                    } => format!(
                        "::iced::widget::operation::select_range::<{message}>({}, {}, {})",
                        id(target),
                        value(start, "usize")?,
                        value(end, "usize")?
                    ),
                    WidgetOperation::Snap { id: target, x, y } => format!(
                        "::iced::widget::operation::snap_to::<{message}>({}, ::iced::widget::operation::RelativeOffset {{ x: {}, y: {} }})",
                        id(target),
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WidgetOperation::SnapEnd { id: target } => format!(
                        "::iced::widget::operation::snap_to_end::<{message}>({})",
                        id(target)
                    ),
                    WidgetOperation::ScrollTo { id: target, x, y } => format!(
                        "::iced::widget::operation::scroll_to::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target),
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WidgetOperation::ScrollBy { id: target, x, y } => format!(
                        "::iced::widget::operation::scroll_by::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target),
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                };
                writeln!(
                    out,
                    "{}{task}{}",
                    if return_task { "return " } else { "" },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
            }
            Statement::WindowOperation {
                operation, route, ..
            } => {
                has_task = true;
                let id = "__window";
                let value = |value: &Expr, cast: &str| {
                    Ok::<_, Error>(format!(
                        "({}) as {cast}",
                        expr_code(value, env, document, ValueMode::Owned)?
                    ))
                };
                let size = |width: &Expr, height: &Expr| {
                    Ok::<_, Error>(format!(
                        "::iced::Size::new({}, {})",
                        value(width, "f32")?,
                        value(height, "f32")?
                    ))
                };
                let optional_size = |size_value: &Option<(Expr, Expr)>| {
                    Ok::<_, Error>(match size_value {
                        Some((width, height)) => {
                            format!("::std::option::Option::Some({})", size(width, height)?)
                        }
                        None => "::std::option::Option::None".into(),
                    })
                };
                let bool_value = |value: &Expr| expr_code(value, env, document, ValueMode::Owned);
                let task = match operation {
                    WindowOperation::Close => {
                        format!("::iced::window::close::<{message}>({id})")
                    }
                    WindowOperation::Drag => {
                        format!("::iced::window::drag::<{message}>({id})")
                    }
                    WindowOperation::DragResize(direction) => {
                        let direction = match direction {
                            WindowDirection::North => "North",
                            WindowDirection::South => "South",
                            WindowDirection::East => "East",
                            WindowDirection::West => "West",
                            WindowDirection::NorthEast => "NorthEast",
                            WindowDirection::NorthWest => "NorthWest",
                            WindowDirection::SouthEast => "SouthEast",
                            WindowDirection::SouthWest => "SouthWest",
                        };
                        format!(
                            "::iced::window::drag_resize::<{message}>({id}, ::iced::window::Direction::{direction})"
                        )
                    }
                    WindowOperation::Resize(width, height) => format!(
                        "::iced::window::resize::<{message}>({id}, {})",
                        size(width, height)?
                    ),
                    WindowOperation::Resizable(enabled) => format!(
                        "::iced::window::set_resizable::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::MinSize(size) => format!(
                        "::iced::window::set_min_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::MaxSize(size) => format!(
                        "::iced::window::set_max_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::ResizeIncrements(size) => format!(
                        "::iced::window::set_resize_increments::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::Size => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = ordered_route_code(
                            route,
                            &["value.width as f64", "value.height as f64"],
                            env,
                            document,
                            message,
                        )?;
                        format!("::iced::window::size({id}).map(move |value| {message_code})")
                    }
                    WindowOperation::IsMaximized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::is_maximized({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Maximize(enabled) => format!(
                        "::iced::window::maximize::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::IsMinimized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::is_minimized({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Minimize(enabled) => format!(
                        "::iced::window::minimize::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::Position => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            ordered_route_code(route, &["x", "y"], env, document, message)?;
                        format!(
                            "::iced::window::position({id}).map(move |value| {{ let (x, y) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.x as f64), ::std::option::Option::Some(value.y as f64))); {message_code} }})"
                        )
                    }
                    WindowOperation::ScaleFactor => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            route_code(route, "value as f64", env, document, message)?;
                        format!(
                            "::iced::window::scale_factor({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Move(x, y) => format!(
                        "::iced::window::move_to::<{message}>({id}, ::iced::Point::new({}, {}))",
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WindowOperation::Mode => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::mode({id}).map(move |value| {{ let value = match value {{ ::iced::window::Mode::Windowed => \"windowed\", ::iced::window::Mode::Fullscreen => \"fullscreen\", ::iced::window::Mode::Hidden => \"hidden\" }}.to_owned(); {message_code} }})"
                        )
                    }
                    WindowOperation::SetMode(mode) => {
                        let mode = match mode {
                            WindowMode::Windowed => "Windowed",
                            WindowMode::Fullscreen => "Fullscreen",
                            WindowMode::Hidden => "Hidden",
                        };
                        format!(
                            "::iced::window::set_mode::<{message}>({id}, ::iced::window::Mode::{mode})"
                        )
                    }
                    WindowOperation::ToggleMaximize => {
                        format!("::iced::window::toggle_maximize::<{message}>({id})")
                    }
                    WindowOperation::ToggleDecorations => {
                        format!("::iced::window::toggle_decorations::<{message}>({id})")
                    }
                    WindowOperation::Attention(attention) => {
                        let attention: String = match attention {
                            None => "::std::option::Option::None".into(),
                            Some(WindowAttention::Critical) => "::std::option::Option::Some(::iced::window::UserAttention::Critical)".into(),
                            Some(WindowAttention::Informational) => "::std::option::Option::Some(::iced::window::UserAttention::Informational)".into(),
                        };
                        format!(
                            "::iced::window::request_user_attention::<{message}>({id}, {attention})"
                        )
                    }
                    WindowOperation::Focus => {
                        format!("::iced::window::gain_focus::<{message}>({id})")
                    }
                    WindowOperation::SetLevel(level) => {
                        let level = match level {
                            WindowLevel::Normal => "Normal",
                            WindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
                            WindowLevel::AlwaysOnTop => "AlwaysOnTop",
                        };
                        format!(
                            "::iced::window::set_level::<{message}>({id}, ::iced::window::Level::{level})"
                        )
                    }
                    WindowOperation::SystemMenu => {
                        format!("::iced::window::show_system_menu::<{message}>({id})")
                    }
                    WindowOperation::MousePassthrough(enabled) => {
                        let enabled = bool_value(enabled)?;
                        format!(
                            "if {enabled} {{ ::iced::window::enable_mouse_passthrough::<{message}>({id}) }} else {{ ::iced::window::disable_mouse_passthrough::<{message}>({id}) }}"
                        )
                    }
                    WindowOperation::MonitorSize => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = ordered_route_code(
                            route,
                            &["width", "height"],
                            env,
                            document,
                            message,
                        )?;
                        format!(
                            "::iced::window::monitor_size({id}).map(move |value| {{ let (width, height) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.width as f64), ::std::option::Option::Some(value.height as f64))); {message_code} }})"
                        )
                    }
                    WindowOperation::AutomaticTabbing(enabled) => format!(
                        "::iced::window::allow_automatic_tabbing::<{message}>({})",
                        bool_value(enabled)?
                    ),
                };
                let task = if matches!(operation, WindowOperation::AutomaticTabbing(_)) {
                    task
                } else {
                    format!("::iced::window::oldest().and_then(move |__window| {task})")
                };
                writeln!(
                    out,
                    "{}{task}{}",
                    if return_task { "return " } else { "" },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
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
    slot: Option<&SlotContext>,
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
            *kind, options, id, styles, children, document, message, env, scope, slot,
        ),
        ViewNode::Text {
            value,
            options,
            styles,
            ..
        } => {
            let style = Style::parse(styles, document);
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let mut code = format!("::iced::widget::text({value})");
            append_text_options(&mut code, options, &style, env, document)?;
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
            options,
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
            if let Some(secure) = &options.secure {
                write!(
                    input,
                    ".secure({})",
                    expr_code(secure, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(width) = &options.width {
                write!(input, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    input,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options.text_size {
                write!(
                    input,
                    ".size({} as f32)",
                    expr_code(size, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = &options.line_height {
                write!(
                    input,
                    ".line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(align) = options.align {
                let align = match align {
                    InputAlignment::Left => "Left",
                    InputAlignment::Center => "Center",
                    InputAlignment::Right => "Right",
                };
                write!(input, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
            }
            if let Some(font) = &options.font {
                write!(input, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(icon) = options.icon {
                let size = options.icon_size.as_ref().map_or_else(
                    || Ok("None".to_owned()),
                    |value| {
                        Ok::<_, Error>(format!(
                            "Some(({} as f32).into())",
                            expr_code(value, env, document, ValueMode::Owned)?
                        ))
                    },
                )?;
                let spacing = options.icon_spacing.as_ref().map_or_else(
                    || Ok("0.0".to_owned()),
                    |value| expr_code(value, env, document, ValueMode::Owned),
                )?;
                let side = match options.icon_side.unwrap_or(IconSide::Left) {
                    IconSide::Left => "Left",
                    IconSide::Right => "Right",
                };
                write!(
                    input,
                    ".icon(::iced::widget::text_input::Icon {{ font: ::iced::Font::DEFAULT, code_point: {icon:?}, size: {size}, spacing: {spacing} as f32, side: ::iced::widget::text_input::Side::{side} }})"
                )
                .unwrap();
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
            if let Some(route) = &options.submit {
                let submit = route_code(route, "", env, document, message)?;
                if let Some(disabled) = disabled {
                    write!(
                        input,
                        ".on_submit_maybe(if {} {{ None }} else {{ Some({submit}) }})",
                        expr_code(disabled, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                } else {
                    write!(input, ".on_submit({submit})").unwrap();
                }
            }
            if let Some(route) = &options.paste {
                let paste = route_code(route, "__value", env, document, message)?;
                if let Some(disabled) = disabled {
                    write!(
                        input,
                        ".on_paste_maybe(if {} {{ None }} else {{ Some(move |__value| {paste}) }})",
                        expr_code(disabled, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                } else {
                    write!(input, ".on_paste(move |__value| {paste})").unwrap();
                }
            }
            input.push_str(&input_style_code(&style, document));
            Ok(format!(
                "::iced::widget::column![::iced::widget::text({}), {input}].spacing(6).into()",
                rust_string(label)
            ))
        }
        ViewNode::Button {
            label,
            content,
            id,
            disabled,
            options,
            styles,
            route,
            ..
        } => {
            let style = Style::parse(styles, document);
            let message_code = route_code(route, "", env, document, message)?;
            let content = if let Some(content) = content {
                let child_scope = id.as_ref().map_or_else(
                    || Ok(scope.to_owned()),
                    |id| id_code(id, scope, env, document),
                )?;
                render_node(content, document, message, env, &child_scope, slot)?
            } else {
                format!(
                    "::iced::widget::text({}).into()",
                    rust_string(label.as_ref().expect("button label"))
                )
            };
            let mut code = format!(
                "{{ let __button_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::button(__button_content)"
            );
            if let Some(padding) = style.padding_code() {
                write!(code, ".padding({padding})").unwrap();
            }
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    code,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(clip) = &options.clip {
                write!(
                    code,
                    ".clip({})",
                    expr_code(clip, env, document, ValueMode::Owned)?
                )
                .unwrap();
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
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Checkbox {
            label,
            checked,
            disabled,
            options,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!("::iced::widget::checkbox({checked}).label({label})");
            append_bool_control_options(&mut code, options, env, document, false)?;
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
            options,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!("::iced::widget::toggler({checked}).label({label})");
            append_bool_control_options(&mut code, options, env, document, true)?;
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
            options,
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
            if let Some(default) = &options.default {
                write!(
                    code,
                    ".default({})",
                    expr_code(default, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(shift_step) = &options.shift_step {
                write!(
                    code,
                    ".shift_step({})",
                    expr_code(shift_step, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
                if let Some(length) = length {
                    write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
                }
            }
            append_slider_styles(&mut code, &options.style, env, document)?;
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
            options,
            vertical,
            ..
        } => {
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let min = expr_code(min, env, document, ValueMode::Owned)?;
            let max = expr_code(max, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "::iced::widget::progress_bar(({min} as f32)..=({max} as f32), {value} as f32)"
            );
            if let Some(length) = &options.length {
                write!(code, ".length({})", length_code(length, env, document)?).unwrap();
            }
            if let Some(girth) = &options.girth {
                write!(code, ".girth({})", length_code(girth, env, document)?).unwrap();
            }
            if *vertical {
                code.push_str(".vertical()");
            }
            append_progress_options(&mut code, options, env, document)?;
            Ok(format!("{code}.into()"))
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
        ViewNode::ComboBox {
            state,
            selected,
            placeholder,
            options,
            route,
            span,
        } => {
            let state = env.get(state).ok_or_else(|| {
                Error::new("E150", span, format!("unknown combo state `{state}`"))
            })?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!(
                "{{ let __combo_selection = {selected}; ::iced::widget::combo_box(&{}, {}, __combo_selection.as_ref(), move |__value| {message_code})",
                state.code,
                rust_string(placeholder)
            );
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.menu_height {
                write!(
                    code,
                    ".menu_height({})",
                    length_code(height, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    code,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options.text_size {
                write!(
                    code,
                    ".size({} as f32)",
                    expr_code(size, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(route) = &options.input {
                write!(
                    code,
                    ".on_input(move |__value| {})",
                    route_code(route, "__value", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.hover {
                write!(
                    code,
                    ".on_option_hovered(move |__value| {})",
                    route_code(route, "__value", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.open {
                write!(
                    code,
                    ".on_open({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.close {
                write!(
                    code,
                    ".on_close({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Rule {
            axis,
            thickness,
            options,
            ..
        } => {
            let thickness = expr_code(thickness, env, document, ValueMode::Owned)?;
            let axis = match axis {
                Axis::Horizontal => "horizontal",
                Axis::Vertical => "vertical",
            };
            let mut code = format!("::iced::widget::rule::{axis}({thickness} as f32)");
            append_rule_options(&mut code, options, env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::QrCode {
            data,
            cell_size,
            total_size,
            cell,
            background,
            ..
        } => {
            let mut code = format!("::iced::widget::qr_code(&self.{data})");
            if let Some(value) = cell_size {
                write!(
                    code,
                    ".cell_size({} as f32)",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(value) = total_size {
                write!(
                    code,
                    ".total_size({} as f32)",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if cell.is_some() || background.is_some() {
                let cell = cell.as_deref().map(|value| theme_color(document, value));
                let background = background
                    .as_deref()
                    .map(|value| theme_color(document, value));
                write!(
                    code,
                    ".style(|theme| {{ let default = ::iced::widget::qr_code::default(theme); ::iced::widget::qr_code::Style {{ cell: {}, background: {} }} }})",
                    cell.unwrap_or_else(|| "default.cell".into()),
                    background.unwrap_or_else(|| "default.background".into())
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Space { width, height, .. } => {
            let mut code = String::from("::iced::widget::space()");
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Component {
            name,
            args,
            id,
            content,
            span,
        } => {
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            let mut component_env = HashMap::new();
            for (index, (param, ty)) in component.params.iter().enumerate() {
                let arg = if args.iter().any(|arg| arg.name.is_some()) {
                    args.iter()
                        .find(|arg| arg.name.as_ref() == Some(param))
                        .expect("checker requires every named component prop")
                } else {
                    &args[index]
                };
                component_env.insert(
                    param.clone(),
                    Binding {
                        code: expr_code(&arg.value, env, document, ValueMode::Borrowed)?,
                        ty: ty.clone(),
                        local: false,
                    },
                );
            }
            let component_scope = id.as_ref().map_or_else(
                || format!("format!(\"{{}}/{}\", {scope})", name),
                |id| id_code(id, scope, env, document).unwrap_or_else(|_| scope.into()),
            );
            let component_slot = content.as_deref().map(|content| SlotContext {
                node: content.clone(),
                env: env.clone(),
                parent: slot.cloned().map(Box::new),
            });
            render_node(
                &component.root,
                document,
                message,
                &component_env,
                &component_scope,
                component_slot.as_ref(),
            )
        }
        ViewNode::Slot { span } => {
            let slot = slot.ok_or_else(|| {
                Error::new(
                    "E170",
                    span,
                    "slot reached codegen without component content",
                )
            })?;
            render_node(
                &slot.node,
                document,
                message,
                &slot.env,
                scope,
                slot.parent.as_deref(),
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
            let content = render_node(content, document, message, env, scope, slot)?;
            let tip = render_node(tip, document, message, env, scope, slot)?;
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
            let mut code = format!(
                "{{ let __tooltip_content: ::iced::Element<'_, {message}> = {content}; let __tooltip_tip: ::iced::Element<'_, {message}> = {tip}; ::iced::widget::tooltip(__tooltip_content, __tooltip_tip, ::iced::widget::tooltip::Position::{position}).gap({gap} as f32).padding({padding} as f32).delay(::std::time::Duration::from_millis({delay} as u64)).snap_within_viewport({snap})"
            );
            append_tooltip_style(&mut code, options, env, document)?;
            code.push_str(".into() }");
            Ok(code)
        }
        ViewNode::MouseArea {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
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
            if let Some(route) = &options.move_route {
                write!(
                    code,
                    ".on_move(move |__point| {})",
                    ordered_route_code(
                        route,
                        &["__point.x as f64", "__point.y as f64"],
                        env,
                        document,
                        message,
                    )?
                )
                .unwrap();
            }
            if let Some(route) = &options.scroll {
                let lines = ordered_route_code(
                    route,
                    &["__x as f64", "__y as f64", "false"],
                    env,
                    document,
                    message,
                )?;
                let pixels = ordered_route_code(
                    route,
                    &["__x as f64", "__y as f64", "true"],
                    env,
                    document,
                    message,
                )?;
                write!(
                    code,
                    ".on_scroll(move |__delta| match __delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {lines}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {pixels} }})"
                )
                .unwrap();
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
        ViewNode::Theme {
            preset,
            text,
            background,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let mut code = format!(
                "{{ let __theme_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::themer({}, __theme_content)",
                theme_preset_code(preset)
            );
            if let Some(color) = text {
                write!(code, ".text_color(|_| {})", theme_color(document, color)).unwrap();
            }
            if let Some(color) = background {
                write!(
                    code,
                    ".background(|_| ::iced::Background::Color({}))",
                    theme_color(document, color)
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Float {
            scale,
            x,
            y,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let scale = expr_code(scale, env, document, ValueMode::Owned)?;
            let x = expr_code(x, env, document, ValueMode::Owned)?;
            let y = expr_code(y, env, document, ValueMode::Owned)?;
            Ok(format!(
                "{{ let __float_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::float(__float_content).scale({scale} as f32).translate(move |_, _| ::iced::Vector::new({x} as f32, {y} as f32)).into() }}"
            ))
        }
        ViewNode::Pin {
            width,
            height,
            x,
            y,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let x = expr_code(x, env, document, ValueMode::Owned)?;
            let y = expr_code(y, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __pin_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::pin(__pin_content).x({x} as f32).y({y} as f32)"
            );
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Sensor {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let mut code = format!(
                "{{ let __sensor_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::sensor(__sensor_content)"
            );
            if let Some(route) = &options.show {
                write!(
                    code,
                    ".on_show(move |__size| {})",
                    size_route_code(route, "__size", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.resize {
                write!(
                    code,
                    ".on_resize(move |__size| {})",
                    size_route_code(route, "__size", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.hide {
                write!(
                    code,
                    ".on_hide({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(key) = &options.key {
                write!(
                    code,
                    ".key({})",
                    expr_code(key, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(distance) = &options.anticipate {
                write!(
                    code,
                    ".anticipate({} as f32)",
                    expr_code(distance, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(delay) = &options.delay_ms {
                write!(
                    code,
                    ".delay(::std::time::Duration::from_millis({} as u64))",
                    expr_code(delay, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Responsive {
            content,
            width,
            height,
            ..
        } => {
            let builder = match content {
                ResponsiveContent::Breakpoint {
                    breakpoint,
                    narrow,
                    wide,
                } => {
                    let breakpoint = expr_code(breakpoint, env, document, ValueMode::Owned)?;
                    let narrow = render_node(narrow, document, message, env, scope, slot)?;
                    let wide = render_node(wide, document, message, env, scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: ::iced::Element<'_, {message}> = if __size.width < {breakpoint} as f32 {{ {narrow} }} else {{ {wide} }}; __responsive }}"
                    )
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.insert(
                        width.clone(),
                        Binding {
                            code: "(__size.width as f64)".into(),
                            ty: Type::F64,
                            local: true,
                        },
                    );
                    child_env.insert(
                        height.clone(),
                        Binding {
                            code: "(__size.height as f64)".into(),
                            ty: Type::F64,
                            local: true,
                        },
                    );
                    let content = render_node(content, document, message, &child_env, scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: ::iced::Element<'_, {message}> = {content}; __responsive }}"
                    )
                }
            };
            let mut code = format!("::iced::widget::responsive({builder})");
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::KeyedColumn {
            item,
            items,
            key,
            options,
            child,
            span,
        } => render_keyed_column(
            item, items, key, options, child, span, document, message, env, scope, slot,
        ),
        ViewNode::Lazy {
            dependency,
            binding,
            child,
            span,
        } => {
            let dependency_type = expr_type(
                dependency,
                &env.iter()
                    .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                    .collect(),
                document,
                span,
            )?;
            let dependency = expr_code(dependency, env, document, ValueMode::Owned)?;
            let mut child_env = HashMap::new();
            child_env.insert(
                binding.clone(),
                Binding {
                    code: binding.clone(),
                    ty: dependency_type.clone(),
                    local: false,
                },
            );
            let child = render_node(
                child,
                document,
                message,
                &child_env,
                "__lazy_scope.clone()",
                None,
            )?;
            let dependency_rust = dependency_type.rust(&document.structs);
            Ok(format!(
                "::iced::widget::lazy(({dependency}, ({scope}).to_owned()), move |__dependency| {{ let {binding}: {dependency_rust} = __dependency.0.clone(); let __lazy_scope = __dependency.1.clone(); let __lazy_content: ::iced::Element<'static, {message}> = {child}; __lazy_content }}).into()"
            ))
        }
        ViewNode::Markdown {
            content,
            options,
            route,
            ..
        } => {
            let mut settings = String::from(
                "let mut __markdown_settings = ::iced::widget::markdown::Settings::from(self.__theme());",
            );
            for (value, field) in [
                (&options.text_size, "text_size"),
                (&options.h1_size, "h1_size"),
                (&options.h2_size, "h2_size"),
                (&options.h3_size, "h3_size"),
                (&options.h4_size, "h4_size"),
                (&options.h5_size, "h5_size"),
                (&options.h6_size, "h6_size"),
                (&options.code_size, "code_size"),
                (&options.spacing, "spacing"),
            ] {
                if let Some(value) = value {
                    write!(
                        settings,
                        " __markdown_settings.{field} = ({} as f32).into();",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                }
            }
            let route = route_code(route, "__uri", env, document, message)?;
            Ok(format!(
                "{{ {settings} ::iced::widget::markdown::view(self.{content}.items(), __markdown_settings).map(move |__uri| {route}) }}"
            ))
        }
        ViewNode::TextEditor {
            binding,
            id,
            disabled,
            options,
            ..
        } => {
            let mut code = format!("::iced::widget::text_editor(&self.{binding})");
            if let Some(id) = id {
                write!(
                    code,
                    ".id(::iced::widget::Id::from({}))",
                    id_code(id, scope, env, document)?
                )
                .unwrap();
            }
            if let Some(placeholder) = &options.placeholder {
                write!(code, ".placeholder({})", rust_string(placeholder)).unwrap();
            }
            if let Some(width) = &options.width {
                write!(
                    code,
                    ".width({} as f32)",
                    expr_code(width, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            for (value, method) in [
                (&options.min_height, "min_height"),
                (&options.max_height, "max_height"),
                (&options.size, "size"),
                (&options.padding, "padding"),
            ] {
                if let Some(value) = value {
                    write!(
                        code,
                        ".{method}({} as f32)",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                }
            }
            if let Some(line_height) = &options.line_height {
                match line_height {
                    TextLineHeight::Relative(value) => write!(
                        code,
                        ".line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                    TextLineHeight::Absolute(value) => write!(
                        code,
                        ".line_height(::iced::widget::text::LineHeight::Absolute(({} as f32).into()))",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                }
            }
            if let Some(wrapping) = options.wrapping {
                write!(
                    code,
                    ".wrapping(::iced::widget::text::Wrapping::{})",
                    text_wrapping_code(wrapping)
                )
                .unwrap();
            }
            if let Some(font) = &options.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(syntax) = &options.highlight {
                let theme = match options
                    .highlight_theme
                    .unwrap_or(HighlightTheme::Base16Ocean)
                {
                    HighlightTheme::SolarizedDark => "SolarizedDark",
                    HighlightTheme::Base16Mocha => "Base16Mocha",
                    HighlightTheme::Base16Ocean => "Base16Ocean",
                    HighlightTheme::Base16Eighties => "Base16Eighties",
                    HighlightTheme::InspiredGithub => "InspiredGitHub",
                };
                write!(
                    code,
                    ".highlight({}, ::iced::highlighter::Theme::{theme})",
                    rust_string(syntax)
                )
                .unwrap();
            }
            let variant = editor_variant(binding);
            let enabled = format!(
                "{code}.on_action({message}::{variant} as fn(::iced::widget::text_editor::Action) -> {message})"
            );
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                Ok(format!(
                    "if {disabled} {{ {code}.into() }} else {{ {enabled}.into() }}"
                ))
            } else {
                Ok(format!("{enabled}.into()"))
            }
        }
        ViewNode::Table {
            item,
            rows,
            options,
            columns,
            span,
        } => render_table(
            item, rows, options, columns, span, document, message, env, scope, slot,
        ),
        ViewNode::If { span, .. } | ViewNode::For { span, .. } => Err(Error::new(
            "E170",
            span,
            "if and for must be children of a layout node",
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_table(
    item: &str,
    rows: &Expr,
    options: &TableOptions,
    columns: &[TableColumn],
    span: &Span,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let Type::List(inner) = expr_type(
        rows,
        &env.iter()
            .map(|(name, binding)| (name.clone(), binding.ty.clone()))
            .collect(),
        document,
        span,
    )?
    else {
        unreachable!("checker validates table rows")
    };
    let rows = expr_code(rows, env, document, ValueMode::Owned)?;
    let row_type = *inner;
    let row_rust = row_type.rust(&document.structs);
    let mut cell_env = env.clone();
    cell_env.insert(
        item.into(),
        Binding {
            code: item.into(),
            ty: row_type,
            local: true,
        },
    );
    let mut column_codes = Vec::with_capacity(columns.len());
    for (index, column) in columns.iter().enumerate() {
        let header_scope = format!("format!(\"{{}}/header({index})\", {scope})");
        let cell_scope = format!("format!(\"{{}}/row({{}})/column({index})\", {scope}, __row)");
        let header = render_node(&column.header, document, message, env, &header_scope, slot)?;
        let cell = render_node(
            &column.cell,
            document,
            message,
            &cell_env,
            &cell_scope,
            slot,
        )?;
        let mut code = format!(
            "{{ let __table_header: ::iced::Element<'_, {message}> = {header}; ::iced::widget::table::column(__table_header, move |(__row, {item}): (usize, {row_rust})| -> ::iced::Element<'_, {message}> {{ {cell} }})"
        );
        if let Some(width) = &column.width {
            write!(code, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(align) = column.align_x {
            let align = match align {
                InputAlignment::Left => "Left",
                InputAlignment::Center => "Center",
                InputAlignment::Right => "Right",
            };
            write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
        }
        if let Some(align) = column.align_y {
            let align = match align {
                VerticalAlignment::Top => "Top",
                VerticalAlignment::Center => "Center",
                VerticalAlignment::Bottom => "Bottom",
            };
            write!(code, ".align_y(::iced::alignment::Vertical::{align})").unwrap();
        }
        code.push_str(" }");
        column_codes.push(code);
    }
    let mut code = format!(
        "::iced::widget::table::table(::std::vec![{}], {rows}.into_iter().enumerate())",
        column_codes.join(", ")
    );
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    for (value, method) in [
        (&options.padding, "padding"),
        (&options.padding_x, "padding_x"),
        (&options.padding_y, "padding_y"),
        (&options.separator, "separator"),
        (&options.separator_x, "separator_x"),
        (&options.separator_y, "separator_y"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into()"))
}

#[allow(clippy::too_many_arguments)]
fn render_keyed_column(
    item: &str,
    items: &Expr,
    key: &Expr,
    options: &LayoutOptions,
    child: &ViewNode,
    span: &Span,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let Type::List(inner) = expr_type(
        items,
        &env.iter()
            .map(|(name, binding)| (name.clone(), binding.ty.clone()))
            .collect(),
        document,
        span,
    )?
    else {
        unreachable!("checker validates keyed lists")
    };
    let items = expr_code(items, env, document, ValueMode::Borrowed)?;
    let mut child_env = env.clone();
    child_env.insert(
        item.into(),
        Binding {
            code: item.into(),
            ty: *inner,
            local: false,
        },
    );
    let key = expr_code(key, &child_env, document, ValueMode::Owned)?;
    let child_scope = format!("format!(\"{{}}/key({{:?}})\", {scope}, __key)");
    let child = render_node(child, document, message, &child_env, &child_scope, slot)?;
    let mut code = format!(
        "{{ let mut __children: ::std::vec::Vec<_> = ::std::vec::Vec::new(); for {item} in {items}.iter() {{ let __key = {key}; let __child: ::iced::Element<'_, {message}> = {child}; __children.push((__key, __child)); }} let __layout = ::iced::widget::keyed_column(__children)"
    );
    if let Some(spacing) = &options.spacing {
        write!(
            code,
            ".spacing({} as f32)",
            expr_code(spacing, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.height {
        write!(code, ".height({})", length_code(height, env, document)?).unwrap();
    }
    if let Some(max_width) = &options.max_width {
        write!(
            code,
            ".max_width({} as f32)",
            expr_code(max_width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(align) = options.align {
        let align = match align {
            FlexAlignment::Start => "Start",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "End",
        };
        write!(code, ".align_items(::iced::Alignment::{align})").unwrap();
    }
    Ok(format!("{code}; __layout.into() }}"))
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
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let style = Style::parse(styles, document);
    if kind == Layout::Scroll {
        let child_scope = id.as_ref().map_or_else(
            || Ok(scope.to_owned()),
            |id| id_code(id, scope, env, document),
        )?;
        let child = render_node(&children[0], document, message, env, &child_scope, slot)?;
        let mut code = String::from("::iced::widget::scrollable(__scroll_content)");
        let scroll = options.scroll.as_ref().expect("scroll options");
        let bar = scroll_bar_code(scroll, env, document)?;
        let direction = match scroll.direction {
            ScrollDirection::Vertical => {
                format!("::iced::widget::scrollable::Direction::Vertical({bar})")
            }
            ScrollDirection::Horizontal => {
                format!("::iced::widget::scrollable::Direction::Horizontal({bar})")
            }
            ScrollDirection::Both => format!(
                "::iced::widget::scrollable::Direction::Both {{ vertical: {bar}, horizontal: {bar} }}"
            ),
        };
        write!(code, ".direction({direction})").unwrap();
        if let Some(id) = id {
            write!(
                code,
                ".id(::iced::widget::Id::from({}))",
                id_code(id, scope, env, document)?
            )
            .unwrap();
        }
        let anchor = |anchor| match anchor {
            ScrollAnchor::Start => "Start",
            ScrollAnchor::End => "End",
        };
        write!(
            code,
            ".anchor_x(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_x)
        )
        .unwrap();
        write!(
            code,
            ".anchor_y(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_y)
        )
        .unwrap();
        if let Some(auto_scroll) = &scroll.auto_scroll {
            write!(
                code,
                ".auto_scroll({})",
                expr_code(auto_scroll, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(route) = &scroll.route {
            let message_code = ordered_route_code(
                route,
                &[
                    "__absolute.x as f64",
                    "__absolute.y as f64",
                    "__relative.x as f64",
                    "__relative.y as f64",
                ],
                env,
                document,
                message,
            )?;
            write!(
                code,
                ".on_scroll(move |__viewport| {{ let __absolute = __viewport.absolute_offset(); let __relative = __viewport.relative_offset(); {message_code} }})"
            )
            .unwrap();
        }
        append_size(&mut code, &style);
        if let Some(width) = &scroll.width {
            write!(code, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &scroll.height {
            write!(code, ".height({})", length_code(height, env, document)?).unwrap();
        }
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
    render_children(
        &mut body,
        children,
        document,
        message,
        env,
        &child_scope,
        slot,
    )?;
    let constructor = match kind {
        Layout::Column => "column",
        Layout::Row => "row",
        Layout::Grid => "grid",
        Layout::Stack => "stack",
        Layout::Scroll => unreachable!("scroll returned above"),
    };
    if kind == Layout::Stack && options.under > 0 {
        write!(
            body,
            " let __under = ({} as usize).min(__children.len()); let __above = __children.split_off(__under); let __layout = __above.into_iter().fold(::iced::widget::Stack::new(), |__stack, __child| __stack.push(__child)); let __layout = __children.into_iter().rev().fold(__layout, |__stack, __child| __stack.push_under(__child))",
            options.under
        )
        .unwrap();
    } else {
        write!(
            body,
            " let __layout = ::iced::widget::{constructor}(__children)"
        )
        .unwrap();
    }
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
    if kind == Layout::Grid {
        if let Some(spacing) = &options.spacing {
            write!(
                body,
                ".spacing({} as f32)",
                expr_code(spacing, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(width) = &options.width {
            let LengthValue::Fixed(width) = width else {
                unreachable!("grid widths are always fixed")
            };
            write!(
                body,
                ".width({} as f32)",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(height) = &options.grid_height {
            match height {
                GridSizing::AspectRatio { width, height } => write!(
                    body,
                    ".height(::iced::widget::grid::aspect_ratio({} as f32, {} as f32))",
                    expr_code(width, env, document, ValueMode::Owned)?,
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap(),
                GridSizing::EvenlyDistribute(length) => {
                    write!(body, ".height({})", length_code(length, env, document)?).unwrap();
                }
            }
        }
        if let Some(fluid) = &options.fluid {
            write!(
                body,
                ".fluid({} as f32)",
                expr_code(fluid, env, document, ValueMode::Owned)?
            )
            .unwrap();
        } else if let Some(columns) = &options.columns {
            write!(
                body,
                ".columns({} as usize)",
                expr_code(columns, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if matches!(kind, Layout::Column | Layout::Row) {
        if let Some(spacing) = &options.spacing {
            write!(
                body,
                ".spacing({} as f32)",
                expr_code(spacing, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
            write!(body, ".padding({padding})").unwrap();
        }
        if let Some(width) = &options.width {
            write!(body, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &options.height {
            write!(body, ".height({})", length_code(height, env, document)?).unwrap();
        }
        if let Some(max_width) = &options.max_width {
            write!(
                body,
                ".max_width({} as f32)",
                expr_code(max_width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(align) = options.align {
            let alignment = match (kind, align) {
                (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                (Layout::Column, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                (Layout::Column, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Vertical::Center",
                (Layout::Row, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                _ => unreachable!("only row and column reach flex alignment"),
            };
            let method = if kind == Layout::Column {
                "align_x"
            } else {
                "align_y"
            };
            write!(body, ".{method}({alignment})").unwrap();
        }
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if options.wrap {
            body.push_str(".wrap()");
            if let Some(spacing) = &options.wrap_spacing {
                let method = if kind == Layout::Column {
                    "horizontal_spacing"
                } else {
                    "vertical_spacing"
                };
                write!(
                    body,
                    ".{method}({} as f32)",
                    expr_code(spacing, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(align) = options.wrap_align {
                let alignment = match (kind, align) {
                    (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                    (Layout::Column, FlexAlignment::Center) => {
                        "::iced::alignment::Vertical::Center"
                    }
                    (Layout::Column, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                    (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                    (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                    (Layout::Row, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                    _ => unreachable!("only row and column can wrap"),
                };
                write!(body, ".align_x({alignment})").unwrap();
            }
        }
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
        if let Some(width) = &options.width {
            write!(body, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &options.height {
            write!(body, ".height({})", length_code(height, env, document)?).unwrap();
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

fn scroll_bar_code(
    scroll: &ScrollOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let constructor = if scroll.hidden_bar { "hidden" } else { "new" };
    let mut code = format!("::iced::widget::scrollable::Scrollbar::{constructor}()");
    for (value, method) in [
        (&scroll.bar_width, "width"),
        (&scroll.bar_margin, "margin"),
        (&scroll.scroller_width, "scroller_width"),
        (&scroll.bar_spacing, "spacing"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(code)
}

fn render_children(
    out: &mut String,
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
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
                render_children(out, children, document, message, env, scope, slot)?;
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
                render_children(out, children, document, message, &child_env, scope, slot)?;
                out.push_str(" }");
            }
            _ => {
                let child = render_node(child, document, message, env, scope, slot)?;
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

#[derive(Clone)]
struct SlotContext {
    node: ViewNode,
    env: HashMap<String, Binding>,
    parent: Option<Box<SlotContext>>,
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
            "markdown" => format!(
                "::iced::widget::markdown::Content::parse(&{})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "editor" => format!(
                "::iced::widget::text_editor::Content::with_text(&{})",
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

fn size_route_code(
    route: &Route,
    size: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    ordered_route_code(
        route,
        &[
            &format!("{size}.width as f64"),
            &format!("{size}.height as f64"),
        ],
        env,
        document,
        message,
    )
}

fn ordered_route_code(
    route: &Route,
    payloads: &[&str],
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    let mut payload = payloads.iter();
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok((*payload.next().expect("checked payload count")).to_owned()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}

fn initial_code(expr: &Expr, ty: &Type, document: &Document) -> String {
    match (expr, ty) {
        (Expr::Str(value), Type::Str) => format!("{}.to_owned()", rust_string(value)),
        (Expr::Str(value), Type::Markdown) => format!(
            "::iced::widget::markdown::Content::parse({})",
            rust_string(value)
        ),
        (Expr::Str(value), Type::Editor) => format!(
            "::iced::widget::text_editor::Content::with_text({})",
            rust_string(value)
        ),
        (Expr::EmptyList, Type::List(_)) => "::std::vec::Vec::new()".into(),
        (Expr::EmptyList, Type::Combo(_)) => {
            "::iced::widget::combo_box::State::new(::std::vec::Vec::new())".into()
        }
        (Expr::List(values), Type::Combo(_)) => format!(
            "::iced::widget::combo_box::State::new(::std::vec![{}])",
            values
                .iter()
                .map(|value| {
                    expr_code(value, &HashMap::new(), document, ValueMode::Owned)
                        .unwrap_or_else(|_| "::core::default::Default::default()".into())
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        (Expr::None, Type::Option(_)) => "::std::option::Option::None".into(),
        (Expr::Bool(value), _) => value.to_string(),
        (Expr::I64(value), _) => value.to_string(),
        (Expr::F64(value), _) => rust_f64(*value),
        _ => expr_code(expr, &HashMap::new(), document, ValueMode::Owned)
            .unwrap_or_else(|_| "::core::default::Default::default()".into()),
    }
}

fn state_bindings(root: &ViewNode, editors: bool) -> Vec<String> {
    fn collect(node: &ViewNode, editors: bool, output: &mut Vec<String>) {
        match node {
            ViewNode::Input { binding, .. } if !editors => {
                if !output.contains(binding) {
                    output.push(binding.clone());
                }
            }
            ViewNode::TextEditor { binding, .. } if editors => {
                if !output.contains(binding) {
                    output.push(binding.clone());
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, editors, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, editors, output);
                collect(tip, editors, output);
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, editors, output);
                    collect(&column.cell, editors, output);
                }
            }
            ViewNode::MouseArea { content, .. } | ViewNode::Theme { content, .. } => {
                collect(content, editors, output)
            }
            ViewNode::Component {
                content: Some(content),
                ..
            } => collect(content, editors, output),
            ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => {
                collect(child, editors, output)
            }
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, editors, output),
            ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => collect(content, editors, output),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, editors, output);
                    collect(wide, editors, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, editors, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(root, editors, &mut output);
    output
}

fn input_bindings(root: &ViewNode) -> Vec<String> {
    state_bindings(root, false)
}

fn editor_bindings(root: &ViewNode) -> Vec<String> {
    state_bindings(root, true)
}

fn needs_extern_noop(document: &Document) -> bool {
    fn contains(node: &ViewNode) -> bool {
        match node {
            ViewNode::ExternComponent { route: None, .. } => true,
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => children.iter().any(contains),
            ViewNode::Tooltip { content, tip, .. } => contains(content) || contains(tip),
            ViewNode::Table { columns, .. } => columns
                .iter()
                .any(|column| contains(&column.header) || contains(&column.cell)),
            ViewNode::MouseArea { content, .. } | ViewNode::Theme { content, .. } => {
                contains(content)
            }
            ViewNode::Component {
                content: Some(content),
                ..
            } => contains(content),
            ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => contains(child),
            ViewNode::Button {
                content: Some(content),
                ..
            } => contains(content),
            ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => contains(content),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    contains(narrow) || contains(wide)
                }
                ResponsiveContent::Size { content, .. } => contains(content),
            },
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
        LengthValue::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        LengthValue::Shrink => "::iced::Shrink".into(),
        LengthValue::Fixed(value) => format!(
            "{} as f32",
            expr_code(value, env, document, ValueMode::Owned)?
        ),
    })
}

fn typed_padding_code(
    padding: &PaddingOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let code = |value: Option<&Expr>| {
        value
            .map(|value| expr_code(value, env, document, ValueMode::Owned))
            .transpose()
    };
    let all = code(padding.all.as_ref())?.unwrap_or_else(|| "0.0".into());
    let x = code(padding.x.as_ref())?.unwrap_or_else(|| all.clone());
    let y = code(padding.y.as_ref())?.unwrap_or_else(|| all.clone());
    let top = code(padding.top.as_ref())?.unwrap_or_else(|| y.clone());
    let right = code(padding.right.as_ref())?.unwrap_or_else(|| x.clone());
    let bottom = code(padding.bottom.as_ref())?.unwrap_or(y);
    let left = code(padding.left.as_ref())?.unwrap_or(x);
    Ok(Some(format!(
        "::iced::Padding {{ top: {top} as f32, right: {right} as f32, bottom: {bottom} as f32, left: {left} as f32 }}"
    )))
}

fn radius_code(
    uniform: Option<&Expr>,
    corners: [Option<&Expr>; 4],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if uniform.is_none() && corners.iter().all(Option::is_none) {
        return Ok(None);
    }
    let base = uniform
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "0.0".to_owned());
    let mut values = Vec::with_capacity(4);
    for corner in corners {
        values.push(
            corner
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| base.clone()),
        );
    }
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {} as f32, top_right: {} as f32, bottom_right: {} as f32, bottom_left: {} as f32 }}",
        values[0], values[1], values[2], values[3]
    )))
}

fn append_slider_styles(
    code: &mut String,
    styles: &SliderStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if styles.active.is_none() && styles.hovered.is_none() && styles.dragged.is_none() {
        return Ok(());
    }
    let complete = styles.active.is_some() && styles.hovered.is_some() && styles.dragged.is_some();
    code.push_str(".style(move |__theme, __status| { let mut __style = ::iced::widget::slider::default(__theme, __status); match __status {");
    for (status, style) in [
        ("Active", &styles.active),
        ("Hovered", &styles.hovered),
        ("Dragged", &styles.dragged),
    ] {
        if let Some(style) = style {
            write!(code, " ::iced::widget::slider::Status::{status} => {{").unwrap();
            append_slider_style_fields(code, style, env, document)?;
            code.push_str(" }");
        }
    }
    if !complete {
        code.push_str(" _ => {}");
    }
    code.push_str(" } __style })");
    Ok(())
}

fn append_slider_style_fields(
    code: &mut String,
    style: &SliderStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    for (color, field) in [
        (&style.rail_start, "__style.rail.backgrounds.0"),
        (&style.rail_end, "__style.rail.backgrounds.1"),
        (&style.rail_border_color, "__style.rail.border.color"),
        (&style.handle_color, "__style.handle.background"),
        (&style.handle_border_color, "__style.handle.border_color"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {}.into();", theme_color(document, color)).unwrap();
        }
    }
    for (value, field) in [
        (&style.rail_width, "__style.rail.width"),
        (&style.rail_border_width, "__style.rail.border.width"),
        (&style.handle_border_width, "__style.handle.border_width"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                " {field} = {} as f32;",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(radius) = radius_code(
        style.rail_radius.as_ref(),
        [
            style.rail_radius_top_left.as_ref(),
            style.rail_radius_top_right.as_ref(),
            style.rail_radius_bottom_right.as_ref(),
            style.rail_radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.rail.border.radius = {radius};").unwrap();
    }
    if let Some(shape) = &style.handle_shape {
        let shape = match shape {
            SliderHandleShape::Circle(radius) => format!(
                "::iced::widget::slider::HandleShape::Circle {{ radius: {} as f32 }}",
                expr_code(radius, env, document, ValueMode::Owned)?
            ),
            SliderHandleShape::Rectangle { width } => {
                let radius = radius_code(
                    style.handle_radius.as_ref(),
                    [
                        style.handle_radius_top_left.as_ref(),
                        style.handle_radius_top_right.as_ref(),
                        style.handle_radius_bottom_right.as_ref(),
                        style.handle_radius_bottom_left.as_ref(),
                    ],
                    env,
                    document,
                )?
                .unwrap_or_else(|| "::iced::border::Radius::default()".to_owned());
                format!(
                    "::iced::widget::slider::HandleShape::Rectangle {{ width: {width}, border_radius: {radius} }}"
                )
            }
        };
        write!(code, " __style.handle.shape = {shape};").unwrap();
    }
    Ok(())
}

fn append_tooltip_style(
    code: &mut String,
    options: &TooltipOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.background.is_none()
        && options.text_color.is_none()
        && options.border_color.is_none()
        && options.border_width.is_none()
        && !has_radius
        && options.shadow_color.is_none()
        && options.shadow_x.is_none()
        && options.shadow_y.is_none()
        && options.shadow_blur.is_none()
        && options.pixel_snap.is_none()
    {
        return Ok(());
    }
    let preset = match options.style.unwrap_or(TooltipStyle::Transparent) {
        TooltipStyle::Transparent => "transparent",
        TooltipStyle::Rounded => "rounded_box",
        TooltipStyle::Bordered => "bordered_box",
        TooltipStyle::Dark => "dark",
        TooltipStyle::Primary => "primary",
        TooltipStyle::Secondary => "secondary",
        TooltipStyle::Success => "success",
        TooltipStyle::Warning => "warning",
        TooltipStyle::Danger => "danger",
    };
    write!(
        code,
        ".style(move |__theme| {{ let mut __style = ::iced::widget::container::{preset}(__theme);"
    )
    .unwrap();
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = Some({}.into());",
            theme_color(document, background)
        )
        .unwrap();
    }
    if let Some(text) = &options.text_color {
        write!(
            code,
            " __style.text_color = Some({});",
            theme_color(document, text)
        )
        .unwrap();
    }
    if let Some(border) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("tooltip radius options were present");
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(shadow) = &options.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, shadow)
        )
        .unwrap();
    }
    for (value, field) in [
        (&options.shadow_x, "__style.shadow.offset.x"),
        (&options.shadow_y, "__style.shadow.offset.y"),
        (&options.shadow_blur, "__style.shadow.blur_radius"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                " {field} = {} as f32;",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(pixel_snap) = &options.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(pixel_snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn append_progress_options(
    code: &mut String,
    options: &ProgressOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.background.is_none()
        && options.bar.is_none()
        && options.border_color.is_none()
        && options.border_width.is_none()
        && !has_radius
    {
        return Ok(());
    }
    let preset = match options.style.unwrap_or(ProgressStyle::Primary) {
        ProgressStyle::Primary => "primary",
        ProgressStyle::Secondary => "secondary",
        ProgressStyle::Success => "success",
        ProgressStyle::Warning => "warning",
        ProgressStyle::Danger => "danger",
    };
    write!(
        code,
        ".style(move |__theme| {{ let mut __style = ::iced::widget::progress_bar::{preset}(__theme);"
    )
    .unwrap();
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = {}.into();",
            theme_color(document, background)
        )
        .unwrap();
    }
    if let Some(bar) = &options.bar {
        write!(
            code,
            " __style.bar = {}.into();",
            theme_color(document, bar)
        )
        .unwrap();
    }
    if let Some(border) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("progress radius options were present");
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn append_rule_options(
    code: &mut String,
    options: &RuleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.fill.is_none()
        && options.color.is_none()
        && !has_radius
        && options.snap.is_none()
    {
        return Ok(());
    }
    let preset = match options.style.unwrap_or(RuleStyle::Default) {
        RuleStyle::Default => "default",
        RuleStyle::Weak => "weak",
    };
    write!(
        code,
        ".style(move |__theme| {{ let mut __style = ::iced::widget::rule::{preset}(__theme);"
    )
    .unwrap();
    if let Some(fill) = &options.fill {
        let fill = match fill {
            RuleFill::Full => "::iced::widget::rule::FillMode::Full".to_owned(),
            RuleFill::Percent(value) => format!(
                "::iced::widget::rule::FillMode::Percent({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            RuleFill::Padded(value) => {
                format!("::iced::widget::rule::FillMode::Padded({value})")
            }
            RuleFill::AsymmetricPadding(first, second) => {
                format!("::iced::widget::rule::FillMode::AsymmetricPadding({first}, {second})")
            }
        };
        write!(code, " __style.fill_mode = {fill};").unwrap();
    }
    if let Some(color) = &options.color {
        write!(code, " __style.color = {};", theme_color(document, color)).unwrap();
    }
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("rule radius options were present");
        write!(code, " __style.radius = {radius};").unwrap();
    }
    if let Some(snap) = &options.snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn append_text_options(
    code: &mut String,
    options: &TextOptions,
    style: &Style,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(size) = &options.size {
        write!(
            code,
            ".size({} as f32)",
            expr_code(size, env, document, ValueMode::Owned)?
        )
        .unwrap();
    } else if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    if let Some(line_height) = &options.line_height {
        let line_height = match line_height {
            TextLineHeight::Relative(value) => format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            TextLineHeight::Absolute(value) => format!(
                "::iced::widget::text::LineHeight::Absolute(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
        };
        write!(code, ".line_height({line_height})").unwrap();
    }
    if let Some(alignment) = options.align_x {
        write!(
            code,
            ".align_x(::iced::widget::text::Alignment::{})",
            text_alignment_code(alignment)
        )
        .unwrap();
    }
    if let Some(alignment) = options.align_y {
        let alignment = match alignment {
            VerticalAlignment::Top => "Top",
            VerticalAlignment::Center => "Center",
            VerticalAlignment::Bottom => "Bottom",
        };
        write!(code, ".align_y(::iced::alignment::Vertical::{alignment})").unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".shaping(::iced::widget::text::Shaping::{})",
            text_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".wrapping(::iced::widget::text::Wrapping::{})",
            text_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = &options.font {
        let font = font_preset_code(font, document)?;
        if style.bold {
            write!(
                code,
                ".font(::iced::Font {{ weight: ::iced::font::Weight::Bold, ..{font} }})"
            )
            .unwrap();
        } else {
            write!(code, ".font({font})").unwrap();
        }
    } else if style.bold {
        code.push_str(
            ".font(::iced::Font { weight: ::iced::font::Weight::Bold, ..::iced::Font::DEFAULT })",
        );
    }
    Ok(())
}

fn append_bool_control_options(
    code: &mut String,
    options: &BoolControlOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
    toggler: bool,
) -> Result<(), Error> {
    for (value, method) in [
        (&options.size, "size"),
        (&options.spacing, "spacing"),
        (&options.text_size, "text_size"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.line_height {
        write!(
            code,
            ".text_line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
            expr_code(height, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".text_shaping(::iced::widget::text::Shaping::{})",
            text_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".text_wrapping(::iced::widget::text::Wrapping::{})",
            text_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = &options.font {
        write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
    }
    if toggler {
        if let Some(alignment) = options.alignment {
            write!(
                code,
                ".text_alignment(::iced::widget::text::Alignment::{})",
                text_alignment_code(alignment)
            )
            .unwrap();
        }
    } else if let Some(icon) = options.icon {
        let size = options.icon_size.as_ref().map_or_else(
            || Ok("None".to_owned()),
            |value| {
                Ok::<_, Error>(format!(
                    "Some(({} as f32).into())",
                    expr_code(value, env, document, ValueMode::Owned)?
                ))
            },
        )?;
        let line_height = if let Some(value) = &options.icon_line_height {
            format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
        } else {
            "::iced::widget::text::LineHeight::default()".to_owned()
        };
        let shaping = options.icon_shaping.map_or("Auto", text_shaping_code);
        write!(
            code,
            ".icon(::iced::widget::checkbox::Icon {{ font: ::iced::Font::DEFAULT, code_point: {icon:?}, size: {size}, line_height: {line_height}, shaping: ::iced::widget::text::Shaping::{shaping} }})"
        )
        .unwrap();
    }
    Ok(())
}

fn text_shaping_code(shaping: TextShaping) -> &'static str {
    match shaping {
        TextShaping::Auto => "Auto",
        TextShaping::Basic => "Basic",
        TextShaping::Advanced => "Advanced",
    }
}

fn text_wrapping_code(wrapping: TextWrapping) -> &'static str {
    match wrapping {
        TextWrapping::None => "None",
        TextWrapping::Word => "Word",
        TextWrapping::Glyph => "Glyph",
        TextWrapping::WordOrGlyph => "WordOrGlyph",
    }
}

fn font_preset_code(font: &FontPreset, document: &Document) -> Result<String, Error> {
    match font {
        FontPreset::Default => Ok("::iced::Font::DEFAULT".into()),
        FontPreset::Monospace => Ok("::iced::Font::MONOSPACE".into()),
        FontPreset::Named(name) => document
            .fonts
            .iter()
            .find(|font| font.name == *name)
            .map(font_decl_code)
            .ok_or_else(|| Error::new("E171", &Span::line(1), format!("unknown font `{name}`"))),
    }
}

fn font_decl_code(font: &FontDecl) -> String {
    let family = match &font.family {
        FontFamily::Named(name) => format!("::iced::font::Family::Name({})", rust_string(name)),
        FontFamily::Serif => "::iced::font::Family::Serif".into(),
        FontFamily::SansSerif => "::iced::font::Family::SansSerif".into(),
        FontFamily::Cursive => "::iced::font::Family::Cursive".into(),
        FontFamily::Fantasy => "::iced::font::Family::Fantasy".into(),
        FontFamily::Monospace => "::iced::font::Family::Monospace".into(),
    };
    let weight = match font.weight {
        FontWeight::Thin => "Thin",
        FontWeight::ExtraLight => "ExtraLight",
        FontWeight::Light => "Light",
        FontWeight::Normal => "Normal",
        FontWeight::Medium => "Medium",
        FontWeight::Semibold => "Semibold",
        FontWeight::Bold => "Bold",
        FontWeight::ExtraBold => "ExtraBold",
        FontWeight::Black => "Black",
    };
    let stretch = match font.stretch {
        FontStretch::UltraCondensed => "UltraCondensed",
        FontStretch::ExtraCondensed => "ExtraCondensed",
        FontStretch::Condensed => "Condensed",
        FontStretch::SemiCondensed => "SemiCondensed",
        FontStretch::Normal => "Normal",
        FontStretch::SemiExpanded => "SemiExpanded",
        FontStretch::Expanded => "Expanded",
        FontStretch::ExtraExpanded => "ExtraExpanded",
        FontStretch::UltraExpanded => "UltraExpanded",
    };
    let style = match font.style {
        FontStyle::Normal => "Normal",
        FontStyle::Italic => "Italic",
        FontStyle::Oblique => "Oblique",
    };
    format!(
        "::iced::Font {{ family: {family}, weight: ::iced::font::Weight::{weight}, stretch: ::iced::font::Stretch::{stretch}, style: ::iced::font::Style::{style} }}"
    )
}

fn text_alignment_code(alignment: TextAlignment) -> &'static str {
    match alignment {
        TextAlignment::Default => "Default",
        TextAlignment::Left => "Left",
        TextAlignment::Center => "Center",
        TextAlignment::Right => "Right",
        TextAlignment::Justified => "Justified",
    }
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

fn editor_variant(binding: &str) -> String {
    format!("__Edit{}", pascal(binding))
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

fn theme_preset_code(preset: &ThemePreset) -> String {
    match preset {
        ThemePreset::Default => "::std::option::Option::None".into(),
        ThemePreset::App => "::std::option::Option::Some(Self::__theme(self))".into(),
        ThemePreset::BuiltIn(name) => format!(
            "::std::option::Option::Some(::iced::Theme::{})",
            pascal(name)
        ),
    }
}

fn qr_data_code(qr: &QrData) -> String {
    let module = "::iced::widget::qr_code";
    let data = match &qr.data {
        QrPayload::Text(value) => rust_string(value),
        QrPayload::Bytes(values) => format!(
            "&[{}][..]",
            values
                .iter()
                .map(|value| format!("0x{value:02x}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let correction = |value| match value {
        QrCorrection::Low => format!("{module}::ErrorCorrection::Low"),
        QrCorrection::Medium => format!("{module}::ErrorCorrection::Medium"),
        QrCorrection::Quartile => format!("{module}::ErrorCorrection::Quartile"),
        QrCorrection::High => format!("{module}::ErrorCorrection::High"),
    };
    let constructor = if let Some(version) = qr.version {
        let version = match version {
            QrVersion::Normal(value) => format!("{module}::Version::Normal({value})"),
            QrVersion::Micro(value) => format!("{module}::Version::Micro({value})"),
        };
        let correction = correction(qr.correction.unwrap_or(QrCorrection::Medium));
        format!("{module}::Data::with_version({data}, {version}, {correction})")
    } else if let Some(value) = qr.correction {
        format!(
            "{module}::Data::with_error_correction({data}, {})",
            correction(value)
        )
    } else {
        format!("{module}::Data::new({data})")
    };
    format!("{constructor}.expect(\"invalid qr data `{}`\")", qr.name)
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
        .split(['_', '-'])
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
    fn lowers_complete_common_application_and_window_settings() {
        let source = r#"app Configured
  title "Configured app"
  id "dev.example.configured"
  default-text-size 15
  antialiasing false
  vsync false
  scale-factor 1.25
  window
    size 960 720
    maximized true
    fullscreen false
    position 10 -20
    min-size 480 360
    max-size 1920 1080
    visible true
    resizable false
    closeable false
    minimizable false
    decorations false
    transparent true
    blur true
    level always-on-top
    exit-on-close-request false
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  text "Configured"
"#;
        let generated = compile(source, "configured.ice").unwrap();
        for expected in [
            ".title(\"Configured app\")",
            "id: ::std::option::Option::Some(\"dev.example.configured\".to_owned())",
            "default_text_size: ::iced::Pixels(15 as f32)",
            "antialiasing: false",
            "vsync: false",
            "size: ::iced::Size::new(960 as f32, 720 as f32)",
            "maximized: true",
            "fullscreen: false",
            "Position::Specific(::iced::Point::new(10 as f32, -20 as f32))",
            "min_size: ::std::option::Option::Some(::iced::Size::new(480 as f32, 360 as f32))",
            "max_size: ::std::option::Option::Some(::iced::Size::new(1920 as f32, 1080 as f32))",
            "visible: true",
            "resizable: false",
            "closeable: false",
            "minimizable: false",
            "decorations: false",
            "transparent: true",
            "blur: true",
            "level: ::iced::window::Level::AlwaysOnTop",
            "exit_on_close_request: false",
            ".scale_factor(|_| 1.25 as f32)",
        ] {
            assert!(generated.contains(expected), "missing {expected}");
        }
    }

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
    fn lowers_qr_data_and_widget_options() {
        let source = r#"app Codes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
qr automatic "one"
qr corrected "two" correction=quartile
qr fixed "three" correction=low version=micro(4)
qr binary bytes(00 ff a4)
view
  col
    qr automatic cell-size=5.0
    qr corrected total-size=120.0 cell=primary background=white
    qr fixed
    qr binary
"#;
        let generated = compile(source, "codes.ice").unwrap();
        assert!(generated.contains("qr_code::Data::new(\"one\")"));
        assert!(generated.contains("qr_code::Data::with_error_correction(\"two\", ::iced::widget::qr_code::ErrorCorrection::Quartile)"));
        assert!(generated.contains("qr_code::Data::with_version(\"three\", ::iced::widget::qr_code::Version::Micro(4), ::iced::widget::qr_code::ErrorCorrection::Low)"));
        assert!(generated.contains("qr_code::Data::new(&[0x00u8, 0xffu8, 0xa4u8][..])"));
        assert!(
            generated.contains("::iced::widget::qr_code(&self.automatic).cell_size(5.0 as f32)")
        );
        assert!(generated.contains(
            "::iced::widget::qr_code(&self.corrected).total_size(120.0 as f32).style(|theme|"
        ));
        assert!(generated.contains("qr_code::Style { cell: ::iced::Color"));
    }

    #[test]
    fn lowers_nested_iced_themes() {
        let source = r#"app Themes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
  surface #111111
view
  col
    theme app
      text "App theme"
    theme tokyo-night text=foreground background=surface
      text "Built-in theme"
    theme
      text "Default mode"
"#;
        let generated = compile(source, "themes.ice").unwrap();
        assert!(generated.contains("themer(::std::option::Option::Some(Self::__theme(self))"));
        assert!(
            generated.contains("themer(::std::option::Option::Some(::iced::Theme::TokyoNight)")
        );
        assert!(generated.contains(".text_color(|_| ::iced::Color"));
        assert!(generated.contains(".background(|_| ::iced::Background::Color"));
        assert!(generated.contains("themer(::std::option::Option::None"));
    }

    #[test]
    fn lowers_component_children_and_slot_forwarding() {
        let source = r#"app Composition
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
component Card(title:str)
  col #card
    text title
    slot
component Wrapper(title:str)
  Card title=title
    slot
view
  Wrapper title="Editor" #editor
    input "Name" #name <-> draft
"#;
        let generated = compile(source, "composition.ice").unwrap();
        assert!(generated.contains("__BindDraft(::std::string::String)"));
        assert!(generated.contains("::iced::widget::text_input(\"\", &self.draft)"));
        assert!(generated.contains(
            "format!(\"{}/name\", format!(\"{}/card\", format!(\"{}/Card\", format!(\"{}/editor\", \"Composition\"))))"
        ));
    }

    #[test]
    fn lowers_fully_configured_keyed_columns() {
        let source = r#"app Keyed
extern crate::backend
  Item(id:i64, name:str)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
view
  keyed item in items by=item.id width=fill(2) height=120.0 spacing=8.0 padding=4.0 padding-left=12.0 max-width=640.0 align=end
    scroll #row
      text item.name
"#;
        let generated = compile(source, "keyed.ice").unwrap();
        assert!(generated.contains("for item in self.items.iter()"));
        assert!(generated.contains("__children.push((__key, __child))"));
        assert!(generated.contains("::iced::widget::keyed_column(__children)"));
        assert!(generated.contains(".spacing(8.0 as f32)"));
        assert!(generated.contains("left: 12.0 as f32"));
        assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));
        assert!(generated.contains(".height(120.0 as f32)"));
        assert!(generated.contains(".max_width(640.0 as f32)"));
        assert!(generated.contains(".align_items(::iced::Alignment::End)"));
        assert!(generated.contains("format!(\"{}/key({:?})\""));
    }

    #[test]
    fn lowers_lazy_to_an_owned_static_subtree() {
        let source = r#"app LazyDemo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  title = "Hello"
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
        let generated = compile(source, "lazy.ice").unwrap();
        assert!(
            generated
                .contains("::iced::widget::lazy((self.title.clone(), (\"LazyDemo\").to_owned())")
        );
        assert!(generated.contains("let cached: ::std::string::String = __dependency.0.clone()"));
        assert!(generated.contains("let __lazy_content: ::iced::Element<'static,"));
        assert!(generated.contains("let __lazy_scope = __dependency.1.clone()"));
    }

    #[test]
    fn lowers_parsed_markdown_with_complete_sizes_and_link_route() {
        let source = r##"app Docs
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  docs:markdown = "# Hello"
on open(url)
on reset
  docs = markdown("# Reset")
view
  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 spacing=12.0 -> open _
"##;
        let generated = compile(source, "docs.ice").unwrap();
        assert!(generated.contains("docs: ::iced::widget::markdown::Content::parse(\"# Hello\")"));
        assert!(generated.contains(
            "self.docs = ::iced::widget::markdown::Content::parse(&\"# Reset\".to_owned())"
        ));
        for field in [
            "text_size",
            "h1_size",
            "h2_size",
            "h3_size",
            "h4_size",
            "h5_size",
            "h6_size",
            "code_size",
            "spacing",
        ] {
            assert!(generated.contains(&format!("__markdown_settings.{field} =")));
        }
        assert!(generated.contains("::iced::widget::markdown::view(self.docs.items()"));
        assert!(generated.contains("map(move |__uri| __DocsMessage::Open(__uri))"));
    }

    #[test]
    fn lowers_structured_tables_with_complete_native_options() {
        let source = r#"app Rows
extern crate::backend
  Item(name:str, done:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  rows:[Item] = []
view
  table row in rows width=fill padding=4.0 padding-x=8.0 padding-y=6.0 separator=1.0 separator-x=2.0 separator-y=3.0
    column width=fill(2) align-x=right align-y=bottom
      header
        text "Name"
      cell
        scroll #value
          text row.name
"#;
        let generated = compile(source, "rows.ice").unwrap();
        assert!(generated.contains("table::table(::std::vec!["));
        assert!(generated.contains("self.rows.clone().into_iter().enumerate()"));
        assert!(generated.contains("move |(__row, row): (usize, crate::backend::Item)|"));
        assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));
        assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Right)"));
        assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
        for method in [
            "padding(4.0 as f32)",
            "padding_x(8.0 as f32)",
            "padding_y(6.0 as f32)",
            "separator(1.0 as f32)",
            "separator_x(2.0 as f32)",
            "separator_y(3.0 as f32)",
        ] {
            assert!(generated.contains(method));
        }
        assert!(generated.contains("format!(\"{}/row({})/column(0)\""));
    }

    #[test]
    fn lowers_bound_text_editors_and_internal_actions() {
        let source = r#"app Notes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = "fn main() {}"
  locked = false
view
  editor #body <-> body placeholder="Write" width=640.0 height=fill min-height=80.0 max-height=240.0 size=14.0 line-height-px=18.0 padding=8.0 wrapping=word-or-glyph font=mono highlight="rs" highlight-theme=inspired-github disabled=locked
"#;
        let generated = compile(source, "notes.ice").unwrap();
        assert!(generated.contains("body: ::iced::widget::text_editor::Content::with_text"));
        assert!(generated.contains("__EditBody(::iced::widget::text_editor::Action)"));
        assert!(generated.contains("self.body.perform(action)"));
        assert!(generated.contains("::iced::widget::text_editor(&self.body)"));
        assert!(generated.contains(".width(640.0 as f32)"));
        assert!(generated.contains(".height(::iced::Fill)"));
        assert!(generated.contains(".min_height(80.0 as f32)"));
        assert!(generated.contains(".max_height(240.0 as f32)"));
        assert!(generated.contains("LineHeight::Absolute((18.0 as f32).into())"));
        assert!(generated.contains("Wrapping::WordOrGlyph"));
        assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
        assert!(
            generated.contains(".highlight(\"rs\", ::iced::highlighter::Theme::InspiredGitHub)")
        );
        assert!(generated.contains("if self.locked"));
        assert!(generated.contains(".on_action(__NotesMessage::__EditBody"));
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
  col
    grid columns=2 width=640.0 spacing=12.0 height=aspect(16.0,9.0) @gap-2
      toggler "Enabled" checked=enabled -> enabled_changed _
      slider amount min=0.0 max=100.0 step=0.5 default=50.0 shift-step=0.1 vertical width=20.0 height=fill(2) release=released -> amount_changed _
        active rail-start=primary rail-end=background rail-width=4.0 rail-border=transparent rail-border-width=1.0 rail-radius=2.0 rail-radius-tl=1.0 handle=circle(7.0) handle-color=primary handle-border=foreground handle-border-width=1.0
        hovered rail-start=foreground rail-end=background handle=rect(12) handle-color=foreground handle-radius=3.0 handle-radius-tl=1.0
        dragged rail-start=danger handle=circle(8.0) handle-color=danger
      slider amount min=0.0 max=100.0 step=1.0 width=fill height=18.0 -> amount_changed _
      progress amount vertical length=fill(2) girth=20.0 style=secondary background=background bar=primary/75 border=foreground border-width=1.0 radius=4.0 radius-tl=2.0
      progress amount style=success
      progress amount style=warning
      progress amount style=danger
      radio "First" value=0 selected=(choice == 0) -> choice_changed _
      rule horizontal thickness=2.0 style=weak fill=full color=primary/50 radius=4.0 radius-tl=2.0 snap=false
      rule horizontal fill=percent(75.0)
      rule horizontal fill=pad(4)
      rule horizontal fill=pad(4,8)
      space width=fill(2) height=shrink
      stack clip=true width=fill(2) height=120.0 under=1
        text "base"
        text "overlay"
    grid fluid=240.0 height=fill(2)
      text "fluid"
"#;
        let generated = compile(source, "controls.ice").unwrap();
        assert!(
            generated.contains("::iced::widget::grid(__children).spacing(8).spacing(12.0 as f32).width(640.0 as f32).height(::iced::widget::grid::aspect_ratio(16.0 as f32, 9.0 as f32)).columns(2 as usize)")
        );
        assert!(generated.contains(
            "::iced::widget::grid(__children).height(::iced::Length::FillPortion(2)).fluid(240.0 as f32)"
        ));
        assert!(generated.contains("::iced::widget::vertical_slider"));
        assert!(generated.contains(".default(50.0).shift_step(0.1).width(20.0 as f32).height(::iced::Length::FillPortion(2))"));
        assert!(generated.contains("::iced::widget::slider"));
        assert!(generated.contains(".width(::iced::Fill).height(18.0 as f32)"));
        assert!(generated.contains(".style(move |__theme, __status|"));
        assert!(generated.contains("slider::Status::Active"));
        assert!(generated.contains("slider::Status::Hovered"));
        assert!(generated.contains("slider::Status::Dragged"));
        assert!(generated.contains("slider::HandleShape::Circle"));
        assert!(generated.contains("slider::HandleShape::Rectangle"));
        assert!(generated.contains("__style.rail.backgrounds.0"));
        assert!(generated.contains("::iced::widget::progress_bar"));
        assert!(generated.contains(".vertical()"));
        assert!(generated.contains(".length(::iced::Length::FillPortion(2)).girth(20.0 as f32)"));
        assert!(generated.contains("progress_bar::secondary(__theme)"));
        assert!(generated.contains("progress_bar::success(__theme)"));
        assert!(generated.contains("progress_bar::warning(__theme)"));
        assert!(generated.contains("progress_bar::danger(__theme)"));
        assert!(generated.contains("__style.border.radius"));
        assert!(generated.contains("::iced::widget::radio"));
        assert!(generated.contains("::iced::widget::rule::weak(__theme)"));
        assert!(generated.contains("rule::FillMode::Full"));
        assert!(generated.contains("rule::FillMode::Percent(75.0 as f32)"));
        assert!(generated.contains("rule::FillMode::Padded(4)"));
        assert!(generated.contains("rule::FillMode::AsymmetricPadding(4, 8)"));
        assert!(generated.contains("__style.snap = false"));
        assert!(generated.contains(
            "::iced::widget::space().width(::iced::Length::FillPortion(2)).height(::iced::Shrink)"
        ));
        assert!(generated.contains("__children.split_off(__under)"));
        assert!(generated.contains("::iced::widget::Stack::new()"));
        assert!(generated.contains("__stack.push_under(__child)"));
        assert!(
            generated
                .contains(".clip(true).width(::iced::Length::FillPortion(2)).height(120.0 as f32)")
        );
    }

    #[test]
    fn lowers_complete_flex_layouts_and_wrapping() {
        let source = r#"app Layouts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  col width=fill height=shrink spacing=8.0 padding=1.0 padding-x=2.0 padding-y=3.0 padding-top=4.0 padding-right=5.0 padding-bottom=6.0 padding-left=7.0 max-width=640.0 align=center clip=true wrap wrap-spacing=12.0 wrap-align=end
    row width=fill(2) height=48.0 spacing=4.0 padding=2.0 align=end clip=false wrap wrap-spacing=6.0 wrap-align=start
      text "One"
      text "Two"
"#;
        let generated = compile(source, "layouts.ice").unwrap();
        assert!(generated.contains("::iced::widget::column(__children).spacing(8.0 as f32)"));
        assert!(generated.contains("::iced::Padding { top: 4.0 as f32, right: 5.0 as f32, bottom: 6.0 as f32, left: 7.0 as f32 }"));
        assert!(generated.contains(".width(::iced::Fill).height(::iced::Shrink)"));
        assert!(generated.contains(".max_width(640.0 as f32)"));
        assert!(generated.contains(
            ".align_x(::iced::alignment::Horizontal::Center).clip(true).wrap().horizontal_spacing(12.0 as f32).align_x(::iced::alignment::Vertical::Bottom)"
        ));
        assert!(generated.contains(".width(::iced::Length::FillPortion(2)).height(48.0 as f32)"));
        assert!(generated.contains(
            ".align_y(::iced::alignment::Vertical::Bottom).clip(false).wrap().vertical_spacing(6.0 as f32).align_x(::iced::alignment::Horizontal::Left)"
        ));
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
    fn lowers_searchable_combo_boxes() {
        let source = r#"app Search
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
  query = ""
on selected(next)
  selected = some(next)
on searched(next)
  query = next
on hovered(next)
on opened
on closed
view
  combo modes selected "Search modes" width=fill menu-height=120.0 padding=8.0 text-size=14.0 input=searched hover=hovered open=opened close=closed -> selected _
"#;
        let generated = compile(source, "search.ice").unwrap();
        assert!(
            generated.contains(
                "pub(crate) modes: ::iced::widget::combo_box::State<::std::string::String>"
            )
        );
        assert!(generated.contains(
            "::iced::widget::combo_box::State::new(::std::vec![\"List\".to_owned(), \"Board\".to_owned()])"
        ));
        assert!(generated.contains(
            "::iced::widget::combo_box(&self.modes, \"Search modes\", __combo_selection.as_ref()"
        ));
        assert!(generated.contains(".on_input(move |__value| __SearchMessage::Searched(__value))"));
        assert!(
            generated
                .contains(".on_option_hovered(move |__value| __SearchMessage::Hovered(__value))")
        );
    }

    #[test]
    fn lowers_structural_widgets_and_size_events() {
        let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  sensor_key = 0
  width = 0.0
  height = 0.0
on shown(w, h)
  width = w
  height = h
on resized(w, h)
  width = w
  height = h
on hidden
view
  col
    float scale=1.1 x=4.0 y=-2.0
      text "Floating"
    pin width=fill height=80.0 x=12.0 y=8.0
      text "Pinned"
    sensor show=shown resize=resized hide=hidden key=sensor_key anticipate=32.0 delay=10
      text "Observed"
    responsive at=600.0 width=fill height=40.0
      text "Narrow"
      text "Wide"
    responsive size=(available_width, available_height) width=fill height=fill
      col
        if available_width < available_height
          text "Portrait"
        if available_width >= available_height
          text "Landscape"
"#;
        let generated = compile(source, "structure.ice").unwrap();
        assert!(generated.contains("::iced::widget::float(__float_content).scale(1.1 as f32)"));
        assert!(generated.contains("::iced::widget::pin(__pin_content).x(12.0 as f32)"));
        assert!(generated.contains(
            ".on_show(move |__size| __StructureMessage::Shown(__size.width as f64, __size.height as f64))"
        ));
        assert!(generated.contains(".key(self.sensor_key)"));
        assert!(generated.contains("::iced::widget::responsive(move |__size|"));
        assert!(generated.contains("if __size.width < 600.0 as f32"));
        assert!(generated.contains("if ((__size.width as f64) < (__size.height as f64))"));
        assert!(generated.contains("if ((__size.width as f64) >= (__size.height as f64))"));
    }

    #[test]
    fn lowers_configured_scrollables_and_viewport_events() {
        let source = r#"app Scrolling
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  absolute_x = 0.0
  absolute_y = 0.0
  relative_x = 0.0
  relative_y = 0.0
on scrolled(ax, ay, rx, ry)
  absolute_x = ax
  absolute_y = ay
  relative_x = rx
  relative_y = ry
view
  scroll #feed direction=both width=fill height=200.0 bar=hidden bar-width=8.0 bar-margin=2.0 scroller-width=6.0 bar-spacing=4.0 anchor-x=end anchor-y=start auto=true scroll=scrolled
    col
      text "Scrollable"
"#;
        let generated = compile(source, "scrolling.ice").unwrap();
        assert!(generated.contains("scrollable::Direction::Both"));
        assert!(generated.contains("scrollable::Scrollbar::hidden().width(8.0 as f32)"));
        assert!(generated.contains(".anchor_x(::iced::widget::scrollable::Anchor::End)"));
        assert!(generated.contains(".auto_scroll(true)"));
        assert!(generated.contains("let __absolute = __viewport.absolute_offset()"));
        assert!(generated.contains(
            "__ScrollingMessage::Scrolled(__absolute.x as f64, __absolute.y as f64, __relative.x as f64, __relative.y as f64)"
        ));
    }

    #[test]
    fn lowers_extended_text_input_behavior() {
        let source = r#"app Form
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
  disabled = false
  secure = true
on submitted
on pasted(next)
  value = next
view
  input "Secret" #secret <-> value hint="Paste token" disabled=disabled secure=secure submit=submitted paste=pasted width=240.0 padding=8.0 text-size=14.0 line-height=1.2 align=center font=mono icon="•" icon-side=right icon-size=12.0 icon-spacing=4.0
"#;
        let generated = compile(source, "form.ice").unwrap();
        assert!(generated.contains(".secure(self.secure)"));
        assert!(generated.contains(".width(240.0 as f32).padding(8.0 as f32).size(14.0 as f32)"));
        assert!(generated.contains("LineHeight::Relative(1.2 as f32)"));
        assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
        assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
        assert!(generated.contains("code_point: '•'"));
        assert!(generated.contains(".on_submit_maybe(if self.disabled"));
        assert!(generated.contains(".on_paste_maybe(if self.disabled"));
    }

    #[test]
    fn lowers_button_children_and_typed_properties() {
        let source = r#"app Actions
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  disabled = false
on pressed
view
  button #action disabled=disabled width=fill height=48.0 padding=8.0 clip=true -> pressed
    row
      text "Save"
      text "⌘S"
"#;
        let generated = compile(source, "actions.ice").unwrap();
        assert!(generated.contains("let __button_content: ::iced::Element"));
        assert!(generated.contains("::iced::widget::row(__children)"));
        assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
        assert!(generated.contains(".padding(8.0 as f32).clip(true)"));
        assert!(generated.contains(".on_press_maybe(if self.disabled"));
    }

    #[test]
    fn lowers_checkbox_and_toggler_typography() {
        let source = r#"app Preferences
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  enabled = false
on changed(next)
  enabled = next
view
  col
    checkbox "Checkbox" checked=enabled size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono icon="✓" icon-size=12.0 icon-line-height=1.0 icon-shaping=basic -> changed _
    toggler "Toggler" checked=enabled size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=auto wrapping=glyph font=default align=right -> changed _
"#;
        let generated = compile(source, "preferences.ice").unwrap();
        assert!(generated.contains(".size(20.0 as f32).spacing(8.0 as f32)"));
        assert!(generated.contains(".width(::iced::Fill)"));
        assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
        assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
        assert!(generated.contains("checkbox::Icon"));
        assert!(generated.contains("code_point: '✓'"));
        assert!(generated.contains(".text_alignment(::iced::widget::text::Alignment::Right)"));
    }

    #[test]
    fn lowers_full_text_format() {
        let source = r#"app Typography
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Long text" width=fill height=40.0 size=16.0 line-height-px=20.0 font=mono align-x=justified align-y=center shaping=advanced wrapping=word-or-glyph @font-bold
"#;
        let generated = compile(source, "typography.ice").unwrap();
        assert!(generated.contains(".width(::iced::Fill).height(40.0 as f32)"));
        assert!(generated.contains("LineHeight::Absolute((20.0 as f32).into())"));
        assert!(generated.contains("text::Alignment::Justified"));
        assert!(generated.contains("alignment::Vertical::Center"));
        assert!(generated.contains("text::Shaping::Advanced"));
        assert!(generated.contains("text::Wrapping::WordOrGlyph"));
        assert!(generated.contains("..::iced::Font::MONOSPACE"));
    }

    #[test]
    fn lowers_declared_font_descriptors_and_app_default() {
        let source = r#"app Typography
font brand family="Inter" weight=semibold stretch=semi-expanded style=italic default=true
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Brand" font=brand @font-bold
"#;
        let generated = compile(source, "typography.ice").unwrap();
        assert!(generated.contains(".default_font(::iced::Font"));
        assert!(generated.contains("Family::Name(\"Inter\")"));
        assert!(generated.contains("Weight::Semibold"));
        assert!(generated.contains("Stretch::SemiExpanded"));
        assert!(generated.contains("Style::Italic"));
        assert!(generated.contains("weight: ::iced::font::Weight::Bold, ..::iced::Font"));
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
    fn lowers_native_keyboard_subscriptions() {
        let source = r#"app Shortcuts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  key = ""
  command = false
on pressed(event)
  key = event.key
  command = event.modifiers.command
on released(event)
  key = event.key
on modifiers_changed(modifiers)
  command = modifiers.command
subscribe
  keyboard press -> pressed _
  keyboard release -> released _
  keyboard modifiers -> modifiers_changed _
view
  text key
"#;
        let generated = compile(source, "shortcuts.ice").unwrap();
        assert!(generated.contains("struct __IceKeyPress"));
        assert!(generated.contains("struct __IceKeyRelease"));
        assert!(generated.contains("struct __IceKeyModifiers"));
        assert!(generated.contains("::iced::keyboard::listen().filter_map"));
        assert!(generated.contains("::iced::keyboard::Event::KeyPressed"));
        assert!(generated.contains("::iced::keyboard::Event::KeyReleased"));
        assert!(generated.contains("::iced::keyboard::Event::ModifiersChanged"));
        assert!(generated.contains("self.key = event.key.clone()"));
        assert!(generated.contains("self.command = event.modifiers.command.clone()"));
    }

    #[test]
    fn lowers_native_system_tasks_and_subscription() {
        let source = r#"app Diagnostics
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  cpu = ""
  mode = "none"
on inspect
  task system info -> inspected _
on inspected(info)
  cpu = info.cpu_brand
on read_theme
  task system theme -> theme_changed _
on theme_changed(next)
  mode = next
subscribe
  system theme -> theme_changed _
view
  text cpu
"#;
        let generated = compile(source, "diagnostics.ice").unwrap();
        assert!(generated.contains("struct __IceSystemInfo"));
        assert!(generated.contains("fn __ice_system_info(value: ::iced::system::Information)"));
        assert!(generated.contains("::iced::system::information().map(__ice_system_info)"));
        assert!(generated.contains("::iced::system::theme().map(__ice_system_theme)"));
        assert!(generated.contains("::iced::system::theme_changes().map(__ice_system_theme)"));
        assert!(generated.contains("self.cpu = info.cpu_brand.clone()"));
    }

    #[test]
    fn lowers_native_clipboard_tasks() {
        let source = r#"app Clipboard
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value:str? = none
on read
  task clipboard read -> read_done _
on read_done(next)
  value = next
on read_primary
  task clipboard read-primary -> read_done _
on write
  task clipboard write "copied"
on write_primary
  task clipboard write-primary "selected"
view
  text "Clipboard"
"#;
        let generated = compile(source, "clipboard.ice").unwrap();
        assert!(generated.contains("::iced::clipboard::read().map"));
        assert!(generated.contains("::iced::clipboard::read_primary().map"));
        assert!(generated.contains("::iced::clipboard::write::<__ClipboardMessage>"));
        assert!(generated.contains("::iced::clipboard::write_primary::<__ClipboardMessage>"));
    }

    #[test]
    fn lowers_all_static_widget_operations() {
        let source = r#"app Operations
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
on checked(value)
on previous
  task widget focus-previous
on next
  task widget focus-next
on focus
  task widget focus #field
on check
  task widget focused #field -> checked _
on front
  task widget cursor-front #field
on end
  task widget cursor-end #field
on cursor
  task widget cursor #field 2
on all
  task widget select-all #field
on range
  task widget select #field 1 3
on snap
  task widget snap #list 0.0 1.0
on snap_end
  task widget snap-end #list
on scroll_to
  task widget scroll-to #list 0.0 24.0
on scroll_by
  task widget scroll-by #list -4.0 8.0
view
  col
    input "Value" #field <-> value
    scroll #list
      text "Content"
"#;
        let generated = compile(source, "operations.ice").unwrap();
        for function in [
            "focus_previous",
            "focus_next",
            "focus::<",
            "is_focused",
            "move_cursor_to_front",
            "move_cursor_to_end",
            "move_cursor_to::<",
            "select_all",
            "select_range",
            "snap_to::<",
            "snap_to_end",
            "scroll_to::<",
            "scroll_by::<",
        ] {
            assert!(generated.contains(function), "missing {function}");
        }
        assert!(generated.contains("Id::new(\"Operations/field\")"));
        assert!(generated.contains("Id::new(\"Operations/list\")"));
        assert!(generated.contains("RelativeOffset { x: (0.0) as f32, y: (1.0) as f32 }"));
        assert!(generated.contains("AbsoluteOffset"));
        assert!(generated.contains("(-4.0)"));
    }

    #[test]
    fn checks_and_lowers_main_window_tasks() {
        let source = r#"app WindowTasks
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on closed
on size_read(width, height)
on bool_read(value)
on optional_bool_read(value)
on optional_pair_read(x, y)
on scale_read(value)
on mode_read(value)
on close_window
  task window close
on drag_window
  task window drag
on drag_resize_window
  task window drag-resize north-east
on resize_window
  task window resize 800.0 600.0
on resizable_window
  task window resizable true
on min_size_window
  task window min-size 320.0 240.0
on clear_min_size
  task window min-size none
on max_size_window
  task window max-size 1920.0 1080.0
on resize_increments_window
  task window resize-increments 8.0 16.0
on read_size
  task window size -> size_read _ _
on read_maximized
  task window maximized -> bool_read _
on maximize_window
  task window maximize true
on read_minimized
  task window minimized -> optional_bool_read _
on minimize_window
  task window minimize false
on read_position
  task window position -> optional_pair_read _ _
on read_scale
  task window scale-factor -> scale_read _
on move_window
  task window move -10.0 20.0
on read_mode
  task window mode -> mode_read _
on mode_window
  task window set-mode fullscreen
on toggle_maximize_window
  task window toggle-maximize
on toggle_decorations_window
  task window toggle-decorations
on attention_window
  task window attention informational
on clear_attention
  task window attention none
on focus_window
  task window focus
on level_window
  task window level always-on-top
on system_menu_window
  task window system-menu
on passthrough_window
  task window mouse-passthrough false
on read_monitor
  task window monitor-size -> optional_pair_read _ _
on automatic_tabbing
  task window automatic-tabbing false
view
  text "Window"
"#;
        let generated = compile(source, "window_tasks.ice").unwrap();
        for function in [
            "window::close",
            "window::drag",
            "window::drag_resize",
            "window::resize",
            "window::set_resizable",
            "window::set_min_size",
            "window::set_max_size",
            "window::set_resize_increments",
            "window::size",
            "window::is_maximized",
            "window::maximize",
            "window::is_minimized",
            "window::minimize",
            "window::position",
            "window::scale_factor",
            "window::move_to",
            "window::mode",
            "window::set_mode",
            "window::toggle_maximize",
            "window::toggle_decorations",
            "window::request_user_attention",
            "window::gain_focus",
            "window::set_level",
            "window::show_system_menu",
            "window::enable_mouse_passthrough",
            "window::disable_mouse_passthrough",
            "window::monitor_size",
            "window::allow_automatic_tabbing",
        ] {
            assert!(generated.contains(function), "missing {function}");
        }
        assert!(generated.contains("window::oldest().and_then"));

        let error = compile(
            &source.replace("task window close", "task window close -> closed"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E173");

        let error = compile(
            &source.replace("resize 800.0 600.0", "resize -1.0 600.0"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E128");

        let error = compile(
            &source.replace("size -> size_read _ _", "size -> size_read _"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E129");
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
on moved(x, y)
on scrolled(x, y, pixels)
view
  col
    image "photo.ppm" width=fill height=64.0 fit=cover filter=nearest rotation=0.5 opacity=0.8 scale=1.2 expand=true radius=4.0
    svg "icon.svg" width=48.0 height=shrink fit=scale-down rotation=0.1 opacity=0.9
    tooltip position=cursor gap=2.0 padding=5.0 delay=100 snap=false style=success background=background text=foreground border=primary/75 border-width=1.0 radius=5.0 radius-tl=2.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true
      mouse enter=entered exit=exited press=pressed move=moved scroll=scrolled cursor=pointer
        text "Hover"
      text "Tip"
"#;
        let generated = compile(source, "media.ice").unwrap();
        assert!(generated.contains("::iced::widget::image(\"photo.ppm\".to_owned())"));
        assert!(generated.contains(".filter_method(::iced::widget::image::FilterMethod::Nearest)"));
        assert!(generated.contains("::iced::widget::svg(\"icon.svg\".to_owned())"));
        assert!(generated.contains("tooltip::Position::FollowCursor"));
        assert!(generated.contains(".delay(::std::time::Duration::from_millis(100 as u64))"));
        assert!(generated.contains("container::success(__theme)"));
        assert!(generated.contains("__style.background = Some("));
        assert!(generated.contains("__style.border.radius"));
        assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
        assert!(generated.contains("__style.shadow.blur_radius = 8.0 as f32"));
        assert!(generated.contains("__style.snap = true"));
        assert!(generated.contains(".on_enter(__MediaMessage::Entered)"));
        assert!(generated.contains(
            ".on_move(move |__point| __MediaMessage::Moved(__point.x as f64, __point.y as f64))"
        ));
        assert!(generated.contains("::iced::mouse::ScrollDelta::Lines"));
        assert!(generated.contains("__MediaMessage::Scrolled(__x as f64, __y as f64, true)"));
        assert!(generated.contains(".interaction(::iced::mouse::Interaction::Pointer)"));
    }
}
