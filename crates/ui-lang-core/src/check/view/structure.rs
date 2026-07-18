use super::*;

pub(in crate::check) fn infer_structure_group(
    node: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::Theme {
            preset,
            text,
            background,
            content,
            span,
            ..
        } => {
            if let ThemePreset::Factory(factory) = preset {
                let function =
                    extern_function(document, &factory.function, ExternKind::Theme, span)?;
                check_call_args(function, &factory.args, env, document, span)?;
            }
            if let Some(color) = text
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E137",
                    span,
                    format!("unknown nested theme text color `{color}`"),
                ));
            }
            if let Some(background) = background {
                check_background_value(
                    background,
                    env,
                    document,
                    span,
                    "E137",
                    "nested theme background",
                )?;
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Float {
            scale,
            x,
            y,
            style,
            content,
            span,
        } => {
            require_type(&expr_type(scale, env, document, span)?, &Type::F64, span)?;
            let mut translate_env = env.clone();
            for name in [
                "original_x",
                "original_y",
                "original_width",
                "original_height",
                "viewport_x",
                "viewport_y",
                "viewport_width",
                "viewport_height",
            ] {
                translate_env.insert(name.to_owned(), Type::F64);
            }
            for value in [x, y] {
                require_type(
                    &expr_type(value, &translate_env, document, span)?,
                    &Type::F64,
                    span,
                )?;
            }
            require_literal_range(scale, f64::EPSILON, None, "float scale", span)?;
            check_float_style_options(style, env, document, span)?;
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Pin {
            width,
            height,
            x,
            y,
            content,
            span,
        } => {
            for value in [x, y] {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            }
            for length in [width, height].into_iter().flatten() {
                check_length_value(length, env, document, span, "pin size")?;
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Sensor {
            options,
            content,
            span,
        } => {
            for (route, label) in [(&options.show, "show"), (&options.resize, "resize")]
                .into_iter()
                .filter_map(|(route, label)| route.as_ref().map(|route| (route, label)))
            {
                if route.args.len() != 2
                    || route
                        .args
                        .iter()
                        .any(|arg| !matches!(arg, RouteArg::Payload))
                {
                    return Err(Error::new(
                        "E129",
                        span,
                        format!("sensor {label} route receives width and height"),
                    ));
                }
                infer_route(route, Some(Type::F64), env, document, signatures)?;
            }
            if let Some(route) = &options.hide {
                infer_route(route, None, env, document, signatures)?;
            }
            if let Some(key) = &options.key {
                let ty = expr_type(key, env, document, span)?;
                if !matches!(
                    ty,
                    Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
                ) {
                    return Err(Error::new(
                        "E129",
                        span,
                        "sensor key must be bool, i64, f64, str, or an extern type",
                    ));
                }
            }
            if let Some(distance) = &options.anticipate {
                require_type(&expr_type(distance, env, document, span)?, &Type::F64, span)?;
                require_literal_range(distance, 0.0, None, "sensor anticipation", span)?;
            }
            if let Some(delay) = &options.delay_ms {
                require_type(&expr_type(delay, env, document, span)?, &Type::I64, span)?;
                if matches!(delay, Expr::I64(value) if *value < 0) {
                    return Err(Error::new("E128", span, "sensor delay cannot be negative"));
                }
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Responsive {
            content,
            width,
            height,
            span,
        } => {
            for length in [width, height].into_iter().flatten() {
                check_length_value(length, env, document, span, "responsive size")?;
            }
            match content {
                ResponsiveContent::Breakpoint {
                    breakpoint,
                    narrow,
                    wide,
                } => {
                    require_type(
                        &expr_type(breakpoint, env, document, span)?,
                        &Type::F64,
                        span,
                    )?;
                    require_literal_range(
                        breakpoint,
                        f64::EPSILON,
                        None,
                        "responsive breakpoint",
                        span,
                    )?;
                    infer_view(narrow, env, document, signatures, ids)?;
                    infer_view(wide, env, document, signatures, ids)?;
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.insert(width.clone(), Type::F64);
                    child_env.insert(height.clone(), Type::F64);
                    infer_view(content, &child_env, document, signatures, ids)?;
                }
            }
        }
        _ => return Ok(false),
    };
    Ok(true)
}
