use super::*;

pub(in crate::codegen) fn generate_theme(
    out: &mut String,
    document: &Document,
) -> Result<(), Error> {
    let state_env = state_env(document, "self");
    let mut callback_env = state_env.clone();
    if document.daemon {
        callback_env.insert(
            "window".into(),
            Binding {
                code: "window".into(),
                ty: Type::WindowId,
                local: true,
            },
        );
    }
    let callback_arg = if document.daemon {
        ", window: ::iced::window::Id"
    } else {
        ""
    };
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
    writeln!(out, "fn __theme(&self{callback_arg}) -> ::iced::Theme {{").unwrap();
    if let Some(setting) = &document.settings.theme {
        if let Expr::Call { name, args } = &setting.value
            && document
                .functions
                .iter()
                .any(|function| function.name == *name && function.kind == ExternKind::Theme)
        {
            writeln!(
                out,
                "{}",
                theme_factory_code(name, args, &callback_env, document)?
            )
            .unwrap();
        } else {
            let value = expr_code(&setting.value, &callback_env, document, ValueMode::Owned)?;
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
        let value = expr_code(&setting.value, &callback_env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __title(&self{callback_arg}) -> ::std::string::String {{ {value} }}"
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
                let value = expr_code(&setting.value, &state_env, document, ValueMode::Owned)?;
                writeln!(out, "__style.{field} = ({value}).parse::<::iced::Color>().unwrap_or(__style.{field});").unwrap();
            }
        }
        writeln!(out, "__style }}").unwrap();
    }
    if let Some(setting) = &document.settings.scale_factor {
        let value = expr_code(&setting.value, &callback_env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __scale_factor(&self{callback_arg}) -> f32 {{ (({value}) as f32).max(f32::EPSILON) }}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_boot(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(&document.app);
    writeln!(out, "fn __state() -> Self {{").unwrap();
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            templates,
            ..
        } = node
        else {
            unreachable!()
        };
        let field = pane_field(name);
        writeln!(
            out,
            "let {field} = ::iced::widget::pane_grid::State::with_configuration({});",
            pane_configuration_code(
                configuration,
                (!templates.is_empty()).then(|| pane_type(name)).as_deref()
            )
        )
        .unwrap();
        let slots = pane_split_slots(configuration);
        if slots.iter().any(Option::is_some) {
            let slots = slots
                .iter()
                .map(|name| {
                    name.map_or_else(
                        || "::std::option::Option::None".into(),
                        |name| format!("::std::option::Option::Some({})", rust_string(name)),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                out,
                "let {} = [{slots}].into_iter().zip({field}.layout().splits().copied()).filter_map(|(__name, __split)| __name.map(|__name| (__name, __split))).collect();",
                pane_splits_field(name)
            )
            .unwrap();
        }
    }
    writeln!(out, "Self {{").unwrap();
    let accessibility_bridge = if document.daemon {
        "::ui_lang_runtime::Bridge::without_native_adapter()"
    } else {
        "::ui_lang_runtime::Bridge::new()"
    };
    writeln!(out, "__ice_accessibility: {accessibility_bridge},").unwrap();
    for qr in &document.qr_codes {
        writeln!(out, "{}: {},", qr.name, qr_data_code(qr)).unwrap();
    }
    for state in &document.states {
        writeln!(out, "{}: {},", state.name, initial_code(state, document)).unwrap();
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
        writeln!(out, "{},", pane_field(name)).unwrap();
        if pane_split_slots(configuration).iter().any(Option::is_some) {
            writeln!(out, "{},", pane_splits_field(name)).unwrap();
        }
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
    if document.daemon {
        writeln!(out, "(state, task)\n}}").unwrap();
    } else {
        writeln!(
            out,
            "let __accessibility = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\n(state, ::iced::Task::batch([task, __accessibility]))\n}}"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn generate_presets(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(&document.app);
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
        if document.daemon {
            writeln!(out, "}})();\n(state, task)\n}}").unwrap();
        } else {
            writeln!(
                out,
                "}})();\nlet __accessibility = ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)));\n(state, ::iced::Task::batch([task, __accessibility]))\n}}"
            )
            .unwrap();
        }
    }
    Ok(())
}

pub(in crate::codegen) fn generate_update(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let accessibility_root = rust_string(&document.app);
    let has_fallthrough_arm = document
        .handlers
        .iter()
        .any(|handler| handler.name != "mount")
        || pane_grids(&document.view).into_iter().any(|node| {
            matches!(node, ViewNode::PaneGrid { options, .. } if options.resize_leeway.is_some() || options.draggable)
        })
        || !controlled_state_bindings(document, false)
            .expect("checker validates controlled input bindings")
            .is_empty()
        || !controlled_state_bindings(document, true)
            .expect("checker validates controlled editor bindings")
            .is_empty()
        || needs_extern_noop(document);
    let task_binding = if has_fallthrough_arm {
        "let __task = "
    } else {
        ""
    };
    writeln!(
        out,
        "fn __update(&mut self, message: {message}) -> ::iced::Task<{message}> {{\n{task_binding}match message {{\n{message}::__AccessibilitySnapshot(__snapshot) => {{ self.__ice_accessibility.update(*__snapshot); return ::iced::Task::none(); }},\n{message}::__AccessibilityAction(__request) => {{ let __refresh = matches!(__request.action, ::ui_lang_runtime::Action::Focus); let __task = self.__ice_accessibility.dispatch(__request); return if __refresh {{ __task.chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))) }} else {{ __task }}; }},\n{message}::__AccessibilityWindow(__id, __event) => {{ self.__ice_accessibility.window_event(__id, __event); return ::iced::Task::none(); }},\n{message}::__AccessibilityFocusNext => {{ return ::ui_lang_runtime::focus_next::<{message}>().chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))); }},\n{message}::__AccessibilityFocusPrevious => {{ return ::ui_lang_runtime::focus_previous::<{message}>().chain(::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))); }},"
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
        // Keep statement-level `return` inside this arm so every user update
        // reaches the post-state accessibility snapshot below.
        writeln!(out, "{pattern} => (|| {{").unwrap();
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
        writeln!(out, "}})(),").unwrap();
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
    if has_animations(document) {
        writeln!(
            out,
            "{message}::__AnimationFrame => return ::iced::Task::none(),"
        )
        .unwrap();
    }
    if !has_fallthrough_arm {
        writeln!(out, "}}\n}}").unwrap();
        return Ok(());
    }
    if document.daemon {
        writeln!(out, "}};\n__task\n}}").unwrap();
    } else {
        writeln!(
            out,
            "}};\n::iced::Task::batch([__task, ::ui_lang_runtime::snapshot::<{message}>({accessibility_root}).map(|__snapshot| {message}::__AccessibilitySnapshot(::std::boxed::Box::new(__snapshot)))])\n}}"
        )
        .unwrap();
    }
    Ok(())
}
