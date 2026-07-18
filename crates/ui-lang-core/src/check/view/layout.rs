use super::*;

pub(in crate::check) fn infer_layout_group(
    node: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::Layout {
            kind,
            options,
            id,
            styles,
            children,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            if let Some(columns) = &options.columns {
                require_type(&expr_type(columns, env, document, span)?, &Type::I64, span)?;
                if matches!(columns, Expr::I64(value) if *value <= 0) {
                    return Err(Error::new("E124", span, "grid columns must be positive"));
                }
            }
            if let Some(fluid) = &options.fluid {
                require_type(&expr_type(fluid, env, document, span)?, &Type::F64, span)?;
                require_literal_range(fluid, f64::EPSILON, None, "grid fluid width", span)?;
            }
            if let Some(height) = &options.grid_height {
                match height {
                    GridSizing::AspectRatio { width, height } => {
                        for (value, label) in
                            [(width, "grid aspect width"), (height, "grid aspect height")]
                        {
                            require_type(
                                &expr_type(value, env, document, span)?,
                                &Type::F64,
                                span,
                            )?;
                            require_literal_range(value, f64::EPSILON, None, label, span)?;
                        }
                    }
                    GridSizing::EvenlyDistribute(LengthValue::Fixed(value)) => {
                        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                        require_literal_range(value, 0.0, None, "grid height", span)?;
                    }
                    GridSizing::EvenlyDistribute(_) => {}
                }
            }
            if let Some(clip) = &options.clip {
                require_type(&expr_type(clip, env, document, span)?, &Type::Bool, span)?;
            }
            let layout_metric = match kind {
                Layout::Column => "column metric",
                Layout::Row => "row metric",
                Layout::Stack => "stack size",
                Layout::Scroll => "scroll metric",
                Layout::Grid => "grid metric",
            };
            for length in [&options.width, &options.height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, layout_metric, span)?;
                }
            }
            for value in [
                &options.spacing,
                &options.padding.all,
                &options.padding.x,
                &options.padding.y,
                &options.padding.top,
                &options.padding.right,
                &options.padding.bottom,
                &options.padding.left,
                &options.max_width,
                &options.wrap_spacing,
            ]
            .into_iter()
            .flatten()
            {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, layout_metric, span)?;
            }
            if let Some(scroll) = &options.scroll {
                for length in [&scroll.width, &scroll.height].into_iter().flatten() {
                    if let LengthValue::Fixed(value) = length {
                        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                        require_literal_range(value, 0.0, None, "scroll size", span)?;
                    }
                }
                for (value, label) in [
                    (&scroll.bar_width, "scroll bar width"),
                    (&scroll.bar_margin, "scroll bar margin"),
                    (&scroll.scroller_width, "scroll scroller width"),
                    (&scroll.bar_spacing, "scroll bar spacing"),
                ] {
                    if let Some(value) = value {
                        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                        require_literal_range(value, 0.0, None, label, span)?;
                    }
                }
                if let Some(auto_scroll) = &scroll.auto_scroll {
                    require_type(
                        &expr_type(auto_scroll, env, document, span)?,
                        &Type::Bool,
                        span,
                    )?;
                }
                if let Some(route) = &scroll.route {
                    infer_ordered_payload_route(
                        route,
                        &[Type::F64, Type::F64, Type::F64, Type::F64],
                        env,
                        document,
                        signatures,
                        "scroll viewport",
                    )?;
                }
                if let Some(route) = &scroll.viewport_route {
                    infer_ordered_payload_route(
                        route,
                        &[const { Type::F64 }; 14],
                        env,
                        document,
                        signatures,
                        "complete scroll viewport",
                    )?;
                }
                if let Some(style) = &scroll.custom_style {
                    let function =
                        extern_function(document, &style.function, ExternKind::ScrollStyle, span)?;
                    check_call_args(function, &style.args, env, document, span)?;
                }
                check_scroll_styles(&scroll.styles, env, document)?;
            }
            check_styles(styles, document, span, StyleTarget::Layout(*kind))?;
            for child in children {
                infer_view(child, env, document, signatures, ids)?;
            }
        }
        ViewNode::Container {
            options,
            id,
            styles,
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            for length in [&options.width, &options.height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "container size", span)?;
                }
            }
            for value in [
                &options.padding.all,
                &options.padding.x,
                &options.padding.y,
                &options.padding.top,
                &options.padding.right,
                &options.padding.bottom,
                &options.padding.left,
                &options.max_width,
                &options.max_height,
            ]
            .into_iter()
            .flatten()
            {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, "container metric", span)?;
            }
            if let Some(clip) = &options.clip {
                require_type(&expr_type(clip, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::ContainerStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_container_style_options(&options.style, env, document, span, "E184")?;
            check_styles(styles, document, span, StyleTarget::Container)?;
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Overlay {
            options,
            content,
            layer,
            span,
        } => {
            require_type(
                &expr_type(&options.visible, env, document, span)?,
                &Type::Bool,
                span,
            )?;
            require_type(
                &expr_type(&options.padding, env, document, span)?,
                &Type::F64,
                span,
            )?;
            require_literal_range(&options.padding, 0.0, None, "overlay padding", span)?;
            if !valid_theme_color(&options.backdrop, document) {
                return Err(Error::new(
                    "E185",
                    span,
                    format!("unknown overlay backdrop color `{}`", options.backdrop),
                ));
            }
            if let Some(dismiss) = &options.dismiss {
                infer_route(dismiss, None, env, document, signatures)?;
            }
            infer_view(content, env, document, signatures, ids)?;
            infer_view(layer, env, document, signatures, ids)?;
        }
        ViewNode::PaneGrid {
            name,
            options,
            panes,
            span,
            ..
        } => {
            if !ids.insert(name.clone()) {
                return Err(Error::new(
                    "E161",
                    span,
                    format!("duplicate local id `#{name}`"),
                ));
            }
            for length in [&options.width, &options.height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "pane-grid bounds", span)?;
                }
            }
            for (value, label) in [
                (&options.spacing, "pane-grid spacing"),
                (&options.min_size, "pane-grid minimum size"),
                (&options.resize_leeway, "pane-grid resize leeway"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
            if let Some(background) = &options.style.region_background {
                check_background_value(
                    background,
                    env,
                    document,
                    span,
                    "E187",
                    "pane-grid background",
                )?;
            }
            for color in [
                &options.style.region_border,
                &options.style.hovered_split,
                &options.style.picked_split,
            ]
            .into_iter()
            .flatten()
            {
                if !valid_theme_color(color, document) {
                    return Err(Error::new(
                        "E187",
                        span,
                        format!("unknown pane-grid style color `{color}`"),
                    ));
                }
            }
            for value in [
                &options.style.region_border_width,
                &options.style.region_radius,
                &options.style.region_radius_top_left,
                &options.style.region_radius_top_right,
                &options.style.region_radius_bottom_right,
                &options.style.region_radius_bottom_left,
                &options.style.hovered_split_width,
                &options.style.picked_split_width,
            ]
            .into_iter()
            .flatten()
            {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, "pane-grid style metric", span)?;
            }
            if let Some(click) = &options.click {
                infer_route(click, Some(Type::Str), env, document, signatures)?;
            }
            for pane in panes {
                check_styles(&pane.styles, document, &pane.span, StyleTarget::PaneContent)?;
                check_container_style_options(&pane.style, env, document, &pane.span, "E187")?;
                if let Some(title) = &pane.title {
                    for value in [
                        &title.padding.all,
                        &title.padding.x,
                        &title.padding.y,
                        &title.padding.top,
                        &title.padding.right,
                        &title.padding.bottom,
                        &title.padding.left,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        require_type(
                            &expr_type(value, env, document, &title.span)?,
                            &Type::F64,
                            &title.span,
                        )?;
                        require_literal_range(value, 0.0, None, "pane title padding", &title.span)?;
                    }
                    check_styles(&title.styles, document, &title.span, StyleTarget::PaneTitle)?;
                    check_container_style_options(
                        &title.style,
                        env,
                        document,
                        &title.span,
                        "E187",
                    )?;
                }
                for node in pane.nodes() {
                    infer_view(node, env, document, signatures, ids)?;
                }
            }
        }
        _ => return Ok(false),
    };
    Ok(true)
}
