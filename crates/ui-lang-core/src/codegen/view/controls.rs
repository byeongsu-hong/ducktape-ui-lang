use super::*;

pub(in crate::codegen) fn render_controls(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Button {
            label,
            content,
            id,
            disabled,
            options,
            styles,
            route,
            span,
        } => {
            let style = Style::parse(styles, document);
            let message_code = route_code(route, "", env, document, message)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "button", span, scope, env, document)?;
            let accessibility_label = options
                .accessibility
                .label
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| {
                    rust_string(label.as_ref().expect("checked button accessibility label"))
                });
            let accessibility_description = options
                .accessibility
                .description
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .map(|value| format!(".description({value})"))
                .unwrap_or_default();
            let disabled_value = disabled
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
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
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __disabled = {disabled_value}; let __activate = {message_code}; let __button_content: __IceElement<'_, {message}> = {content}; let __button = ::iced::widget::button(__button_content)"
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
            code.push_str(
                ".on_press_maybe(if __disabled { None } else { Some(__activate.clone()) })",
            );
            code.push_str(&button_style_code(&style, &options.style, env, document)?);
            Ok(format!(
                "{code}; ::ui_lang_runtime::accessible(__button, __a11y_id, ::ui_lang_runtime::Role::Button).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
            ))
        }
        ViewNode::Checkbox {
            label,
            id,
            checked,
            disabled,
            options,
            style,
            route,
            span,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "checkbox", span, scope, env, document)?;
            let accessibility_label = options
                .accessibility
                .label
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "__label.clone()".into());
            let accessibility_description = options
                .accessibility
                .description
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .map(|value| format!(".description({value})"))
                .unwrap_or_default();
            let disabled_value = disabled
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
            let mut code = format!(
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __label = {label}; let __checked = {checked}; let __disabled = {disabled_value}; let __activate = {{ let __value = !__checked; {message_code} }}; let __checkbox = ::iced::widget::checkbox(__checked).label(__label.clone())"
            );
            append_bool_control_options(&mut code, options, env, document, false)?;
            write!(
                code,
                ".on_toggle_maybe(if __disabled {{ None }} else {{ Some({callback}) }})"
            )
            .unwrap();
            code.push_str(&checkbox_style_code(style, env, document)?);
            Ok(format!(
                "{code}; ::ui_lang_runtime::accessible(__checkbox, __a11y_id, ::ui_lang_runtime::Role::CheckBox).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).checked(__checked).disabled(__disabled).on_activate_maybe(if __disabled {{ None }} else {{ Some(__activate) }}){accessibility_description}.into() }}"
            ))
        }
        ViewNode::Toggler {
            label,
            checked,
            disabled,
            options,
            style,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let mut code = format!("::iced::widget::toggler({checked}).label({label})");
            append_bool_control_options(&mut code, options, env, document, true)?;
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    ".on_toggle_maybe(if {disabled} {{ None }} else {{ Some({callback}) }})"
                )
                .unwrap();
            } else {
                write!(code, ".on_toggle({callback})").unwrap();
            }
            code.push_str(&toggler_style_code(style, env, document)?);
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
            let value = expr_code(value, env, document, ValueMode::Borrowed)?;
            let min = expr_code(min, env, document, ValueMode::Borrowed)?;
            let max = expr_code(max, env, document, ValueMode::Borrowed)?;
            let step = expr_code(step, env, document, ValueMode::Borrowed)?;
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let helper = if *vertical {
                "vertical_slider"
            } else {
                "slider"
            };
            let mut code = format!(
                "::iced::widget::{helper}(({min})..=({max}), {value}, {callback}).step({step})"
            );
            if let Some(default) = &options.default {
                write!(
                    code,
                    ".default({})",
                    expr_code(default, env, document, ValueMode::Borrowed)?
                )
                .unwrap();
            }
            if let Some(shift_step) = &options.shift_step {
                write!(
                    code,
                    ".shift_step({})",
                    expr_code(shift_step, env, document, ValueMode::Borrowed)?
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
            options,
            style,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let callback = route_callback_code(route, "_", &value, env, document, message)?;
            let mut code = format!(
                "::iced::widget::radio({label}, true, if {selected} {{ Some(true) }} else {{ None }}, {callback})"
            );
            append_bool_control_options(&mut code, options, env, document, false)?;
            code.push_str(&radio_style_code(style, env, document)?);
            Ok(format!("{code}.into()"))
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
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let mut code = format!("::iced::widget::pick_list({options}, {selected}, {callback})");
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
            if let Some(height) = &options_config.line_height {
                write!(
                    code,
                    ".text_line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(shaping) = options_config.shaping {
                write!(
                    code,
                    ".text_shaping(::iced::widget::text::Shaping::{})",
                    text_shaping_code(shaping)
                )
                .unwrap();
            }
            if let Some(font) = &options_config.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(handle) = &options_config.handle {
                write!(
                    code,
                    ".handle({})",
                    pick_list_handle_code(handle, env, document)?
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
            code.push_str(&pick_list_style_code(options_config, env, document)?);
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
            let callback =
                route_callback_code(route, "__value", "__value", env, document, message)?;
            let mut code = format!(
                "{{ let __combo_selection = {selected}; ::iced::widget::combo_box(&{}, {}, __combo_selection.as_ref(), {callback})",
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
            if let Some(height) = &options.line_height {
                write!(
                    code,
                    ".line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
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
            if let Some(font) = &options.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(icon) = &options.icon {
                write!(
                    code,
                    ".icon({})",
                    text_input_icon_code(icon, env, document)?
                )
                .unwrap();
            }
            if let Some(route) = &options.input {
                let callback =
                    route_callback_code(route, "__value", "__value", env, document, message)?;
                write!(code, ".on_input({callback})").unwrap();
            }
            if let Some(route) = &options.hover {
                let callback =
                    route_callback_code(route, "__value", "__value", env, document, message)?;
                write!(code, ".on_option_hovered({callback})").unwrap();
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
            code.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                None,
                env,
                document,
                "input_style",
                "text_input",
            )?);
            code.push_str(&menu_style_code(
                options.menu_style.as_deref(),
                options.custom_menu_style.as_ref(),
                env,
                document,
            )?);
            Ok(format!("{code}.into() }}"))
        }
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
