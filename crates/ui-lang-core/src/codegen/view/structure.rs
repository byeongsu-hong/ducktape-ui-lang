use super::*;

pub(in crate::codegen) fn render_structure(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Theme {
            preset,
            text,
            background,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let mut code = format!(
                "{{ let __theme_content: __IceElement<'_, {message}> = {content}; ::iced::widget::themer({}, __theme_content)",
                theme_preset_code(preset, env, document)?
            );
            if let Some(color) = text {
                write!(code, ".text_color(|_| {})", theme_color(document, color)).unwrap();
            }
            if let Some(background) = background {
                write!(
                    code,
                    ".background(|_| {})",
                    background_code(background, env, document)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Float {
            scale,
            x,
            y,
            style,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let scale = expr_code(scale, env, document, ValueMode::Owned)?;
            let mut translate_env = env.clone();
            for (name, code) in [
                ("original_x", "(__original.x as f64)"),
                ("original_y", "(__original.y as f64)"),
                ("original_width", "(__original.width as f64)"),
                ("original_height", "(__original.height as f64)"),
                ("viewport_x", "(__viewport.x as f64)"),
                ("viewport_y", "(__viewport.y as f64)"),
                ("viewport_width", "(__viewport.width as f64)"),
                ("viewport_height", "(__viewport.height as f64)"),
            ] {
                translate_env.insert(
                    name.to_owned(),
                    Binding {
                        code: code.to_owned(),
                        ty: Type::F64,
                        local: true,
                        state: None,
                    },
                );
            }
            let x = expr_code(x, &translate_env, document, ValueMode::Owned)?;
            let y = expr_code(y, &translate_env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __float_content: __IceElement<'_, {message}> = {content}; let __float = ::iced::widget::float(__float_content).scale({scale} as f32).translate(move |__original, __viewport| ::iced::Vector::new({x} as f32, {y} as f32))"
            );
            append_float_style(&mut code, style, env, document)?;
            Ok(format!("{code}; __float.into() }}"))
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
                "{{ let __pin_content: __IceElement<'_, {message}> = {content}; ::iced::widget::pin(__pin_content).x({x} as f32).y({y} as f32)"
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
                "{{ let __sensor_content: __IceElement<'_, {message}> = {content}; ::iced::widget::sensor(__sensor_content)"
            );
            if let Some(route) = &options.show {
                let callback = ordered_route_callback_code(
                    route,
                    "__size",
                    &["__size.width as f64", "__size.height as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, ".on_show({callback})").unwrap();
            }
            if let Some(route) = &options.resize {
                let callback = ordered_route_callback_code(
                    route,
                    "__size",
                    &["__size.width as f64", "__size.height as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, ".on_resize({callback})").unwrap();
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
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = if __size.width < {breakpoint} as f32 {{ {narrow} }} else {{ {wide} }}; __responsive }}"
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
                            state: None,
                        },
                    );
                    child_env.insert(
                        height.clone(),
                        Binding {
                            code: "(__size.height as f64)".into(),
                            ty: Type::F64,
                            local: true,
                            state: None,
                        },
                    );
                    let content = render_node(content, document, message, &child_env, scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: __IceElement<'_, {message}> = {content}; __responsive }}"
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
                    state: None,
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
                "::iced::widget::lazy(({dependency}, ({scope}).to_owned()), move |__dependency| {{ let {binding}: {dependency_rust} = __dependency.0.clone(); let __lazy_scope = __dependency.1.clone(); let __lazy_content: __IceElement<'static, {message}> = {child}; __lazy_content }}).into()"
            ))
        }
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
