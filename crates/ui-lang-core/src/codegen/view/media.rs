use super::*;

pub(in crate::codegen) fn render_media(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Media {
            kind,
            source,
            options,
            span,
        } => {
            let source_type = expr_type(
                source,
                &env.iter()
                    .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                    .collect(),
                document,
                span,
            )?;
            let source = expr_code(source, env, document, ValueMode::Owned)?;
            let mut code = match kind {
                MediaKind::Image => format!("::iced::widget::image({source})"),
                MediaKind::Viewer if source_type == Type::Str => format!(
                    "::iced::widget::image::viewer(::iced::widget::image::Handle::from_path({source}))"
                ),
                MediaKind::Viewer => format!("::iced::widget::image::viewer({source})"),
                MediaKind::Svg if options.svg_memory && source_type == Type::Bytes => format!(
                    "::iced::widget::svg(::iced::widget::svg::Handle::from_memory({source}))"
                ),
                MediaKind::Svg if options.svg_memory => format!(
                    "::iced::widget::svg(::iced::widget::svg::Handle::from_memory(({source}).into_bytes()))"
                ),
                MediaKind::Svg => format!("::iced::widget::svg({source})"),
            };
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            if let Some(fit) = &options.fit {
                write!(
                    code,
                    ".content_fit({})",
                    expr_code(fit, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(rotation) = &options.rotation {
                let rotation_type = expr_type(rotation, &env_types(env), document, span)?;
                let rotation = expr_code(rotation, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    ".rotation({})",
                    if rotation_type == Type::Rotation {
                        rotation
                    } else if options.rotation_solid {
                        format!("::iced::Rotation::Solid(::iced::Radians({rotation} as f32))")
                    } else {
                        format!("{rotation} as f32")
                    }
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
            if *kind == MediaKind::Svg {
                let custom = options
                    .svg_style
                    .as_ref()
                    .map(|style| {
                        let function = document
                            .functions
                            .iter()
                            .find(|item| {
                                item.name == style.function && item.kind == ExternKind::SvgStyle
                            })
                            .expect("checker validates svg style");
                        let args = style
                            .args
                            .iter()
                            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok::<_, Error>(format!(
                            "{}(__theme, __status{})",
                            function.rust_path,
                            args.iter()
                                .map(|arg| format!(", {arg}"))
                                .collect::<String>()
                        ))
                    })
                    .transpose()?;
                let has_colors = options.svg_color.is_some() || options.svg_hover_color.is_some();
                if !has_colors {
                    if let Some(custom) = custom {
                        write!(code, ".style(move |__theme, __status| {custom})").unwrap();
                    }
                } else {
                    let base = custom
                        .unwrap_or_else(|| "::iced::widget::svg::Style::default()".to_owned());
                    let idle = options
                        .svg_color
                        .as_ref()
                        .map(|color| format!("Some({})", theme_color(document, color)));
                    let hovered = match &options.svg_hover_color {
                        Some(Some(color)) => {
                            Some(format!("Some({})", theme_color(document, color)))
                        }
                        Some(None) => Some("None".to_owned()),
                        None => idle.clone(),
                    };
                    write!(
                        code,
                        ".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{"
                    )
                    .unwrap();
                    if let Some(idle) = idle {
                        write!(
                            code,
                            " ::iced::widget::svg::Status::Idle => __style.color = {idle},"
                        )
                        .unwrap();
                    }
                    if let Some(hovered) = hovered {
                        write!(
                            code,
                            " ::iced::widget::svg::Status::Hovered => __style.color = {hovered},"
                        )
                        .unwrap();
                    }
                    code.push_str(" _ => {} } __style })");
                }
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
            for (value, method) in [
                (&options.padding, "padding"),
                (&options.min_scale, "min_scale"),
                (&options.max_scale, "max_scale"),
                (&options.scale_step, "scale_step"),
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
            if let Some(radius) = radius_code(
                options.radius.as_ref(),
                [
                    options.radius_top_left.as_ref(),
                    options.radius_top_right.as_ref(),
                    options.radius_bottom_right.as_ref(),
                    options.radius_bottom_left.as_ref(),
                ],
                env,
                document,
            )? {
                write!(code, ".border_radius({radius})").unwrap();
            }
            if let Some([x, y, width, height]) = &options.crop {
                write!(
                    code,
                    ".crop(::iced::Rectangle {{ x: {}, y: {}, width: {}, height: {} }})",
                    u32_code(x, env, document)?,
                    u32_code(y, env, document)?,
                    u32_code(width, env, document)?,
                    u32_code(height, env, document)?,
                )
                .unwrap();
            }
            if let Some(label) = &options.accessibility.label {
                let accessibility_key =
                    accessibility_key_code(None, "media", span, scope, env, document)?;
                let label = expr_code(label, env, document, ValueMode::Owned)?;
                let description = options
                    .accessibility
                    .description
                    .as_ref()
                    .map(|value| expr_code(value, env, document, ValueMode::Owned))
                    .transpose()?
                    .map(|value| format!(".description({value})"))
                    .unwrap_or_default();
                Ok(format!(
                    "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); ::ui_lang_runtime::accessible({code}, __a11y_id, ::ui_lang_runtime::Role::Image).label({label}){description}.into() }}"
                ))
            } else {
                Ok(format!("{code}.into()"))
            }
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
                "{{ let __tooltip_content: __IceElement<'_, {message}> = {content}; let __tooltip_tip: __IceElement<'_, {message}> = {tip}; ::iced::widget::tooltip(__tooltip_content, __tooltip_tip, ::iced::widget::tooltip::Position::{position}).gap({gap} as f32).padding({padding} as f32).delay(::std::time::Duration::from_millis({delay} as u64)).snap_within_viewport({snap})"
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
                "{{ let __mouse_content: __IceElement<'_, {message}> = {content}; ::iced::widget::mouse_area(__mouse_content)"
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
                let callback = ordered_route_callback_code(
                    route,
                    "__point",
                    &["__point.x as f64", "__point.y as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, ".on_move({callback})").unwrap();
            }
            if let Some(route) = &options.scroll {
                let callback = route_callback_with_code(
                    route,
                    "__delta",
                    env,
                    document,
                    |callback_env| {
                        let lines = ordered_route_code(
                            route,
                            &["__x as f64", "__y as f64", "false"],
                            callback_env,
                            document,
                            message,
                        )?;
                        let pixels = ordered_route_code(
                            route,
                            &["__x as f64", "__y as f64", "true"],
                            callback_env,
                            document,
                            message,
                        )?;
                        Ok(format!(
                            "match __delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {lines}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {pixels} }}"
                        ))
                    },
                )?;
                write!(code, ".on_scroll({callback})").unwrap();
            }
            if let Some(interaction) = options.interaction {
                write!(
                    code,
                    ".interaction(::iced::mouse::Interaction::{})",
                    mouse_interaction_code(interaction)
                )
                .unwrap();
            } else if let Some(interaction) = &options.interaction_expr {
                write!(
                    code,
                    ".interaction({})",
                    expr_code(interaction, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Canvas {
            options,
            locals,
            commands,
            events,
            ..
        } => render_canvas(options, locals, commands, events, document, message, env),
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
