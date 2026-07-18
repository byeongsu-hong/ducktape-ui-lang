use super::*;

pub(super) fn generate_theme(out: &mut String, document: &Document) -> Result<(), Error> {
    let env = state_env(document, "self");
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
    writeln!(out, "fn __app_theme() -> ::iced::Theme {{").unwrap();
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
    writeln!(out, "fn __theme(&self) -> ::iced::Theme {{").unwrap();
    if let Some(setting) = &document.settings.theme {
        if let Expr::Call { name, args } = &setting.value
            && document
                .functions
                .iter()
                .any(|function| function.name == *name && function.kind == ExternKind::Theme)
        {
            writeln!(out, "{}", theme_factory_code(name, args, &env, document)?).unwrap();
        } else {
            let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
            writeln!(out, "match ({value}).as_str() {{").unwrap();
            writeln!(out, "\"app\" => Self::__app_theme(),").unwrap();
            writeln!(out, "\"default\" => <::iced::Theme as ::iced::theme::Base>::default(::iced::theme::Mode::None),").unwrap();
            for name in BUILT_IN_THEMES {
                writeln!(out, "\"{name}\" => ::iced::Theme::{},", pascal(name)).unwrap();
            }
            writeln!(out, "_ => Self::__app_theme(),\n}}").unwrap();
        }
    } else {
        writeln!(out, "Self::__app_theme()").unwrap();
    }
    writeln!(out, "}}").unwrap();
    if let Some(setting) = &document.settings.title {
        let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __title(&self) -> ::std::string::String {{ {value} }}"
        )
        .unwrap();
    }
    if document.settings.background.is_some() || document.settings.text_color.is_some() {
        writeln!(out, "fn __style(&self, __theme: &::iced::Theme) -> ::iced::theme::Style {{ let mut __style = ::iced::theme::Base::base(__theme);").unwrap();
        for (setting, field) in [
            (&document.settings.background, "background_color"),
            (&document.settings.text_color, "text_color"),
        ] {
            if let Some(setting) = setting {
                let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
                writeln!(out, "__style.{field} = ({value}).parse::<::iced::Color>().unwrap_or(__style.{field});").unwrap();
            }
        }
        writeln!(out, "__style }}").unwrap();
    }
    if let Some(setting) = &document.settings.scale_factor {
        let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __scale_factor(&self) -> f32 {{ (({value}) as f32).max(f32::EPSILON) }}"
        )
        .unwrap();
    }
    Ok(())
}

pub(super) fn generate_boot(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    writeln!(out, "fn __state() -> Self {{\nSelf {{").unwrap();
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
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            ..
        } = node
        else {
            unreachable!()
        };
        writeln!(
            out,
            "{}: ::iced::widget::pane_grid::State::with_configuration({}),",
            pane_field(name),
            pane_configuration_code(configuration)
        )
        .unwrap();
    }
    writeln!(
        out,
        "}}\n}}\nfn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();"
    )
    .unwrap();
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

pub(super) fn generate_presets(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    for (index, preset) in document.presets.iter().enumerate() {
        writeln!(
            out,
            "fn __preset_{index}() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\nlet task = (|| {{"
        )
        .unwrap();
        let env = state_env(document, "state");
        let has_task = generate_statements(
            out,
            &preset.statements,
            document,
            message,
            &env,
            "state",
            false,
        )?;
        if !has_task {
            writeln!(out, "::iced::Task::none()").unwrap();
        }
        writeln!(out, "}})();\n(state, task)\n}}").unwrap();
    }
    Ok(())
}

pub(super) fn generate_update(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
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
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid { name, options, .. } = node else {
            unreachable!()
        };
        if options.resize_leeway.is_some() {
            writeln!(
                out,
                "{message}::{}(__event) => {{ self.{}.resize(__event.split, __event.ratio); ::iced::Task::none() }},",
                pane_resize_variant(name),
                pane_field(name)
            )
            .unwrap();
        }
        if options.draggable {
            writeln!(
                out,
                "{message}::{}(__event) => {{ if let ::iced::widget::pane_grid::DragEvent::Dropped {{ pane, target }} = __event {{ self.{}.drop(pane, target); }} ::iced::Task::none() }},",
                pane_drag_variant(name),
                pane_field(name)
            )
            .unwrap();
        }
    }
    for binding in controlled_state_bindings(document, false)
        .expect("checker validates controlled input bindings")
    {
        let variant = binding_variant(&binding);
        writeln!(
            out,
            "{message}::{variant}(value) => {{ self.{binding} = value; ::iced::Task::none() }}"
        )
        .unwrap();
    }
    for binding in controlled_state_bindings(document, true)
        .expect("checker validates controlled editor bindings")
    {
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

pub(super) fn subscription_payload_arity(source: &SubscriptionSource, window_id: bool) -> usize {
    let arity = match source {
        SubscriptionSource::Every { .. }
        | SubscriptionSource::Repeat { .. }
        | SubscriptionSource::Run { .. }
        | SubscriptionSource::Recipe { .. }
        | SubscriptionSource::Events { .. }
        | SubscriptionSource::Extern { .. }
        | SubscriptionSource::Event { .. }
        | SubscriptionSource::Keyboard(_)
        | SubscriptionSource::SystemTheme => 1,
        SubscriptionSource::InputMethod(InputMethodEvent::Opened | InputMethodEvent::Closed)
        | SubscriptionSource::Mouse(MouseEvent::Entered | MouseEvent::Left)
        | SubscriptionSource::Window(
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft,
        ) => 0,
        SubscriptionSource::InputMethod(InputMethodEvent::Commit)
        | SubscriptionSource::Mouse(MouseEvent::Pressed | MouseEvent::Released)
        | SubscriptionSource::Window(
            WindowEvent::Rescaled | WindowEvent::FileHovered | WindowEvent::FileDropped,
        ) => 1,
        SubscriptionSource::Mouse(MouseEvent::Moved)
        | SubscriptionSource::Window(WindowEvent::Moved | WindowEvent::Resized) => 2,
        SubscriptionSource::InputMethod(InputMethodEvent::Preedit)
        | SubscriptionSource::Mouse(MouseEvent::Wheel)
        | SubscriptionSource::Touch(_) => 3,
        SubscriptionSource::Window(WindowEvent::Opened) => 4,
    };
    arity + usize::from(window_id)
}

pub(super) fn identified_window_filter(filter: &str, arity: usize) -> String {
    match arity {
        0 => format!("({filter}).map(|_| __id)"),
        1 => format!("({filter}).map(|__value| (__id, __value))"),
        count => format!(
            "({filter}).map(|__value| (__id, {}))",
            (0..count)
                .map(|index| format!("__value.{index}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
