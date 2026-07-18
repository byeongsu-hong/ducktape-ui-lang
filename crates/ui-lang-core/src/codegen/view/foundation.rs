use super::*;

pub(in crate::codegen) fn render_foundation(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
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
        ViewNode::Container {
            options,
            id,
            styles,
            content,
            ..
        } => render_container(
            options, id, styles, content, document, message, env, scope, slot,
        ),
        ViewNode::Overlay {
            options,
            content,
            layer,
            ..
        } => render_overlay(options, content, layer, document, message, env, scope, slot),
        ViewNode::PaneGrid {
            name,
            options,
            panes,
            templates,
            ..
        } => render_pane_grid(
            name, options, panes, templates, document, message, env, scope, slot,
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
        ViewNode::RichText {
            options,
            color,
            spans,
            styles,
            route,
            ..
        } => render_rich_text(options, color, spans, styles, route, document, message, env),
        ViewNode::Input {
            label,
            id,
            binding,
            hint,
            disabled,
            options,
            styles,
            span,
        } => {
            let style = Style::parse(styles, document);
            let state = env.get(binding).ok_or_else(|| {
                Error::new("E150", span, format!("unknown input state `{binding}`"))
            })?;
            let state_name = controlled_state_name(&state.code, "input", span)?;
            let variant = binding_variant(&state_name);
            let mut input = format!(
                "::iced::widget::text_input({}, &{})",
                rust_string(hint),
                state.code
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
            if let Some(icon) = &options.icon {
                write!(
                    input,
                    ".icon({})",
                    text_input_icon_code(icon, env, document)?
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
            input.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                Some(&style),
                env,
                document,
                "style",
                "text_input",
            )?);
            Ok(format!(
                "::iced::widget::column![::iced::widget::text({}), {input}].spacing(6).into()",
                rust_string(label)
            ))
        }
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
