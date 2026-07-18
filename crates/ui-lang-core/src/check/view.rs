use super::*;

pub(super) fn infer_view(
    node: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<(), Error> {
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
        ViewNode::Text {
            value,
            options,
            styles,
            span,
        } => {
            let ty = expr_type(value, env, document, span)?;
            if !matches!(ty, Type::Str | Type::I64 | Type::F64) {
                return Err(type_error(span, &Type::Str, &ty).hint("text accepts str, i64, or f64"));
            }
            check_text_options(options, env, document, span)?;
            check_styles(styles, document, span, StyleTarget::Text)?;
        }
        ViewNode::RichText {
            options,
            color,
            spans,
            styles,
            route,
            span,
        } => {
            check_text_options(options, env, document, span)?;
            check_styles(styles, document, span, StyleTarget::Text)?;
            if color
                .as_ref()
                .is_some_and(|color| !valid_theme_color(color, document))
            {
                return Err(Error::new("E186", span, "unknown rich-text color"));
            }
            let mut has_links = false;
            for item in spans {
                let ty = expr_type(&item.value, env, document, &item.span)?;
                if !matches!(ty, Type::Str | Type::I64 | Type::F64 | Type::Bool) {
                    return Err(Error::new(
                        "E186",
                        &item.span,
                        "span text must be str, i64, f64, or bool",
                    ));
                }
                check_font(item.options.font.as_ref(), document, &item.span)?;
                check_styles(&item.styles, document, &item.span, StyleTarget::Text)?;
                for color in [&item.options.color, &item.options.border]
                    .into_iter()
                    .flatten()
                {
                    if !valid_theme_color(color, document) {
                        return Err(Error::new(
                            "E186",
                            &item.span,
                            format!("unknown span color `{color}`"),
                        ));
                    }
                }
                if let Some(background) = &item.options.background {
                    check_background_value(
                        background,
                        env,
                        document,
                        &item.span,
                        "E186",
                        "span background",
                    )?;
                }
                for (value, label, min) in [
                    (item.options.size.as_ref(), "span size", f64::EPSILON),
                    (
                        item.options
                            .line_height
                            .as_ref()
                            .map(|height| match height {
                                TextLineHeight::Relative(value)
                                | TextLineHeight::Absolute(value) => value,
                            }),
                        "span line height",
                        f64::EPSILON,
                    ),
                    (item.options.border_width.as_ref(), "span border width", 0.0),
                    (item.options.radius.as_ref(), "span radius", 0.0),
                    (item.options.radius_top_left.as_ref(), "span radius", 0.0),
                    (item.options.radius_top_right.as_ref(), "span radius", 0.0),
                    (
                        item.options.radius_bottom_right.as_ref(),
                        "span radius",
                        0.0,
                    ),
                    (item.options.radius_bottom_left.as_ref(), "span radius", 0.0),
                    (item.options.padding.all.as_ref(), "span padding", 0.0),
                    (item.options.padding.x.as_ref(), "span padding", 0.0),
                    (item.options.padding.y.as_ref(), "span padding", 0.0),
                    (item.options.padding.top.as_ref(), "span padding", 0.0),
                    (item.options.padding.right.as_ref(), "span padding", 0.0),
                    (item.options.padding.bottom.as_ref(), "span padding", 0.0),
                    (item.options.padding.left.as_ref(), "span padding", 0.0),
                ] {
                    if let Some(value) = value {
                        require_type(
                            &expr_type(value, env, document, &item.span)?,
                            &Type::F64,
                            &item.span,
                        )?;
                        require_literal_range(value, min, None, label, &item.span)?;
                    }
                }
                for value in [&item.options.underline, &item.options.strikethrough]
                    .into_iter()
                    .flatten()
                {
                    require_type(
                        &expr_type(value, env, document, &item.span)?,
                        &Type::Bool,
                        &item.span,
                    )?;
                }
                if let Some(link) = &item.options.link {
                    has_links = true;
                    require_type(
                        &expr_type(link, env, document, &item.span)?,
                        &Type::Str,
                        &item.span,
                    )?;
                }
            }
            match (has_links, route) {
                (true, Some(route)) => {
                    infer_route(route, Some(Type::Str), env, document, signatures)?;
                }
                (true, None) => {
                    return Err(Error::new(
                        "E186",
                        span,
                        "rich-text spans with `link=` require `-> handler _`",
                    ));
                }
                (false, Some(_)) => {
                    return Err(Error::new(
                        "E186",
                        span,
                        "rich-text without linked spans cannot emit a route",
                    ));
                }
                (false, None) => {}
            }
        }
        ViewNode::Input {
            id,
            binding,
            disabled,
            options,
            styles,
            span,
            ..
        } => {
            check_id(id, env, document, ids, span)?;
            let Some(binding_ty) = env.get(binding) else {
                return Err(Error::new(
                    "E120",
                    span,
                    format!("unknown binding `{binding}`"),
                ));
            };
            require_type(binding_ty, &Type::Str, span)?;
            if let Some(disabled) = disabled {
                let ty = expr_type(disabled, env, document, span)?;
                require_type(&ty, &Type::Bool, span)?;
            }
            if let Some(secure) = &options.secure {
                require_type(&expr_type(secure, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(route) = &options.submit {
                infer_route(route, None, env, document, signatures)?;
            }
            if let Some(route) = &options.paste {
                infer_route(route, Some(Type::Str), env, document, signatures)?;
            }
            if let Some(length) = &options.width
                && let LengthValue::Fixed(value) = length
            {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, "input width", span)?;
            }
            for (value, label, min) in [
                (&options.padding, "input padding", 0.0),
                (&options.text_size, "input text size", f64::EPSILON),
                (&options.line_height, "input line height", f64::EPSILON),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, min, None, label, span)?;
                }
            }
            check_font(options.font.as_ref(), document, span)?;
            check_text_input_icon(options.icon.as_ref(), env, document, "input")?;
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::InputStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_text_input_styles(&options.style, env, document, span, "input")?;
            check_styles(styles, document, span, StyleTarget::Input)?;
        }
        ViewNode::Button {
            id,
            disabled,
            options,
            content,
            styles,
            route,
            span,
            ..
        } => {
            check_id(id, env, document, ids, span)?;
            if let Some(disabled) = disabled {
                let ty = expr_type(disabled, env, document, span)?;
                require_type(&ty, &Type::Bool, span)?;
            }
            for length in [&options.width, &options.height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "button size", span)?;
                }
            }
            if let Some(padding) = &options.padding {
                require_type(&expr_type(padding, env, document, span)?, &Type::F64, span)?;
                require_literal_range(padding, 0.0, None, "button padding", span)?;
            }
            if let Some(clip) = &options.clip {
                require_type(&expr_type(clip, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(style) = &options.style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::ButtonStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            for status in [
                &options.style.active,
                &options.style.hovered,
                &options.style.pressed,
                &options.style.disabled,
            ]
            .into_iter()
            .flatten()
            {
                check_container_style_options(
                    &status.options,
                    env,
                    document,
                    &status.span,
                    "E129",
                )?;
            }
            infer_route(route, None, env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Button)?;
            if let Some(content) = content {
                infer_view(content, env, document, signatures, ids)?;
            }
        }
        ViewNode::Checkbox {
            label,
            id,
            checked,
            disabled,
            options,
            style,
            styles,
            route,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            require_type(&expr_type(label, env, document, span)?, &Type::Str, span)?;
            require_type(&expr_type(checked, env, document, span)?, &Type::Bool, span)?;
            if let Some(disabled) = disabled {
                require_type(
                    &expr_type(disabled, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            check_bool_control_options(options, env, document, span)?;
            if let Some(style) = &style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::CheckboxStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_checkbox_styles(style, env, document, span)?;
            infer_route(route, Some(Type::Bool), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Checkbox)?;
        }
        ViewNode::Toggler {
            label,
            checked,
            disabled,
            options,
            style,
            styles,
            route,
            span,
        } => {
            require_type(&expr_type(label, env, document, span)?, &Type::Str, span)?;
            require_type(&expr_type(checked, env, document, span)?, &Type::Bool, span)?;
            if let Some(disabled) = disabled {
                require_type(
                    &expr_type(disabled, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            check_bool_control_options(options, env, document, span)?;
            if let Some(style) = &style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::TogglerStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_toggler_styles(style, env, document, span)?;
            infer_route(route, Some(Type::Bool), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Toggler)?;
        }
        ViewNode::Slider {
            value,
            min,
            max,
            step,
            options,
            vertical,
            styles,
            route,
            release,
            span,
            ..
        } => {
            let value_type = expr_type(value, env, document, span)?;
            if !matches!(&value_type, Type::F64 | Type::Named(_)) {
                return Err(Error::new(
                    "E125",
                    span,
                    "slider values must be f64 or an extern numeric type",
                ));
            }
            for expr in [min, max, step] {
                require_type(&expr_type(expr, env, document, span)?, &value_type, span)?;
            }
            for expr in [&options.default, &options.shift_step]
                .into_iter()
                .flatten()
            {
                require_type(&expr_type(expr, env, document, span)?, &value_type, span)?;
            }
            if value_type == Type::F64 {
                require_literal_range(step, f64::EPSILON, None, "slider step", span)?;
                if let Some(shift_step) = &options.shift_step {
                    require_literal_range(
                        shift_step,
                        f64::EPSILON,
                        None,
                        "slider shift step",
                        span,
                    )?;
                }
                if let (Some(min), Some(max)) = (f64_literal(min), f64_literal(max))
                    && min > max
                {
                    return Err(Error::new("E128", span, "slider min cannot exceed max"));
                }
                if let Some(default) = options.default.as_ref().and_then(f64_literal)
                    && (f64_literal(min).is_some_and(|min| default < min)
                        || f64_literal(max).is_some_and(|max| default > max))
                {
                    return Err(Error::new(
                        "E128",
                        span,
                        "slider default is outside its range",
                    ));
                }
            }
            for (length, fluid, label) in [
                (&options.width, !*vertical, "slider width"),
                (&options.height, *vertical, "slider height"),
            ] {
                if let Some(length) = length {
                    match length {
                        LengthValue::Fixed(value) => {
                            require_type(
                                &expr_type(value, env, document, span)?,
                                &Type::F64,
                                span,
                            )?;
                            require_literal_range(value, 0.0, None, label, span)?;
                        }
                        _ if !fluid => {
                            return Err(Error::new(
                                "E129",
                                span,
                                format!("{label} must be fixed on this axis"),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            if let Some(style) = &options.style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::SliderStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_slider_styles(&options.style, env, document, span)?;
            infer_route(route, Some(value_type), env, document, signatures)?;
            if let Some(release) = release {
                infer_route(release, None, env, document, signatures)?;
            }
            check_styles(styles, document, span, StyleTarget::Slider)?;
        }
        ViewNode::Progress {
            value,
            min,
            max,
            options,
            styles,
            span,
            ..
        } => {
            for expr in [value, min, max] {
                require_type(&expr_type(expr, env, document, span)?, &Type::F64, span)?;
            }
            if let (Some(min), Some(max)) = (f64_literal(min), f64_literal(max))
                && min > max
            {
                return Err(Error::new("E128", span, "progress min cannot exceed max"));
            }
            for length in [&options.length, &options.girth].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "progress size", span)?;
                }
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::ProgressStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            for (background, label) in [
                (&options.background, "progress background"),
                (&options.bar, "progress bar"),
            ] {
                if let Some(background) = background {
                    check_background_value(background, env, document, span, "E129", label)?;
                }
            }
            if let Some(color) = &options.border_color
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("unknown progress color `{color}`"),
                ));
            }
            for (value, label) in [
                (&options.border_width, "progress border width"),
                (&options.radius, "progress radius"),
                (&options.radius_top_left, "progress radius"),
                (&options.radius_top_right, "progress radius"),
                (&options.radius_bottom_right, "progress radius"),
                (&options.radius_bottom_left, "progress radius"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
            check_styles(styles, document, span, StyleTarget::Progress)?;
        }
        ViewNode::Radio {
            label,
            value,
            selected,
            options,
            style,
            styles,
            route,
            span,
        } => {
            require_type(&expr_type(label, env, document, span)?, &Type::Str, span)?;
            let value_type = expr_type(value, env, document, span)?;
            if !matches!(
                value_type,
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            ) {
                return Err(Error::new(
                    "E125",
                    span,
                    "radio values must be bool, i64, f64, str, or an extern type",
                ));
            }
            require_type(
                &expr_type(selected, env, document, span)?,
                &Type::Bool,
                span,
            )?;
            check_bool_control_options(options, env, document, span)?;
            if let Some(style) = &style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::RadioStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_radio_styles(style, env, document, span)?;
            infer_route(route, Some(value_type), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Radio)?;
        }
        ViewNode::PickList {
            options,
            selected,
            options_config,
            route,
            span,
        } => {
            let Type::List(option_type) = expr_type(options, env, document, span)? else {
                return Err(Error::new("E129", span, "pick options must be a list"));
            };
            let Type::Option(selected_type) = expr_type(selected, env, document, span)? else {
                return Err(Error::new(
                    "E129",
                    span,
                    "pick selection must use an optional `T?` value",
                ));
            };
            require_type(&option_type, &selected_type, span)?;
            if !matches!(
                option_type.as_ref(),
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            ) {
                return Err(Error::new(
                    "E129",
                    span,
                    "pick values must be bool, i64, f64, str, or an extern type",
                ));
            }
            if let Some(placeholder) = &options_config.placeholder {
                require_type(
                    &expr_type(placeholder, env, document, span)?,
                    &Type::Str,
                    span,
                )?;
            }
            for length in [&options_config.width, &options_config.menu_height]
                .into_iter()
                .flatten()
            {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "pick size", span)?;
                }
            }
            for (value, label) in [
                (&options_config.padding, "pick padding"),
                (&options_config.text_size, "pick text size"),
                (&options_config.line_height, "pick line height"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
            check_font(options_config.font.as_ref(), document, span)?;
            check_pick_list_handle(options_config.handle.as_ref(), env, document, span)?;
            if let Some(style) = &options_config.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::PickListStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            if let Some(style) = &options_config.custom_menu_style {
                let function =
                    extern_function(document, &style.function, ExternKind::MenuStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_pick_list_styles(options_config, env, document, span)?;
            infer_route(route, Some(*option_type), env, document, signatures)?;
            for route in [&options_config.open, &options_config.close]
                .into_iter()
                .flatten()
            {
                infer_route(route, None, env, document, signatures)?;
            }
        }
        ViewNode::ComboBox {
            state,
            selected,
            options,
            route,
            span,
            ..
        } => {
            let Some(Type::Combo(option_type)) = env.get(state) else {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("combo state `{state}` must have type `combo[T]`"),
                ));
            };
            let Type::Option(selected_type) = expr_type(selected, env, document, span)? else {
                return Err(Error::new(
                    "E129",
                    span,
                    "combo selection must use an optional `T?` value",
                ));
            };
            require_type(option_type, &selected_type, span)?;
            if !matches!(
                option_type.as_ref(),
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            ) {
                return Err(Error::new(
                    "E129",
                    span,
                    "combo values must be bool, i64, f64, str, or an extern type",
                ));
            }
            for length in [&options.width, &options.menu_height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "combo size", span)?;
                }
            }
            for (value, label) in [
                (&options.padding, "combo padding"),
                (&options.text_size, "combo text size"),
                (&options.line_height, "combo line height"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
            check_font(options.font.as_ref(), document, span)?;
            check_text_input_icon(options.icon.as_ref(), env, document, "combo")?;
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::InputStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            if let Some(style) = &options.custom_menu_style {
                let function =
                    extern_function(document, &style.function, ExternKind::MenuStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_text_input_styles(&options.style, env, document, span, "combo")?;
            check_menu_style(options.menu_style.as_deref(), env, document, span)?;
            for (route, payload, label) in [
                (Some(route), Some((**option_type).clone()), "selection"),
                (options.input.as_ref(), Some(Type::Str), "input"),
                (
                    options.hover.as_ref(),
                    Some((**option_type).clone()),
                    "hover",
                ),
            ] {
                if let Some(route) = route {
                    if route
                        .args
                        .iter()
                        .any(|arg| !matches!(arg, RouteArg::Payload))
                    {
                        return Err(Error::new(
                            "E129",
                            span,
                            format!("combo {label} routes only accept `_` payloads"),
                        ));
                    }
                    infer_route(route, payload, env, document, signatures)?;
                }
            }
            for route in [&options.open, &options.close].into_iter().flatten() {
                infer_route(route, None, env, document, signatures)?;
            }
        }
        ViewNode::Rule {
            thickness,
            options,
            styles,
            span,
            ..
        } => {
            require_type(
                &expr_type(thickness, env, document, span)?,
                &Type::F64,
                span,
            )?;
            require_literal_range(thickness, 0.0, None, "rule thickness", span)?;
            if let Some(RuleFill::Percent(percent)) = &options.fill {
                require_type(&expr_type(percent, env, document, span)?, &Type::F64, span)?;
                require_literal_range(percent, 0.0, Some(100.0), "rule percent", span)?;
            }
            if let Some(color) = &options.color
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("unknown rule color `{color}`"),
                ));
            }
            for radius in [
                &options.radius,
                &options.radius_top_left,
                &options.radius_top_right,
                &options.radius_bottom_right,
                &options.radius_bottom_left,
            ]
            .into_iter()
            .flatten()
            {
                require_type(&expr_type(radius, env, document, span)?, &Type::F64, span)?;
                require_literal_range(radius, 0.0, None, "rule radius", span)?;
            }
            if let Some(snap) = &options.snap {
                require_type(&expr_type(snap, env, document, span)?, &Type::Bool, span)?;
            }
            check_styles(styles, document, span, StyleTarget::Rule)?;
        }
        ViewNode::QrCode {
            data,
            cell_size,
            total_size,
            cell,
            background,
            span,
        } => {
            if !document.qr_codes.iter().any(|item| item.name == *data) {
                return Err(
                    Error::new("E136", span, format!("unknown qr data `{data}`"))
                        .hint(format!("declare `qr {data} \"...\"` before the view")),
                );
            }
            for (value, label) in [
                (cell_size.as_ref(), "qr cell size"),
                (total_size.as_ref(), "qr total size"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
            for (color, label) in [(cell, "cell"), (background, "background")] {
                if let Some(color) = color
                    && !valid_theme_color(color, document)
                {
                    return Err(Error::new(
                        "E136",
                        span,
                        format!("unknown qr {label} color `{color}`"),
                    ));
                }
            }
        }
        ViewNode::Space {
            width,
            height,
            styles,
            span,
        } => {
            for length in [width, height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "space length", span)?;
                }
            }
            check_styles(styles, document, span, StyleTarget::Space)?;
        }
        ViewNode::If {
            condition,
            children,
            span,
        } => {
            require_type(
                &expr_type(condition, env, document, span)?,
                &Type::Bool,
                span,
            )?;
            for child in children {
                infer_view(child, env, document, signatures, ids)?;
            }
        }
        ViewNode::For {
            item,
            items,
            children,
            span,
        } => {
            let Type::List(inner) = expr_type(items, env, document, span)? else {
                return Err(Error::new("E121", span, "for expects a list expression"));
            };
            let mut child_env = env.clone();
            child_env.insert(item.clone(), *inner);
            for child in children {
                infer_view(child, &child_env, document, signatures, ids)?;
            }
        }
        ViewNode::KeyedColumn {
            item,
            items,
            key,
            options,
            child,
            span,
        } => {
            let Type::List(inner) = expr_type(items, env, document, span)? else {
                return Err(Error::new("E138", span, "keyed expects a list expression"));
            };
            let mut child_env = env.clone();
            child_env.insert(item.clone(), *inner);
            let key_type = expr_type(key, &child_env, document, span)?;
            if !matches!(key_type, Type::Bool | Type::I64 | Type::F64) {
                return Err(Error::new(
                    "E138",
                    span,
                    "keyed keys must be copyable bool, i64, or f64 values",
                ));
            }
            for length in [&options.width, &options.height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "keyed size", span)?;
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
            ]
            .into_iter()
            .flatten()
            {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, "keyed metric", span)?;
            }
            infer_view(child, &child_env, document, signatures, ids)?;
        }
        ViewNode::Lazy {
            dependency,
            binding,
            child,
            span,
        } => {
            let dependency_type = expr_type(dependency, env, document, span)?;
            if !lazy_hashable(&dependency_type) {
                return Err(Error::new(
                    "E139",
                    span,
                    format!(
                        "lazy dependency type `{}` does not implement stable hashing",
                        dependency_type.display()
                    ),
                )
                .hint("use bool, i64, str, an extern type with Hash + Clone, or a list/optional of those"));
            }
            check_lazy_subtree(child, document, &mut HashSet::new(), false)?;
            let child_env = HashMap::from([(binding.clone(), dependency_type)]);
            let mut child_ids = HashSet::new();
            infer_view(child, &child_env, document, signatures, &mut child_ids)?;
        }
        ViewNode::Markdown {
            content,
            options,
            route,
            span,
        } => {
            let content_type = env.get(content).ok_or_else(|| {
                Error::new("E139", span, format!("unknown markdown state `{content}`"))
            })?;
            require_type(content_type, &Type::Markdown, span)?;
            for (value, label, min) in [
                (&options.text_size, "markdown text size", f64::EPSILON),
                (&options.h1_size, "markdown h1 size", f64::EPSILON),
                (&options.h2_size, "markdown h2 size", f64::EPSILON),
                (&options.h3_size, "markdown h3 size", f64::EPSILON),
                (&options.h4_size, "markdown h4 size", f64::EPSILON),
                (&options.h5_size, "markdown h5 size", f64::EPSILON),
                (&options.h6_size, "markdown h6 size", f64::EPSILON),
                (&options.code_size, "markdown code size", f64::EPSILON),
                (&options.spacing, "markdown spacing", 0.0),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, min, None, label, span)?;
                }
            }
            check_markdown_style(&options.style, env, document, span)?;
            let payload = if let Some(viewer) = &options.viewer {
                let function =
                    extern_function(document, &viewer.function, ExternKind::MarkdownViewer, span)?;
                check_call_args(function, &viewer.args, env, document, span)?;
                function.output.clone()
            } else {
                Type::Str
            };
            infer_route(route, Some(payload), env, document, signatures)?;
        }
        ViewNode::TextEditor {
            binding,
            id,
            disabled,
            options,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let binding_type = env.get(binding).ok_or_else(|| {
                Error::new("E139", span, format!("unknown editor state `{binding}`"))
            })?;
            require_type(binding_type, &Type::Editor, span)?;
            if let Some(disabled) = disabled {
                require_type(
                    &expr_type(disabled, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            for (value, label, min) in [
                (&options.width, "editor width", 0.0),
                (&options.min_height, "editor minimum height", 0.0),
                (&options.max_height, "editor maximum height", 0.0),
                (&options.size, "editor text size", f64::EPSILON),
                (&options.padding, "editor padding", 0.0),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, min, None, label, span)?;
                }
            }
            if let Some(LengthValue::Fixed(value)) = &options.height {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, "editor height", span)?;
            }
            if let Some(line_height) = &options.line_height {
                let value = match line_height {
                    TextLineHeight::Relative(value) | TextLineHeight::Absolute(value) => value,
                };
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, f64::EPSILON, None, "editor line height", span)?;
            }
            if let (Some(Expr::F64(min)), Some(Expr::F64(max))) =
                (&options.min_height, &options.max_height)
                && min > max
            {
                return Err(Error::new(
                    "E139",
                    span,
                    "editor min-height cannot exceed max-height",
                ));
            }
            check_font(options.font.as_ref(), document, span)?;
            if let Some(highlighter) = &options.highlighter {
                let function = extern_function(
                    document,
                    &highlighter.function,
                    ExternKind::EditorHighlighter,
                    span,
                )?;
                check_call_args(function, &highlighter.args, env, document, span)?;
            }
            if let Some(binding) = &options.key_binding {
                let function =
                    extern_function(document, &binding.function, ExternKind::EditorBinding, span)?;
                check_call_args(function, &binding.args, env, document, span)?;
                infer_route(
                    options
                        .key_binding_route
                        .as_ref()
                        .expect("parser requires a key-binding route"),
                    Some(function.output.clone()),
                    env,
                    document,
                    signatures,
                )?;
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::EditorStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_text_input_styles(&options.style, env, document, span, "editor")?;
        }
        ViewNode::Table {
            item,
            rows,
            options,
            columns,
            span,
        } => {
            let Type::List(inner) = expr_type(rows, env, document, span)? else {
                return Err(Error::new("E139", span, "table expects a list of rows"));
            };
            if let Some(LengthValue::Fixed(value)) = &options.width {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, "table width", span)?;
            }
            for (value, label) in [
                (&options.padding, "table padding"),
                (&options.padding_x, "table horizontal padding"),
                (&options.padding_y, "table vertical padding"),
                (&options.separator, "table separator"),
                (&options.separator_x, "table horizontal separator"),
                (&options.separator_y, "table vertical separator"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
            let mut cell_env = env.clone();
            cell_env.insert(item.clone(), *inner);
            for column in columns {
                if let Some(LengthValue::Fixed(value)) = &column.width {
                    require_type(
                        &expr_type(value, env, document, &column.span)?,
                        &Type::F64,
                        &column.span,
                    )?;
                    require_literal_range(value, 0.0, None, "table column width", &column.span)?;
                }
                let mut header_ids = HashSet::new();
                infer_view(&column.header, env, document, signatures, &mut header_ids)?;
                let mut cell_ids = HashSet::new();
                infer_view(&column.cell, &cell_env, document, signatures, &mut cell_ids)?;
            }
        }
        ViewNode::Component {
            name,
            args,
            id,
            slots: supplied_slots,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            if args.iter().any(|arg| arg.name.is_some()) {
                let mut supplied = HashSet::new();
                for arg in args {
                    let prop = arg.name.as_ref().expect("named component call");
                    let Some((_, expected)) =
                        component.params.iter().find(|(param, _)| param == prop)
                    else {
                        return Err(Error::new(
                            "E123",
                            span,
                            format!("component `{name}` has no prop `{prop}`"),
                        ));
                    };
                    if !supplied.insert(prop) {
                        return Err(Error::new(
                            "E123",
                            span,
                            format!("component `{name}` receives prop `{prop}` more than once"),
                        ));
                    }
                    let actual = expr_type(&arg.value, env, document, span)?;
                    require_type(&actual, expected, span)?;
                }
                if let Some((missing, _)) = component
                    .params
                    .iter()
                    .find(|(param, _)| !supplied.contains(param))
                {
                    return Err(Error::new(
                        "E123",
                        span,
                        format!("component `{name}` is missing prop `{missing}`"),
                    ));
                }
            } else {
                if args.len() != component.params.len() {
                    return Err(Error::new(
                        "E123",
                        span,
                        format!(
                            "component `{name}` expects {} arguments, got {}",
                            component.params.len(),
                            args.len()
                        ),
                    ));
                }
                for (arg, (_, expected)) in args.iter().zip(&component.params) {
                    let actual = expr_type(&arg.value, env, document, span)?;
                    require_type(&actual, expected, span)?;
                }
            }
            let declared_slots = slots(&component.root);
            let mut supplied = HashSet::new();
            for component_slot in supplied_slots {
                if !supplied.insert(component_slot.name.as_str()) {
                    return Err(Error::new(
                        "E124",
                        &component_slot.span,
                        format!(
                            "component `{name}` receives slot `{}` more than once",
                            component_slot.name
                        ),
                    ));
                }
                if !declared_slots
                    .iter()
                    .any(|(declared, _)| *declared == component_slot.name)
                {
                    return Err(Error::new(
                        "E124",
                        &component_slot.span,
                        format!(
                            "component `{name}` does not declare slot `{}`",
                            component_slot.name
                        ),
                    )
                    .hint(format!(
                        "add `slot {}` inside the component definition",
                        component_slot.name
                    )));
                }
                let mut child_ids = HashSet::new();
                infer_view(
                    &component_slot.content,
                    env,
                    document,
                    signatures,
                    &mut child_ids,
                )?;
            }
            if let Some((missing, _)) = declared_slots
                .iter()
                .find(|(declared, _)| !supplied.contains(*declared))
            {
                return Err(Error::new(
                    "E124",
                    span,
                    format!("component `{name}` requires slot `{missing}`"),
                ));
            }
        }
        ViewNode::Slot { .. } => {}
        ViewNode::ExternComponent {
            function,
            args,
            route,
            span,
        } => {
            let component = extern_function(document, function, ExternKind::Component, span)?;
            check_call_args(component, args, env, document, span)?;
            match (&component.output, route) {
                (Type::Unit, None) => {}
                (_, Some(route)) => infer_route(
                    route,
                    Some(component.output.clone()),
                    env,
                    document,
                    signatures,
                )?,
                (_, None) => {
                    return Err(Error::new(
                        "E126",
                        span,
                        format!(
                            "extern component `{function}` emits `{}` and requires a route",
                            component.output.display()
                        ),
                    ));
                }
            }
        }
        ViewNode::Themer {
            function,
            args,
            route,
            span,
        } => {
            let themer = extern_function(document, function, ExternKind::Themer, span)?;
            check_call_args(themer, args, env, document, span)?;
            match (&themer.output, route) {
                (Type::Unit, None) => {}
                (_, Some(route)) => infer_route(
                    route,
                    Some(themer.output.clone()),
                    env,
                    document,
                    signatures,
                )?,
                (_, None) => {
                    return Err(Error::new(
                        "E126",
                        span,
                        format!(
                            "themer `{function}` emits `{}` and requires a route",
                            themer.output.display()
                        ),
                    ));
                }
            }
        }
        ViewNode::Shader {
            function,
            args,
            width,
            height,
            route,
            span,
        } => {
            let shader = extern_function(document, function, ExternKind::Shader, span)?;
            check_call_args(shader, args, env, document, span)?;
            for length in [width, height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "shader size", span)?;
                }
            }
            match (&shader.output, route) {
                (Type::Unit, None) => {}
                (_, Some(route)) => infer_route(
                    route,
                    Some(shader.output.clone()),
                    env,
                    document,
                    signatures,
                )?,
                (_, None) => {
                    return Err(Error::new(
                        "E191",
                        span,
                        format!(
                            "shader `{function}` emits `{}` and requires a route",
                            shader.output.display()
                        ),
                    ));
                }
            }
        }
        ViewNode::Media {
            kind,
            source,
            options,
            span,
        } => {
            let source_ty = expr_type(source, env, document, span)?;
            let valid_source = match kind {
                MediaKind::Image | MediaKind::Viewer => {
                    source_ty == Type::Str || source_ty == Type::Image
                }
                MediaKind::Svg if options.svg_memory => {
                    source_ty == Type::Str || source_ty == Type::Bytes
                }
                MediaKind::Svg => source_ty == Type::Str,
            };
            if !valid_source {
                let error = type_error(
                    span,
                    if matches!(kind, MediaKind::Image | MediaKind::Viewer) {
                        &Type::Image
                    } else if options.svg_memory {
                        &Type::Bytes
                    } else {
                        &Type::Str
                    },
                    &source_ty,
                );
                return Err(if matches!(kind, MediaKind::Image | MediaKind::Viewer) {
                    error.hint("image and viewer accept a path string or image handle")
                } else if options.svg_memory {
                    error.hint("SVG memory accepts UTF-8 text or raw bytes")
                } else {
                    error
                });
            }
            for length in [&options.width, &options.height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "media size", span)?;
                }
            }
            for (value, label, min, max) in [
                (&options.rotation, "rotation", None, None),
                (&options.opacity, "opacity", Some(0.0), Some(1.0)),
                (&options.scale, "scale", Some(f64::EPSILON), None),
                (&options.radius, "radius", Some(0.0), None),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(
                        value,
                        min.unwrap_or(f64::NEG_INFINITY),
                        max,
                        label,
                        span,
                    )?;
                }
            }
            for value in [
                &options.radius_top_left,
                &options.radius_top_right,
                &options.radius_bottom_right,
                &options.radius_bottom_left,
            ]
            .into_iter()
            .flatten()
            {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, "radius", span)?;
            }
            if let Some(expand) = &options.expand {
                require_type(&expr_type(expand, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(crop) = &options.crop {
                for value in crop {
                    require_type(&expr_type(value, env, document, span)?, &Type::I64, span)?;
                    require_literal_range(
                        value,
                        0.0,
                        Some(u32::MAX as f64),
                        "image crop coordinate",
                        span,
                    )?;
                }
            }
            for (value, label, min) in [
                (&options.padding, "viewer padding", 0.0),
                (&options.min_scale, "viewer minimum scale", f64::EPSILON),
                (&options.max_scale, "viewer maximum scale", f64::EPSILON),
                (&options.scale_step, "viewer scale step", f64::EPSILON),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, min, None, label, span)?;
                }
            }
            let min_scale = options.min_scale.as_ref().map_or(Some(0.25), f64_literal);
            let max_scale = options.max_scale.as_ref().map_or(Some(10.0), f64_literal);
            if matches!((min_scale, max_scale), (Some(min), Some(max)) if min > max) {
                return Err(Error::new(
                    "E128",
                    span,
                    "viewer minimum scale cannot exceed maximum scale",
                ));
            }
            for color in options
                .svg_color
                .iter()
                .chain(options.svg_hover_color.iter().flatten())
            {
                if !valid_theme_color(color, document) {
                    return Err(Error::new(
                        "E129",
                        span,
                        format!("unknown svg color `{color}`"),
                    ));
                }
            }
            if let Some(style) = &options.svg_style {
                let function =
                    extern_function(document, &style.function, ExternKind::SvgStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
        }
        ViewNode::Tooltip {
            options,
            content,
            tip,
            span,
        } => {
            for (value, label) in [
                (&options.gap, "tooltip gap"),
                (&options.padding, "tooltip padding"),
            ] {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, label, span)?;
            }
            require_type(
                &expr_type(&options.delay_ms, env, document, span)?,
                &Type::I64,
                span,
            )?;
            if matches!(&options.delay_ms, Expr::I64(value) if *value < 0) {
                return Err(Error::new("E128", span, "tooltip delay cannot be negative"));
            }
            require_type(
                &expr_type(&options.snap, env, document, span)?,
                &Type::Bool,
                span,
            )?;
            if let Some(background) = &options.background {
                check_background_value(
                    background,
                    env,
                    document,
                    span,
                    "E129",
                    "tooltip background",
                )?;
            }
            for color in [
                &options.text_color,
                &options.border_color,
                &options.shadow_color,
            ]
            .into_iter()
            .flatten()
            {
                if !valid_theme_color(color, document) {
                    return Err(Error::new(
                        "E129",
                        span,
                        format!("unknown tooltip color `{color}`"),
                    ));
                }
            }
            for (value, label) in [
                (&options.border_width, "tooltip border width"),
                (&options.radius, "tooltip radius"),
                (&options.radius_top_left, "tooltip radius"),
                (&options.radius_top_right, "tooltip radius"),
                (&options.radius_bottom_right, "tooltip radius"),
                (&options.radius_bottom_left, "tooltip radius"),
                (&options.shadow_blur, "tooltip shadow blur"),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
            for value in [&options.shadow_x, &options.shadow_y].into_iter().flatten() {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            }
            if let Some(pixel_snap) = &options.pixel_snap {
                require_type(
                    &expr_type(pixel_snap, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::ContainerStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            infer_view(content, env, document, signatures, ids)?;
            infer_view(tip, env, document, signatures, ids)?;
        }
        ViewNode::MouseArea {
            options, content, ..
        } => {
            for route in [
                &options.press,
                &options.release,
                &options.double_click,
                &options.right_press,
                &options.right_release,
                &options.middle_press,
                &options.middle_release,
                &options.enter,
                &options.exit,
            ]
            .into_iter()
            .flatten()
            {
                infer_route(route, None, env, document, signatures)?;
            }
            if let Some(route) = &options.move_route {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    env,
                    document,
                    signatures,
                    "mouse move",
                )?;
            }
            if let Some(route) = &options.scroll {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64, Type::Bool],
                    env,
                    document,
                    signatures,
                    "mouse scroll",
                )?;
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Canvas {
            options,
            locals,
            commands,
            events,
            span,
        } => {
            for length in [&options.width, &options.height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "canvas size", span)?;
                }
            }
            if let Some(dependency) = &options.cache {
                let ty = expr_type(dependency, env, document, span)?;
                if !lazy_hashable(&ty) {
                    return Err(Error::new(
                        "E190",
                        span,
                        format!(
                            "canvas cache dependency type `{}` does not implement stable hashing",
                            ty.display()
                        ),
                    )
                    .hint("use bool, i64, str, bytes, an extern Hash + Clone type, or a list/optional of those"));
                }
            }
            if options.cache_group.is_some() && options.cache.is_none() {
                return Err(Error::new(
                    "E190",
                    span,
                    "canvas cache-group requires `cache=`",
                ));
            }
            if let Some(capture) = &options.capture {
                require_type(&expr_type(capture, env, document, span)?, &Type::Bool, span)?;
            }
            for route in [&options.enter, &options.exit].into_iter().flatten() {
                infer_route(route, None, env, document, signatures)?;
            }
            for route in [
                &options.press,
                &options.release,
                &options.right_press,
                &options.right_release,
                &options.middle_press,
                &options.middle_release,
                &options.move_route,
            ]
            .into_iter()
            .flatten()
            {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    env,
                    document,
                    signatures,
                    "canvas pointer event",
                )?;
            }
            if let Some(route) = &options.scroll {
                infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64, Type::Bool],
                    env,
                    document,
                    signatures,
                    "canvas scroll",
                )?;
            }
            let known = document
                .structs
                .iter()
                .map(|item| item.name.as_str())
                .collect::<HashSet<_>>();
            let mut canvas_env = env.clone();
            let mut local_types = HashMap::new();
            for local in locals {
                if matches!(
                    local.name.as_str(),
                    "cache" | "cache_key" | "inside" | "canvas_width" | "canvas_height"
                ) {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        format!("canvas state name `{}` is reserved", local.name),
                    ));
                }
                if env.contains_key(&local.name) {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        format!(
                            "canvas state `{}` conflicts with an app state or component parameter",
                            local.name
                        ),
                    ));
                }
                if local_types
                    .insert(local.name.clone(), local.ty.clone())
                    .is_some()
                {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        format!("duplicate canvas state `{}`", local.name),
                    ));
                }
                check_declared_type(&local.ty, &local.span, &known)?;
                let actual = expr_type(&local.initial, &HashMap::new(), document, &local.span)?;
                if let Type::Combo(expected) = &local.ty {
                    let Type::List(actual) = actual else {
                        return Err(Error::new(
                            "E104",
                            &local.span,
                            "combo canvas state must be initialized with a list",
                        ));
                    };
                    require_type(&actual, expected, &local.span)?;
                } else {
                    let text_initial =
                        matches!(local.ty, Type::Markdown | Type::Editor) && actual == Type::Str;
                    if actual != Type::Unknown && !text_initial && !compatible(&local.ty, &actual) {
                        return Err(type_error(&local.span, &local.ty, &actual));
                    }
                }
                canvas_env.insert(local.name.clone(), local.ty.clone());
            }
            canvas_env.insert("canvas_width".into(), Type::F64);
            canvas_env.insert("canvas_height".into(), Type::F64);
            if let Some(interaction) = &options.interaction_expr {
                require_type(
                    &expr_type(interaction, &canvas_env, document, span)?,
                    &Type::Str,
                    span,
                )?;
                if let Expr::Str(value) = interaction
                    && !valid_canvas_cursor(value)
                {
                    return Err(Error::new(
                        "E190",
                        span,
                        format!("unknown canvas cursor `{value}`"),
                    ));
                }
            }
            if let Some(outside) = &options.interaction_outside {
                if options.interaction.is_none() && options.interaction_expr.is_none() {
                    return Err(Error::new(
                        "E190",
                        span,
                        "canvas cursor-outside requires `cursor=`",
                    ));
                }
                require_type(
                    &expr_type(outside, &canvas_env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            check_canvas_commands(commands, &canvas_env, document)?;
            let mut seen = HashSet::new();
            for event in events {
                let name = canvas_event_name(&event.source).ok_or_else(|| {
                    Error::new("E190", &event.span, "invalid canvas event source")
                })?;
                if !seen.insert(name) {
                    return Err(Error::new(
                        "E190",
                        &event.span,
                        format!("duplicate canvas event `{name}`"),
                    ));
                }
                let payloads =
                    native_subscription_payloads(&event.source, false).ok_or_else(|| {
                        Error::new("E190", &event.span, "invalid canvas event source")
                    })?;
                if !event.route_payload
                    && !event.bindings.is_empty()
                    && event.bindings.len() != payloads.len()
                {
                    return Err(Error::new(
                        "E190",
                        &event.span,
                        format!(
                            "canvas event `{name}` exposes {} values, but {} bindings were declared",
                            payloads.len(),
                            event.bindings.len()
                        ),
                    ));
                }
                let mut event_env = canvas_env.clone();
                for (binding, ty) in event.bindings.iter().zip(&payloads) {
                    if event_env.contains_key(binding) {
                        return Err(Error::new(
                            "E190",
                            &event.span,
                            format!(
                                "canvas event binding `{binding}` conflicts with existing state"
                            ),
                        ));
                    }
                    event_env.insert(binding.clone(), ty.clone());
                }
                for update in &event.updates {
                    let expected = local_types.get(&update.name).ok_or_else(|| {
                        Error::new(
                            "E190",
                            &update.span,
                            format!("unknown canvas state `{}`", update.name),
                        )
                    })?;
                    let actual = expr_type(&update.value, &event_env, document, &update.span)?;
                    require_type(&actual, expected, &update.span)?;
                }
                if event.updates.is_empty() && event.action.is_none() && !event.capture {
                    return Err(Error::new(
                        "E190",
                        &event.span,
                        "canvas event block has no effect",
                    ));
                };
                let Some(CanvasEventAction::Route(route)) = &event.action else {
                    continue;
                };
                if event.route_payload {
                    infer_ordered_payload_route(
                        route,
                        &payloads,
                        env,
                        document,
                        signatures,
                        "canvas event",
                    )?;
                } else {
                    if route
                        .args
                        .iter()
                        .any(|arg| matches!(arg, RouteArg::Payload))
                    {
                        return Err(Error::new(
                            "E190",
                            &event.span,
                            "canvas event block `emit` uses named bindings instead of `_`",
                        ));
                    }
                    infer_route(route, None, &event_env, document, signatures)?;
                }
            }
        }
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
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "pin size", span)?;
                }
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
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "responsive size", span)?;
                }
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
    }
    Ok(())
}

pub(super) fn lazy_hashable(ty: &Type) -> bool {
    match ty {
        Type::Bool
        | Type::I64
        | Type::Str
        | Type::Bytes
        | Type::Instant
        | Type::WindowId
        | Type::WidgetId
        | Type::Key
        | Type::PhysicalKey
        | Type::KeyModifiers
        | Type::MouseButton
        | Type::TouchFinger
        | Type::Named(_) => true,
        Type::List(inner) | Type::Option(inner) => lazy_hashable(inner),
        Type::Result(output, error) => lazy_hashable(output) && lazy_hashable(error),
        Type::F64
        | Type::Combo(_)
        | Type::Markdown
        | Type::Editor
        | Type::Event
        | Type::KeyLocation
        | Type::KeyPress
        | Type::KeyRelease
        | Type::Pixels
        | Type::Padding
        | Type::Degrees
        | Type::Radians
        | Type::Point
        | Type::PointU32
        | Type::Vector
        | Type::Size
        | Type::Rectangle
        | Type::RectangleU32
        | Type::Transformation
        | Type::MouseCursor
        | Type::MouseClick
        | Type::SystemInfo
        | Type::WidgetTarget
        | Type::TaskHandle
        | Type::Image
        | Type::Unit
        | Type::Unknown => false,
    }
}
