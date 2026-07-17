use crate::Error;
use crate::ast::*;
use std::collections::{HashMap, HashSet};

pub fn check(document: &mut Document) -> Result<(), Error> {
    check_unique(document)?;
    check_fonts(document)?;
    check_slots(document)?;
    check_declared_types(document)?;
    check_theme(document)?;
    check_qr_data(document)?;

    let states: HashMap<String, Type> = document
        .states
        .iter()
        .map(|state| (state.name.clone(), state.ty.clone()))
        .collect();
    for state in &document.states {
        let actual = expr_type(&state.initial, &HashMap::new(), document, &state.span)?;
        if let Type::Combo(expected) = &state.ty {
            let Type::List(actual) = actual else {
                return Err(Error::new(
                    "E104",
                    &state.span,
                    "combo state must be initialized with a list",
                ));
            };
            require_type(&actual, expected, &state.span)?;
        } else {
            let text_initial =
                matches!(state.ty, Type::Markdown | Type::Editor) && actual == Type::Str;
            if actual != Type::Unknown && !text_initial && !compatible(&state.ty, &actual) {
                return Err(type_error(&state.span, &state.ty, &actual));
            }
        }
    }

    let mut signatures: HashMap<String, Vec<Option<Type>>> = document
        .handlers
        .iter()
        .map(|handler| (handler.name.clone(), vec![None; handler.params.len()]))
        .collect();

    let mut ids = HashSet::new();
    infer_view(&document.view, &states, document, &mut signatures, &mut ids)?;
    let operation_ids = static_widget_ids(&document.view);
    for component in &document.components {
        if let Some(span) = editor_span(&component.root) {
            return Err(
                Error::new("E139", span, "editor cannot bind a component parameter")
                    .hint("pass the editor through the component slot from the app view"),
            );
        }
        let env = component.params.iter().cloned().collect();
        let mut ids = HashSet::new();
        infer_view(&component.root, &env, document, &mut signatures, &mut ids)?;
    }
    infer_subscriptions(document, &states, &mut signatures)?;
    for handler in &document.handlers {
        infer_runs(handler, document, &mut signatures)?;
    }

    for handler in &mut document.handlers {
        let inferred = signatures.get(&handler.name).expect("handler signature");
        for (param, inferred) in handler.params.iter_mut().zip(inferred) {
            param.ty = inferred.clone().ok_or_else(|| {
                Error::new(
                    "E102",
                    &handler.span,
                    format!(
                        "cannot infer type of `{}` in handler `{}`",
                        param.name, handler.name
                    ),
                )
                .hint("route a typed widget or action payload to this parameter")
            })?;
        }
    }

    for handler in &document.handlers {
        check_handler(handler, &states, document, &operation_ids)?;
    }
    Ok(())
}

fn static_widget_ids(root: &ViewNode) -> HashSet<String> {
    fn insert(id: &Option<Id>, output: &mut HashSet<String>) {
        if let Some(Id { name, key: None }) = id {
            output.insert(name.clone());
        }
    }
    fn collect(node: &ViewNode, output: &mut HashSet<String>) {
        match node {
            ViewNode::Layout { id, children, .. } => {
                insert(id, output);
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Input { id, .. }
            | ViewNode::Checkbox { id, .. }
            | ViewNode::TextEditor { id, .. } => insert(id, output),
            ViewNode::Button { id, content, .. } => {
                insert(id, output);
                if let Some(content) = content {
                    collect(content, output);
                }
            }
            ViewNode::If { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = HashSet::new();
    collect(root, &mut output);
    output
}

fn check_declared_types(document: &Document) -> Result<(), Error> {
    let known = document
        .structs
        .iter()
        .map(|item| item.name.as_str())
        .collect::<HashSet<_>>();
    let check = |ty: &Type, span: &Span| check_declared_type(ty, span, &known);

    for item in &document.structs {
        for (_, ty) in &item.fields {
            check(ty, &item.span)?;
        }
    }
    for item in &document.functions {
        for (_, ty) in &item.params {
            check(ty, &item.span)?;
        }
        check(&item.output, &item.span)?;
        if let Some(error) = &item.error {
            check(error, &item.span)?;
        }
    }
    for state in &document.states {
        check(&state.ty, &state.span)?;
    }
    for component in &document.components {
        for (_, ty) in &component.params {
            check(ty, &component.span)?;
        }
    }
    Ok(())
}

fn check_declared_type(ty: &Type, span: &Span, known: &HashSet<&str>) -> Result<(), Error> {
    match ty {
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => {
            check_declared_type(inner, span, known)
        }
        Type::Named(name) if !known.contains(name.as_str()) => {
            Err(
                Error::new("E103", span, format!("unknown extern type `{name}`")).hint(format!(
                    "declare `{name}(...)` inside the extern block before using it"
                )),
            )
        }
        _ => Ok(()),
    }
}

fn check_unique(document: &Document) -> Result<(), Error> {
    let mut names = HashSet::new();
    for item in &document.structs {
        if !names.insert(("struct", item.name.as_str())) {
            return Err(Error::new(
                "E100",
                &item.span,
                format!("duplicate struct `{}`", item.name),
            ));
        }
        let mut fields = HashSet::new();
        for (field, _) in &item.fields {
            if !fields.insert(field) {
                return Err(Error::new(
                    "E100",
                    &item.span,
                    format!("duplicate field `{field}`"),
                ));
            }
        }
    }
    for item in &document.functions {
        if !names.insert(("fn", item.name.as_str())) {
            return Err(Error::new(
                "E100",
                &item.span,
                format!("duplicate function `{}`", item.name),
            ));
        }
    }
    let mut fields = HashSet::new();
    for qr in &document.qr_codes {
        if !fields.insert(&qr.name) {
            return Err(Error::new(
                "E100",
                &qr.span,
                format!("duplicate qr data `{}`", qr.name),
            ));
        }
    }
    for state in &document.states {
        if !fields.insert(&state.name) {
            return Err(Error::new(
                "E100",
                &state.span,
                format!("duplicate app field `{}`", state.name),
            ));
        }
    }
    let mut handlers = HashSet::new();
    for handler in &document.handlers {
        if !handlers.insert(&handler.name) {
            return Err(Error::new(
                "E100",
                &handler.span,
                format!("duplicate handler `{}`", handler.name),
            ));
        }
    }
    let mut components = HashSet::new();
    for component in &document.components {
        if !components.insert(&component.name) {
            return Err(Error::new(
                "E100",
                &component.span,
                format!("duplicate component `{}`", component.name),
            ));
        }
    }
    Ok(())
}

fn check_fonts(document: &Document) -> Result<(), Error> {
    let mut names = HashSet::new();
    let mut default = None;
    for font in &document.fonts {
        if !names.insert(&font.name) {
            return Err(Error::new(
                "E100",
                &font.span,
                format!("duplicate font `{}`", font.name),
            ));
        }
        if font.default && default.replace(&font.name).is_some() {
            return Err(Error::new(
                "E114",
                &font.span,
                "only one font may be default",
            ));
        }
    }
    Ok(())
}

fn check_font(font: Option<&FontPreset>, document: &Document, span: &Span) -> Result<(), Error> {
    if let Some(FontPreset::Named(name)) = font
        && !document.fonts.iter().any(|font| font.name == *name)
    {
        return Err(Error::new("E114", span, format!("unknown font `{name}`"))
            .hint(format!("declare `font {name} ...` before using it")));
    }
    Ok(())
}

fn check_slots(document: &Document) -> Result<(), Error> {
    let view_slots = slots(&document.view);
    if let Some(span) = view_slots.first() {
        return Err(Error::new(
            "E124",
            span,
            "slot is only valid inside a component definition",
        ));
    }
    for component in &document.components {
        if slots(&component.root).len() > 1 {
            return Err(Error::new(
                "E124",
                &component.span,
                format!("component `{}` may contain only one slot", component.name),
            ));
        }
    }
    Ok(())
}

fn slot_count(node: &ViewNode) -> usize {
    slots(node).len()
}

fn slots(node: &ViewNode) -> Vec<&Span> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<&'a Span>) {
        match node {
            ViewNode::Slot { span } => output.push(span),
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            }
            | ViewNode::MouseArea { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::Component {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }

    let mut output = Vec::new();
    collect(node, &mut output);
    output
}

fn editor_span(node: &ViewNode) -> Option<&Span> {
    match node {
        ViewNode::TextEditor { span, .. } => Some(span),
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => children.iter().find_map(editor_span),
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => editor_span(content),
        ViewNode::Tooltip { content, tip, .. } => editor_span(content).or_else(|| editor_span(tip)),
        ViewNode::Table { columns, .. } => columns
            .iter()
            .find_map(|column| editor_span(&column.header).or_else(|| editor_span(&column.cell))),
        ViewNode::Component {
            content: Some(content),
            ..
        } => editor_span(content),
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                editor_span(narrow).or_else(|| editor_span(wide))
            }
            ResponsiveContent::Size { content, .. } => editor_span(content),
        },
        _ => None,
    }
}

fn check_qr_data(document: &Document) -> Result<(), Error> {
    for qr in &document.qr_codes {
        let valid = match qr.version {
            None | Some(QrVersion::Normal(1..=40)) | Some(QrVersion::Micro(1..=4)) => true,
            Some(QrVersion::Normal(_) | QrVersion::Micro(_)) => false,
        };
        if !valid {
            return Err(Error::new(
                "E136",
                &qr.span,
                "qr version must be normal(1..40) or micro(1..4)",
            ));
        }
    }
    Ok(())
}

fn check_theme(document: &Document) -> Result<(), Error> {
    for required in ["background", "foreground", "primary", "danger"] {
        if !document.theme.contains_key(required) {
            return Err(Error::new(
                "E110",
                &Span::line(1),
                format!("theme is missing `{required}`"),
            ));
        }
    }
    Ok(())
}

fn infer_view(
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
            }
            check_styles(styles, document, span, StyleTarget::Layout(*kind))?;
            for child in children {
                infer_view(child, env, document, signatures, ids)?;
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
                (&options.icon_size, "input icon size", f64::EPSILON),
                (&options.icon_spacing, "input icon spacing", 0.0),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, min, None, label, span)?;
                }
            }
            if options.icon.is_none()
                && (options.icon_side.is_some()
                    || options.icon_size.is_some()
                    || options.icon_spacing.is_some())
            {
                return Err(Error::new(
                    "E129",
                    span,
                    "input icon properties require `icon=\"x\"`",
                ));
            }
            check_font(options.font.as_ref(), document, span)?;
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
            infer_route(route, Some(Type::Bool), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Checkbox)?;
        }
        ViewNode::Toggler {
            label,
            checked,
            disabled,
            options,
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
            for expr in [value, min, max, step] {
                require_type(&expr_type(expr, env, document, span)?, &Type::F64, span)?;
            }
            for expr in [&options.default, &options.shift_step]
                .into_iter()
                .flatten()
            {
                require_type(&expr_type(expr, env, document, span)?, &Type::F64, span)?;
            }
            require_literal_range(step, f64::EPSILON, None, "slider step", span)?;
            if let Some(shift_step) = &options.shift_step {
                require_literal_range(shift_step, f64::EPSILON, None, "slider shift step", span)?;
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
            check_slider_styles(&options.style, env, document, span)?;
            infer_route(route, Some(Type::F64), env, document, signatures)?;
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
            for color in [&options.background, &options.bar, &options.border_color]
                .into_iter()
                .flatten()
            {
                if !valid_theme_color(color, document) {
                    return Err(Error::new(
                        "E129",
                        span,
                        format!("unknown progress color `{color}`"),
                    ));
                }
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
            styles,
            route,
            span,
        } => {
            require_type(&expr_type(label, env, document, span)?, &Type::Str, span)?;
            let value_type = expr_type(value, env, document, span)?;
            if !matches!(value_type, Type::I64 | Type::Bool) {
                return Err(Error::new(
                    "E125",
                    span,
                    "radio values must be i64 or bool in Ice 0.2",
                ));
            }
            require_type(
                &expr_type(selected, env, document, span)?,
                &Type::Bool,
                span,
            )?;
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
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
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
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, label, span)?;
                }
            }
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
            infer_route(route, Some(Type::Str), env, document, signatures)?;
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
            content,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
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
                let actual = expr_type(arg, env, document, span)?;
                require_type(&actual, expected, span)?;
            }
            match (slot_count(&component.root) == 1, content) {
                (true, None) => {
                    return Err(Error::new(
                        "E124",
                        span,
                        format!("component `{name}` requires one child for its slot"),
                    ));
                }
                (false, Some(_)) => {
                    return Err(Error::new(
                        "E124",
                        span,
                        format!("component `{name}` does not declare a slot"),
                    )
                    .hint("add `slot` inside the component definition"));
                }
                _ => {}
            }
            if let Some(content) = content {
                let mut child_ids = HashSet::new();
                infer_view(content, env, document, signatures, &mut child_ids)?;
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
        ViewNode::Media {
            source,
            options,
            span,
            ..
        } => {
            require_type(&expr_type(source, env, document, span)?, &Type::Str, span)?;
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
            if let Some(expand) = &options.expand {
                require_type(&expr_type(expand, env, document, span)?, &Type::Bool, span)?;
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
            for color in [
                &options.background,
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
        ViewNode::Theme {
            text,
            background,
            content,
            span,
            ..
        } => {
            for (color, label) in [(text, "text"), (background, "background")] {
                if let Some(color) = color
                    && !valid_theme_color(color, document)
                {
                    return Err(Error::new(
                        "E137",
                        span,
                        format!("unknown nested theme {label} color `{color}`"),
                    ));
                }
            }
            infer_view(content, env, document, signatures, ids)?;
        }
        ViewNode::Float {
            scale,
            x,
            y,
            content,
            span,
        } => {
            for value in [scale, x, y] {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            }
            require_literal_range(scale, f64::EPSILON, None, "float scale", span)?;
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

fn lazy_hashable(ty: &Type) -> bool {
    match ty {
        Type::Bool | Type::I64 | Type::Str | Type::Named(_) => true,
        Type::List(inner) | Type::Option(inner) => lazy_hashable(inner),
        Type::F64
        | Type::Combo(_)
        | Type::Markdown
        | Type::Editor
        | Type::KeyPress
        | Type::KeyRelease
        | Type::KeyModifiers
        | Type::SystemInfo
        | Type::Unit
        | Type::Unknown => false,
    }
}

fn check_lazy_subtree(
    node: &ViewNode,
    document: &Document,
    components: &mut HashSet<String>,
    supplied_slot: bool,
) -> Result<(), Error> {
    match node {
        ViewNode::Input { span, .. } => Err(Error::new(
            "E139",
            span,
            "input cannot live in lazy because iced text input borrows app state",
        )),
        ViewNode::ComboBox { span, .. } => Err(Error::new(
            "E139",
            span,
            "combo cannot live in lazy because iced combo box borrows search state",
        )),
        ViewNode::QrCode { span, .. } => Err(Error::new(
            "E139",
            span,
            "named QR data cannot live in lazy because iced QR code borrows app state",
        )),
        ViewNode::Markdown { span, .. } => Err(Error::new(
            "E139",
            span,
            "markdown cannot live in lazy because iced markdown borrows parsed content",
        )),
        ViewNode::TextEditor { span, .. } => Err(Error::new(
            "E139",
            span,
            "editor cannot live in lazy because iced text editor borrows content state",
        )),
        ViewNode::Slot { span } if !supplied_slot => Err(Error::new(
            "E139",
            span,
            "a lazy subtree cannot borrow a slot from its enclosing component",
        )),
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => {
            for child in children {
                check_lazy_subtree(child, document, components, supplied_slot)?;
            }
            Ok(())
        }
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => {
            check_lazy_subtree(content, document, components, supplied_slot)
        }
        ViewNode::Tooltip { content, tip, .. } => {
            check_lazy_subtree(content, document, components, supplied_slot)?;
            check_lazy_subtree(tip, document, components, supplied_slot)
        }
        ViewNode::Table { columns, .. } => {
            for column in columns {
                check_lazy_subtree(&column.header, document, components, supplied_slot)?;
                check_lazy_subtree(&column.cell, document, components, supplied_slot)?;
            }
            Ok(())
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                check_lazy_subtree(narrow, document, components, supplied_slot)?;
                check_lazy_subtree(wide, document, components, supplied_slot)
            }
            ResponsiveContent::Size { content, .. } => {
                check_lazy_subtree(content, document, components, supplied_slot)
            }
        },
        ViewNode::Component {
            name,
            content,
            span,
            ..
        } => {
            if let Some(content) = content {
                check_lazy_subtree(content, document, components, supplied_slot)?;
            }
            if !components.insert(name.clone()) {
                return Err(Error::new(
                    "E139",
                    span,
                    format!("recursive component `{name}` cannot be used in lazy"),
                ));
            }
            let component = document
                .components
                .iter()
                .find(|component| component.name == *name)
                .expect("component names are checked before lazy safety");
            let result =
                check_lazy_subtree(&component.root, document, components, content.is_some());
            components.remove(name);
            result
        }
        _ => Ok(()),
    }
}

fn require_literal_range(
    expr: &Expr,
    min: f64,
    max: Option<f64>,
    label: &str,
    span: &Span,
) -> Result<(), Error> {
    let literal = f64_literal(expr);
    if literal.is_some_and(|value| value < min || max.is_some_and(|max| value > max)) {
        return Err(Error::new(
            "E128",
            span,
            format!("{label} is outside its valid range"),
        ));
    }
    Ok(())
}

fn f64_literal(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::F64(value) => Some(*value),
        Expr::Unary {
            op: UnaryOp::Neg,
            value,
        } if matches!(value.as_ref(), Expr::F64(_)) => {
            let Expr::F64(value) = value.as_ref() else {
                unreachable!()
            };
            Some(-value)
        }
        _ => None,
    }
}

fn check_bool_control_options(
    options: &BoolControlOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    check_font(options.font.as_ref(), document, span)?;
    if let Some(length) = &options.width
        && let LengthValue::Fixed(value) = length
    {
        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
        require_literal_range(value, 0.0, None, "control width", span)?;
    }
    for (value, label, min) in [
        (&options.size, "control size", f64::EPSILON),
        (&options.spacing, "control spacing", 0.0),
        (&options.text_size, "control text size", f64::EPSILON),
        (&options.line_height, "control line height", f64::EPSILON),
        (&options.icon_size, "checkbox icon size", f64::EPSILON),
        (
            &options.icon_line_height,
            "checkbox icon line height",
            f64::EPSILON,
        ),
    ] {
        if let Some(value) = value {
            require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            require_literal_range(value, min, None, label, span)?;
        }
    }
    if options.icon.is_none()
        && (options.icon_size.is_some()
            || options.icon_line_height.is_some()
            || options.icon_shaping.is_some())
    {
        return Err(Error::new(
            "E129",
            span,
            "checkbox icon properties require `icon=\"x\"`",
        ));
    }
    Ok(())
}

fn check_slider_styles(
    styles: &SliderStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [&styles.active, &styles.hovered, &styles.dragged]
        .into_iter()
        .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        for color in [
            &style.rail_start,
            &style.rail_end,
            &style.rail_border_color,
            &style.handle_color,
            &style.handle_border_color,
        ]
        .into_iter()
        .flatten()
        {
            if !valid_theme_color(color, document) {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("unknown slider color `{color}`"),
                ));
            }
        }
        for (value, label) in [
            (&style.rail_width, "slider rail width"),
            (&style.rail_border_width, "slider rail border width"),
            (&style.rail_radius, "slider rail radius"),
            (&style.rail_radius_top_left, "slider rail radius"),
            (&style.rail_radius_top_right, "slider rail radius"),
            (&style.rail_radius_bottom_right, "slider rail radius"),
            (&style.rail_radius_bottom_left, "slider rail radius"),
            (&style.handle_border_width, "slider handle border width"),
            (&style.handle_radius, "slider handle radius"),
            (&style.handle_radius_top_left, "slider handle radius"),
            (&style.handle_radius_top_right, "slider handle radius"),
            (&style.handle_radius_bottom_right, "slider handle radius"),
            (&style.handle_radius_bottom_left, "slider handle radius"),
        ] {
            if let Some(value) = value {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_literal_range(value, 0.0, None, label, span)?;
            }
        }
        if let Some(SliderHandleShape::Circle(radius)) = &style.handle_shape {
            require_type(&expr_type(radius, env, document, span)?, &Type::F64, span)?;
            require_literal_range(radius, 0.0, None, "slider handle radius", span)?;
        }
        let has_handle_radius = style.handle_radius.is_some()
            || style.handle_radius_top_left.is_some()
            || style.handle_radius_top_right.is_some()
            || style.handle_radius_bottom_right.is_some()
            || style.handle_radius_bottom_left.is_some();
        if has_handle_radius
            && !matches!(
                &style.handle_shape,
                Some(SliderHandleShape::Rectangle { .. })
            )
        {
            return Err(Error::new(
                "E129",
                span,
                "slider handle radius requires `handle=rect(N)` in the same status",
            ));
        }
    }
    Ok(())
}

fn check_text_options(
    options: &TextOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    check_font(options.font.as_ref(), document, span)?;
    for length in [&options.width, &options.height].into_iter().flatten() {
        if let LengthValue::Fixed(value) = length {
            require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            require_literal_range(value, 0.0, None, "text bounds", span)?;
        }
    }
    for (value, label) in [
        (options.size.as_ref(), "text size"),
        (
            options.line_height.as_ref().map(|height| match height {
                TextLineHeight::Relative(value) | TextLineHeight::Absolute(value) => value,
            }),
            "text line height",
        ),
    ] {
        if let Some(value) = value {
            require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            require_literal_range(value, f64::EPSILON, None, label, span)?;
        }
    }
    Ok(())
}

fn infer_subscriptions(
    document: &Document,
    states: &HashMap<String, Type>,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    for subscription in &document.subscriptions {
        let output = match &subscription.source {
            SubscriptionSource::Extern { function, args } => {
                let source = extern_function(
                    document,
                    function,
                    ExternKind::Subscription,
                    &subscription.span,
                )?;
                check_call_args(source, args, states, document, &subscription.span)?;
                source.output.clone()
            }
            SubscriptionSource::Keyboard(KeyboardEvent::Press) => Type::KeyPress,
            SubscriptionSource::Keyboard(KeyboardEvent::Release) => Type::KeyRelease,
            SubscriptionSource::Keyboard(KeyboardEvent::Modifiers) => Type::KeyModifiers,
            SubscriptionSource::SystemTheme => Type::Str,
        };
        if subscription
            .route
            .args
            .iter()
            .any(|arg| !matches!(arg, RouteArg::Payload))
        {
            return Err(Error::new(
                "E127",
                &subscription.span,
                "subscription routes only accept `_`; read other state in the handler",
            ));
        }
        infer_route(
            &subscription.route,
            Some(output),
            states,
            document,
            signatures,
        )?;
    }
    Ok(())
}

fn infer_runs(
    handler: &Handler,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    let unknown_env: HashMap<String, Type> = handler
        .params
        .iter()
        .map(|param| (param.name.clone(), Type::Unknown))
        .collect();
    for statement in &handler.statements {
        if let Statement::WidgetOperation {
            operation: WidgetOperation::Focused { .. },
            route: Some(route),
            ..
        } = statement
        {
            infer_route(route, Some(Type::Bool), &unknown_env, document, signatures)?;
        }
        if let Statement::Run {
            kind,
            function,
            args,
            success,
            error,
            span,
        } = statement
        {
            if let Some(output) = builtin_task_output(*kind, function, args, span)? {
                infer_route(success, Some(output), &unknown_env, document, signatures)?;
                if error.is_some() {
                    return Err(Error::new(
                        "E131",
                        span,
                        "system tasks are infallible and cannot have an error route",
                    ));
                }
                continue;
            }
            let action = extern_function(document, function, (*kind).into(), span)?;
            infer_route(
                success,
                Some(action.output.clone()),
                &unknown_env,
                document,
                signatures,
            )?;
            match (&action.error, error) {
                (Some(error_ty), Some(route)) => infer_route(
                    route,
                    Some(error_ty.clone()),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                (Some(_), None) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "fallible extern fn requires an error route",
                    ));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "infallible extern fn cannot have an error route",
                    ));
                }
                (None, None) => {}
            }
        }
    }
    Ok(())
}

fn infer_route(
    route: &Route,
    payload: Option<Type>,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    if route.handler == "mount" {
        return Err(Error::new(
            "E135",
            &route.span,
            "`mount` is initialization-only and cannot receive events",
        ));
    }
    let signature = signatures.get_mut(&route.handler).ok_or_else(|| {
        Error::new(
            "E132",
            &route.span,
            format!("unknown handler `{}`", route.handler),
        )
    })?;
    if signature.len() != route.args.len() {
        return Err(Error::new(
            "E133",
            &route.span,
            format!(
                "handler `{}` expects {} arguments, got {}",
                route.handler,
                signature.len(),
                route.args.len()
            ),
        ));
    }
    for (slot, arg) in signature.iter_mut().zip(&route.args) {
        let ty = match arg {
            RouteArg::Payload => payload
                .clone()
                .ok_or_else(|| Error::new("E134", &route.span, "this route has no `_` payload"))?,
            RouteArg::Expr(expr) => expr_type(expr, env, document, &route.span)?,
        };
        if ty == Type::Unknown {
            continue;
        }
        if let Some(existing) = slot {
            if !compatible(existing, &ty) {
                return Err(type_error(&route.span, existing, &ty));
            }
        } else {
            *slot = Some(ty);
        }
    }
    Ok(())
}

fn infer_ordered_payload_route(
    route: &Route,
    payloads: &[Type],
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    label: &str,
) -> Result<(), Error> {
    if route.args.len() != payloads.len()
        || route
            .args
            .iter()
            .any(|arg| !matches!(arg, RouteArg::Payload))
    {
        return Err(Error::new(
            "E129",
            &route.span,
            format!("{label} route expects {} payloads", payloads.len()),
        ));
    }
    infer_route(route, Some(Type::Unknown), env, document, signatures)?;
    let signature = signatures.get_mut(&route.handler).expect("route signature");
    for (slot, ty) in signature.iter_mut().zip(payloads) {
        if let Some(existing) = slot {
            if !compatible(existing, ty) {
                return Err(type_error(&route.span, existing, ty));
            }
        } else {
            *slot = Some(ty.clone());
        }
    }
    Ok(())
}

fn check_handler(
    handler: &Handler,
    states: &HashMap<String, Type>,
    document: &Document,
    operation_ids: &HashSet<String>,
) -> Result<(), Error> {
    let mut env = states.clone();
    env.extend(
        handler
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone())),
    );
    for (index, statement) in handler.statements.iter().enumerate() {
        match statement {
            Statement::Assign {
                target,
                value,
                span,
            } => {
                let expected = states.get(target).ok_or_else(|| {
                    Error::new("E140", span, format!("`{target}` is not writable state"))
                })?;
                if matches!(expected, Type::Combo(_)) {
                    return Err(Error::new(
                        "E140",
                        span,
                        "combo search state is initialized once and cannot be assigned",
                    ));
                }
                let actual = expr_type(value, &env, document, span)?;
                require_type(&actual, expected, span)?;
            }
            Statement::ReturnIf { condition, span } => {
                require_type(
                    &expr_type(condition, &env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            Statement::Run {
                kind,
                function,
                args,
                span,
                ..
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "run must be the final statement in a handler",
                    ));
                }
                if builtin_task_output(*kind, function, args, span)?.is_some() {
                    continue;
                }
                let action = extern_function(document, function, (*kind).into(), span)?;
                check_call_args(action, args, &env, document, span)?;
            }
            Statement::ClipboardWrite { value, span, .. } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "clipboard write must be the final statement in a handler",
                    ));
                }
                require_type(&expr_type(value, &env, document, span)?, &Type::Str, span)?;
            }
            Statement::WidgetOperation {
                operation,
                route,
                span,
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E172",
                        span,
                        "widget operation must be the final statement in a handler",
                    ));
                }
                let target = match operation {
                    WidgetOperation::FocusPrevious | WidgetOperation::FocusNext => None,
                    WidgetOperation::Focus { id }
                    | WidgetOperation::Focused { id }
                    | WidgetOperation::CursorFront { id }
                    | WidgetOperation::CursorEnd { id }
                    | WidgetOperation::Cursor { id, .. }
                    | WidgetOperation::SelectAll { id }
                    | WidgetOperation::Select { id, .. }
                    | WidgetOperation::Snap { id, .. }
                    | WidgetOperation::SnapEnd { id }
                    | WidgetOperation::ScrollTo { id, .. }
                    | WidgetOperation::ScrollBy { id, .. } => Some(id),
                };
                if let Some(id) = target
                    && !operation_ids.contains(id)
                {
                    return Err(Error::new(
                        "E172",
                        span,
                        format!("unknown static app widget `#{id}`"),
                    )
                    .hint("declare this ID in the app view; repeated and component IDs need a scoped selector"));
                }
                match (operation, route) {
                    (WidgetOperation::Focused { .. }, None) => {
                        return Err(Error::new(
                            "E172",
                            span,
                            "widget focused requires `-> handler _`",
                        ));
                    }
                    (WidgetOperation::Focused { .. }, Some(_)) => {}
                    (_, Some(_)) => {
                        return Err(Error::new(
                            "E172",
                            span,
                            "widget effects do not produce a route",
                        ));
                    }
                    (_, None) => {}
                }
                for value in match operation {
                    WidgetOperation::Cursor { position, .. } => vec![(position, "cursor position")],
                    WidgetOperation::Select { start, end, .. } => {
                        vec![(start, "selection start"), (end, "selection end")]
                    }
                    _ => Vec::new(),
                } {
                    require_type(&expr_type(value.0, &env, document, span)?, &Type::I64, span)?;
                    if matches!(value.0, Expr::I64(number) if *number < 0) {
                        return Err(Error::new(
                            "E172",
                            span,
                            format!("{} cannot be negative", value.1),
                        ));
                    }
                }
                if let WidgetOperation::Select {
                    start: Expr::I64(start),
                    end: Expr::I64(end),
                    ..
                } = operation
                    && start > end
                {
                    return Err(Error::new(
                        "E172",
                        span,
                        "selection start cannot exceed end",
                    ));
                }
                for (value, relative) in match operation {
                    WidgetOperation::Snap { x, y, .. } => vec![(x, true), (y, true)],
                    WidgetOperation::ScrollTo { x, y, .. }
                    | WidgetOperation::ScrollBy { x, y, .. } => vec![(x, false), (y, false)],
                    _ => Vec::new(),
                } {
                    require_type(&expr_type(value, &env, document, span)?, &Type::F64, span)?;
                    if relative {
                        require_literal_range(
                            value,
                            0.0,
                            Some(1.0),
                            "relative scroll offset",
                            span,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

impl From<EffectKind> for ExternKind {
    fn from(value: EffectKind) -> Self {
        match value {
            EffectKind::Future => Self::Future,
            EffectKind::Task => Self::Task,
        }
    }
}

fn extern_function<'a>(
    document: &'a Document,
    name: &str,
    kind: ExternKind,
    span: &Span,
) -> Result<&'a ExternFn, Error> {
    document
        .functions
        .iter()
        .find(|item| item.name == name && item.kind == kind)
        .ok_or_else(|| {
            let label = match kind {
                ExternKind::Future => "function",
                ExternKind::Component => "component",
                ExternKind::Task => "task",
                ExternKind::Subscription => "subscription",
            };
            Error::new("E130", span, format!("unknown extern {label} `{name}`"))
        })
}

fn check_call_args(
    function: &ExternFn,
    args: &[Expr],
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if args.len() != function.params.len() {
        return Err(Error::new(
            "E142",
            span,
            format!(
                "extern `{}` expects {} arguments, got {}",
                function.name,
                function.params.len(),
                args.len()
            ),
        ));
    }
    for (arg, (_, expected)) in args.iter().zip(&function.params) {
        let actual = expr_type(arg, env, document, span)?;
        require_type(&actual, expected, span)?;
    }
    Ok(())
}

fn builtin_task_output(
    kind: EffectKind,
    function: &str,
    args: &[Expr],
    span: &Span,
) -> Result<Option<Type>, Error> {
    let output = match (kind, function) {
        (EffectKind::Task, "__ice_system_info") => Some(Type::SystemInfo),
        (EffectKind::Task, "__ice_system_theme") => Some(Type::Str),
        (EffectKind::Task, "__ice_clipboard_read" | "__ice_clipboard_read_primary") => {
            Some(Type::Option(Box::new(Type::Str)))
        }
        _ => None,
    };
    if output.is_some() && !args.is_empty() {
        return Err(Error::new("E142", span, "system tasks take no arguments"));
    }
    Ok(output)
}

pub(crate) fn expr_type(
    expr: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    match expr {
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::I64(_) => Ok(Type::I64),
        Expr::F64(_) => Ok(Type::F64),
        Expr::Str(_) => Ok(Type::Str),
        Expr::EmptyList => Ok(Type::List(Box::new(Type::Unknown))),
        Expr::List(values) => {
            let Some(first) = values.first() else {
                return Ok(Type::List(Box::new(Type::Unknown)));
            };
            let ty = expr_type(first, env, document, span)?;
            for value in &values[1..] {
                let actual = expr_type(value, env, document, span)?;
                require_type(&actual, &ty, span)?;
            }
            Ok(Type::List(Box::new(ty)))
        }
        Expr::None => Ok(Type::Option(Box::new(Type::Unknown))),
        Expr::Path(path) => {
            let mut ty = env
                .get(&path[0])
                .cloned()
                .ok_or_else(|| Error::new("E150", span, format!("unknown value `{}`", path[0])))?;
            for field in &path[1..] {
                ty = field_type(&ty, field, document, span)?;
            }
            Ok(ty)
        }
        Expr::Call { name, args } => match name.as_str() {
            "len" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "len expects one argument"));
                }
                match expr_type(&args[0], env, document, span)? {
                    Type::List(_) | Type::Str => Ok(Type::I64),
                    actual => Err(Error::new(
                        "E152",
                        span,
                        format!("len does not accept `{}`", actual.display()),
                    )),
                }
            }
            "empty" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "empty expects one argument"));
                }
                match expr_type(&args[0], env, document, span)? {
                    Type::List(_) | Type::Str => Ok(Type::Bool),
                    actual => Err(Error::new(
                        "E152",
                        span,
                        format!("empty does not accept `{}`", actual.display()),
                    )),
                }
            }
            "trim" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "trim expects one argument"));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Str)
            }
            "some" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "some expects one argument"));
                }
                Ok(Type::Option(Box::new(expr_type(
                    &args[0], env, document, span,
                )?)))
            }
            "markdown" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "markdown expects one argument"));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Markdown)
            }
            "editor" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "editor expects one argument"));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Editor)
            }
            _ => Err(Error::new(
                "E152",
                span,
                format!("unknown function `{name}`"),
            )),
        },
        Expr::Unary { op, value } => {
            let actual = expr_type(value, env, document, span)?;
            match op {
                UnaryOp::Not => {
                    require_type(&actual, &Type::Bool, span)?;
                    Ok(Type::Bool)
                }
                UnaryOp::Neg if matches!(actual, Type::I64 | Type::F64) => Ok(actual),
                UnaryOp::Neg => Err(Error::new(
                    "E153",
                    span,
                    "numeric negation expects i64 or f64",
                )),
            }
        }
        Expr::Binary { left, op, right } => {
            let left = expr_type(left, env, document, span)?;
            let right = expr_type(right, env, document, span)?;
            match op {
                BinaryOp::And | BinaryOp::Or => {
                    require_type(&left, &Type::Bool, span)?;
                    require_type(&right, &Type::Bool, span)?;
                    Ok(Type::Bool)
                }
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq => {
                    require_type(&left, &right, span)?;
                    Ok(Type::Bool)
                }
                _ => {
                    if !matches!(left, Type::I64 | Type::F64) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "arithmetic expects numeric values",
                        ));
                    }
                    require_type(&left, &right, span)?;
                    Ok(left)
                }
            }
        }
    }
}

fn field_type(ty: &Type, field: &str, document: &Document, span: &Span) -> Result<Type, Error> {
    let found = match ty {
        Type::Named(name) => {
            let item = document
                .structs
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| {
                    Error::new("E151", span, format!("unknown extern struct `{name}`"))
                })?;
            return item
                .fields
                .iter()
                .find(|(name, _)| name == field)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| {
                    Error::new(
                        "E151",
                        span,
                        format!("struct `{name}` has no field `{field}`"),
                    )
                });
        }
        Type::KeyPress => match field {
            "key" | "modified_key" | "physical_key" | "location" => Some(Type::Str),
            "modifiers" => Some(Type::KeyModifiers),
            "text" => Some(Type::Option(Box::new(Type::Str))),
            "repeat" => Some(Type::Bool),
            _ => None,
        },
        Type::KeyRelease => match field {
            "key" | "modified_key" | "physical_key" | "location" => Some(Type::Str),
            "modifiers" => Some(Type::KeyModifiers),
            _ => None,
        },
        Type::KeyModifiers => match field {
            "shift" | "control" | "alt" | "logo" | "command" | "jump" | "macos_command" => {
                Some(Type::Bool)
            }
            _ => None,
        },
        Type::SystemInfo => match field {
            "system_name" | "system_kernel" | "system_version" | "system_short_version" => {
                Some(Type::Option(Box::new(Type::Str)))
            }
            "cpu_brand" | "graphics_backend" | "graphics_adapter" => Some(Type::Str),
            "cpu_cores" | "memory_used" => Some(Type::Option(Box::new(Type::I64))),
            "memory_total" => Some(Type::I64),
            _ => None,
        },
        _ => None,
    };
    found.ok_or_else(|| {
        Error::new(
            "E151",
            span,
            format!("type `{}` has no field `{field}`", ty.display()),
        )
    })
}

fn check_id(
    id: &Option<Id>,
    env: &HashMap<String, Type>,
    document: &Document,
    ids: &mut HashSet<String>,
    span: &Span,
) -> Result<(), Error> {
    let Some(id) = id else {
        return Ok(());
    };
    if let Some(key) = &id.key {
        let ty = expr_type(key, env, document, span)?;
        if !matches!(ty, Type::I64 | Type::Str) {
            return Err(Error::new(
                "E160",
                span,
                "dynamic id keys must be i64 or str",
            ));
        }
    } else if !ids.insert(id.name.clone()) {
        return Err(Error::new(
            "E161",
            span,
            format!("duplicate local id `#{}`", id.name),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum StyleTarget {
    Layout(Layout),
    Text,
    Input,
    Button,
    Checkbox,
    Toggler,
    Slider,
    Progress,
    Radio,
    Rule,
    Space,
}

fn valid_theme_color(value: &str, document: &Document) -> bool {
    let (name, opacity) = value
        .split_once('/')
        .map_or((value, None), |(name, opacity)| (name, Some(opacity)));
    (["white", "black", "transparent"].contains(&name) || document.theme.contains_key(name))
        && opacity.is_none_or(|opacity| opacity.parse::<u8>().is_ok_and(|opacity| opacity <= 100))
}

fn check_styles(
    styles: &[String],
    document: &Document,
    span: &Span,
    target: StyleTarget,
) -> Result<(), Error> {
    let spacing = [
        "0", "1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20", "24",
    ];
    let is_linear = matches!(target, StyleTarget::Layout(Layout::Column | Layout::Row));
    let is_box = matches!(
        target,
        StyleTarget::Layout(Layout::Column | Layout::Row | Layout::Grid | Layout::Stack)
    );
    let target_name = match target {
        StyleTarget::Layout(Layout::Column) => "col",
        StyleTarget::Layout(Layout::Row) => "row",
        StyleTarget::Layout(Layout::Scroll) => "scroll",
        StyleTarget::Layout(Layout::Grid) => "grid",
        StyleTarget::Layout(Layout::Stack) => "stack",
        StyleTarget::Text => "text",
        StyleTarget::Input => "input",
        StyleTarget::Button => "button",
        StyleTarget::Checkbox => "checkbox",
        StyleTarget::Toggler => "toggler",
        StyleTarget::Slider => "slider",
        StyleTarget::Progress => "progress",
        StyleTarget::Radio => "radio",
        StyleTarget::Rule => "rule",
        StyleTarget::Space => "space",
    };

    for original in styles {
        let (variant, utility) = original
            .split_once(':')
            .map_or((None, original.as_str()), |(variant, utility)| {
                (Some(variant), utility)
            });
        let color = ["bg-", "text-", "border-"]
            .iter()
            .find_map(|prefix| utility.strip_prefix(prefix));
        let valid_color = color.is_some_and(|value| valid_theme_color(value, document));
        let valid_spacing = ["p-", "px-", "py-", "gap-"].iter().any(|prefix| {
            utility
                .strip_prefix(prefix)
                .is_some_and(|value| spacing.contains(&value))
        });
        let known = matches!(
            utility,
            "w-full"
                | "h-full"
                | "max-w-sm"
                | "max-w-md"
                | "max-w-lg"
                | "max-w-xl"
                | "max-w-2xl"
                | "items-center"
                | "self-center"
                | "text-xs"
                | "text-sm"
                | "text-base"
                | "text-lg"
                | "text-xl"
                | "text-2xl"
                | "font-bold"
                | "border"
                | "border-2"
                | "rounded-sm"
                | "rounded"
                | "rounded-md"
                | "rounded-lg"
                | "rounded-full"
        ) || valid_spacing
            || valid_color
            || utility
                .strip_prefix("opacity-")
                .is_some_and(|value| ["0", "25", "50", "75", "100"].contains(&value));

        if !known {
            return Err(Error::new(
                "E041",
                span,
                format!("unsupported utility `{original}`"),
            ));
        }

        let supported = match variant {
            Some("hover" | "pressed") => {
                matches!(target, StyleTarget::Button) && utility.starts_with("bg-")
            }
            Some("focus") => matches!(target, StyleTarget::Input) && utility.starts_with("border-"),
            Some("disabled") => {
                matches!(target, StyleTarget::Button) && utility.starts_with("opacity-")
            }
            Some(_) => false,
            None => match utility {
                "w-full" => matches!(target, StyleTarget::Layout(_) | StyleTarget::Input),
                "h-full" => matches!(target, StyleTarget::Layout(_)),
                "max-w-sm" | "max-w-md" | "max-w-lg" | "max-w-xl" | "max-w-2xl" | "self-center" => {
                    is_box
                }
                "items-center" => is_linear,
                "text-xs" | "text-sm" | "text-base" | "text-lg" | "text-xl" | "text-2xl"
                | "font-bold" => matches!(target, StyleTarget::Text),
                "border" | "border-2" => is_box || matches!(target, StyleTarget::Input),
                "rounded-sm" | "rounded" | "rounded-md" | "rounded-lg" | "rounded-full" => {
                    is_box || matches!(target, StyleTarget::Input | StyleTarget::Button)
                }
                _ if utility.starts_with("gap-") => {
                    is_linear || matches!(target, StyleTarget::Layout(Layout::Grid))
                }
                _ if utility.starts_with("p-")
                    || utility.starts_with("px-")
                    || utility.starts_with("py-") =>
                {
                    is_box || matches!(target, StyleTarget::Input | StyleTarget::Button)
                }
                _ if utility.starts_with("bg-") => {
                    is_box || matches!(target, StyleTarget::Input | StyleTarget::Button)
                }
                _ if utility.starts_with("text-") => {
                    is_box || matches!(target, StyleTarget::Text | StyleTarget::Button)
                }
                _ if utility.starts_with("border-") => {
                    is_box || matches!(target, StyleTarget::Input)
                }
                _ => false,
            },
        };
        if !supported {
            return Err(Error::new(
                "E042",
                span,
                format!("utility `{original}` has no effect on `{target_name}`"),
            ));
        }
    }

    let has_border = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| matches!(utility, "border" | "border-2"));
    let has_border_color = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("border-") && utility != "border-2");
    if (is_box || matches!(target, StyleTarget::Input)) && has_border_color && !has_border {
        return Err(Error::new(
            "E044",
            span,
            "border color utilities require `border` or `border-2` on the same node",
        ));
    }
    let has_radius = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("rounded"));
    let has_background = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("bg-"));
    if is_box && has_radius && !has_background && !has_border {
        return Err(Error::new(
            "E044",
            span,
            "rounded layout requires a background or border on the same node",
        ));
    }
    Ok(())
}

fn base_utility(style: &str) -> &str {
    style.split_once(':').map_or(style, |(_, utility)| utility)
}

fn require_type(actual: &Type, expected: &Type, span: &Span) -> Result<(), Error> {
    if compatible(actual, expected) {
        Ok(())
    } else {
        Err(type_error(span, expected, actual))
    }
}

fn compatible(left: &Type, right: &Type) -> bool {
    left == right
        || *left == Type::Unknown
        || *right == Type::Unknown
        || match (left, right) {
            (Type::List(left), Type::List(right)) | (Type::Option(left), Type::Option(right)) => {
                compatible(left, right)
            }
            _ => false,
        }
}

fn type_error(span: &Span, expected: &Type, actual: &Type) -> Error {
    Error::new(
        "E101",
        span,
        format!(
            "expected `{}`, got `{}`",
            expected.display(),
            actual.display()
        ),
    )
}

#[cfg(test)]
mod tests {
    use crate::analyze;

    #[test]
    fn infers_action_result_handler() {
        let source = r#"app Demo
extern crate::backend
  Item(id:i64)
  load() -> [Item] ! Item
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
on mount
  run load() -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items) @text-sm
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "[Item]");
    }

    #[test]
    fn checks_optional_selection_values() {
        let source = r#"app Demo
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
view
  pick choices selected placeholder="Choose" open=opened -> selected _
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty.display(), "[str]");
        assert_eq!(document.states[1].ty.display(), "str?");
        assert_eq!(document.handlers[0].params[0].ty.display(), "str");
    }

    #[test]
    fn rejects_a_non_optional_pick_selection() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  choices = ["List", "Board"]
  selected = "List"
on selected(next)
  selected = next
view
  pick choices selected -> selected _
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("optional"));
    }

    #[test]
    fn checks_qr_declarations_and_references() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
qr code "hello" version=micro(0)
view
  qr code
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E136");
        assert!(error.message.contains("micro(1..4)"));

        let source = source.replace(
            "qr code \"hello\" version=micro(0)",
            "qr saved \"hello\" version=micro(4)",
        );
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E136");
        assert!(error.message.contains("unknown qr data `code`"));
    }

    #[test]
    fn rejects_unknown_nested_theme_colors() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  theme dark text=missing
    text "Hello"
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E137");
        assert!(error.message.contains("missing"));
    }

    #[test]
    fn checks_component_slot_contracts() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
component Card(title:str)
  col
    text title
    slot
view
  Card("Editor")
    input "Name" <-> draft
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "  Card(\"Editor\")\n    input \"Name\" <-> draft",
            "  Card(\"Editor\")",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("requires one child"));

        let error =
            analyze(&source.replace("    text title\n    slot", "    text title")).unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("does not declare a slot"));
    }

    #[test]
    fn checks_keyed_columns_and_copyable_keys() {
        let source = r#"app Demo
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
  keyed item in items by=item.id width=fill height=shrink spacing=8.0 padding=4.0 max-width=640.0 align=center
    text item.name
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("by=item.id", "by=item.name")).unwrap_err();
        assert_eq!(error.code, "E138");
        assert!(error.message.contains("bool, i64, or f64"));

        let error = analyze(&source.replace("spacing=8.0", "spacing=-1.0")).unwrap_err();
        assert!(error.message.contains("outside its valid range"));
    }

    #[test]
    fn checks_lazy_static_boundaries() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  title = "Hello"
  other = "Outside"
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("text len(cached)", "text other")).unwrap_err();
        assert_eq!(error.code, "E150");
        assert!(error.message.contains("unknown value `other`"));

        let error = analyze(&source.replace("title = \"Hello\"", "title = 1.0")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("stable hashing"));

        let error =
            analyze(&source.replace("text len(cached)", "input \"Edit\" <-> cached")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("borrows app state"));

        let component_source = source.replace(
            "view\n  lazy title as cached\n    col\n      text cached\n      text len(cached)",
            "component Editor(value:str)\n  input \"Edit\" <-> value\nview\n  lazy title as cached\n    Editor(cached)",
        );
        let error = analyze(&component_source).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("borrows app state"));
    }

    #[test]
    fn checks_markdown_content_settings_and_links() {
        let source = r##"app Docs
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  docs:markdown = "# Hello [world](https://example.com)"
on open(url)
on reset
  docs = markdown("# Reset")
view
  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 spacing=12.0 -> open _
"##;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty.display(), "markdown");
        assert_eq!(document.handlers[0].params[0].ty.display(), "str");

        let error = analyze(&source.replace("spacing=12.0", "spacing=-1.0")).unwrap_err();
        assert!(error.message.contains("outside its valid range"));

        let error = analyze(&source.replace("markdown docs", "markdown missing")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("unknown markdown state"));
    }

    #[test]
    fn checks_structured_tables_and_metrics() {
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
    column width=fill(2) align-x=left align-y=center
      header
        text "Name"
      cell
        text row.name
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("padding=4.0", "padding=-1.0")).unwrap_err();
        assert!(error.message.contains("outside its valid range"));

        let error = analyze(&source.replace("table row in rows", "table row in true")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("list of rows"));
    }

    #[test]
    fn checks_bound_text_editors_and_highlighting() {
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
  editor #body <-> body placeholder="Write" width=640.0 height=fill min-height=80.0 max-height=240.0 size=14.0 line-height=1.3 padding=8.0 wrapping=word-or-glyph font=mono highlight="rs" highlight-theme=solarized-dark disabled=locked
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty.display(), "editor");

        let error = analyze(&source.replace("min-height=80.0", "min-height=300.0")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("cannot exceed"));
    }

    #[test]
    fn directs_component_editors_through_slots() {
        let source = r#"app Notes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = ""
component EditorPanel(body:editor)
  editor <-> body
view
  EditorPanel(body)
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("component parameter"));
        assert!(error.hint.unwrap().contains("slot"));
    }

    #[test]
    fn rejects_slots_outside_components_and_duplicate_slots() {
        let outside = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  slot
"#;
        let error = analyze(outside).unwrap_err();
        assert_eq!(error.code, "E124");
        assert_eq!(error.line, 8);

        let duplicate = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Card()
  col
    slot
    slot
view
  text "Hello"
"#;
        let error = analyze(duplicate).unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("only one slot"));
    }

    #[test]
    fn checks_combo_search_state_and_routes() {
        let source = r#"app Demo
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
  combo modes selected "Search modes" input=searched hover=hovered open=opened close=closed -> selected _
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty.display(), "combo[str]");
        assert_eq!(document.handlers[0].params[0].ty.display(), "str");
        assert_eq!(document.handlers[1].params[0].ty.display(), "str");
        assert_eq!(document.handlers[2].params[0].ty.display(), "str");
    }

    #[test]
    fn rejects_assignment_to_combo_search_state() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
on reset
  modes = []
on selected(next)
  selected = some(next)
view
  combo modes selected "Search modes" -> selected _
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E140");
        assert!(error.message.contains("cannot be assigned"));
    }

    #[test]
    fn checks_structural_widget_routes_and_ranges() {
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
    stack width=fill(2) height=120.0 clip=true under=1
      text "Base"
      text "Overlay"
    rule horizontal thickness=2.0 style=weak fill=percent(75.0) color=primary/50 radius=4.0 radius-tl=2.0 snap=false
    space width=fill(2) height=shrink
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty.display(), "f64");
        assert_eq!(document.handlers[0].params[1].ty.display(), "f64");
        assert_eq!(document.handlers[1].params[0].ty.display(), "f64");

        let bad_stack = source.replace("height=120.0 clip=true", "height=-1.0 clip=true");
        let error = analyze(&bad_stack).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("stack size"));

        let bad_under = source.replace("under=1", "under=70000");
        let error = analyze(&bad_under).unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("stack under"));

        let duplicate_size_name = source.replace(
            "size=(available_width, available_height)",
            "size=(available_width, available_width)",
        );
        let error = analyze(&duplicate_size_name).unwrap_err();
        assert_eq!(error.code, "E092");
        assert!(error.message.contains("different names"));

        let conflicting_responsive = source.replace(
            "responsive size=(available_width, available_height)",
            "responsive at=600.0 size=(available_width, available_height)",
        );
        let error = analyze(&conflicting_responsive).unwrap_err();
        assert_eq!(error.code, "E092");
        assert!(error.message.contains("either `at=` or `size=`"));
    }

    #[test]
    fn checks_complete_flex_layout_options() {
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
        analyze(source).unwrap();

        let bad_metric = source.replace("spacing=8.0", "spacing=-1.0");
        let error = analyze(&bad_metric).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("column metric"));

        let missing_wrap = source.replace("clip=true wrap wrap-spacing", "clip=true wrap-spacing");
        let error = analyze(&missing_wrap).unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("require `wrap`"));

        let wrong_property = source.replace("row width=", "row max-width=100.0 width=");
        let error = analyze(&wrong_property).unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("unknown layout property"));
    }

    #[test]
    fn checks_complete_grid_sizing() {
        let source = r#"app Layouts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  col
    grid columns=2 width=640.0 spacing=12.0 height=aspect(16.0,9.0)
      text "Fixed"
    grid fluid=240.0 height=fill(2)
      text "Fluid"
"#;
        analyze(source).unwrap();

        let conflicting = source.replace("columns=2", "columns=2 fluid=240.0");
        let error = analyze(&conflicting).unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("mutually exclusive"));

        let zero_fluid = source.replace("fluid=240.0", "fluid=0.0");
        let error = analyze(&zero_fluid).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("grid fluid width"));

        let zero_aspect = source.replace("aspect(16.0,9.0)", "aspect(16.0,0.0)");
        let error = analyze(&zero_aspect).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("grid aspect height"));
    }

    #[test]
    fn rejects_invalid_rule_style_values() {
        let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  rule horizontal fill=percent(101.0)
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("rule percent"));

        let unknown_color = source.replace("fill=percent(101.0)", "color=missing");
        let error = analyze(&unknown_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown rule color"));
    }

    #[test]
    fn checks_slider_options_and_rejects_invalid_ranges() {
        let source = r#"app Controls
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
on changed(next)
  amount = next
view
  col
    slider amount min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 width=fill(2) height=20.0 -> changed _
      active rail-start=primary rail-end=background rail-width=4.0 rail-border=transparent rail-border-width=1.0 rail-radius=2.0 rail-radius-tl=1.0 handle=circle(7.0) handle-color=primary handle-border=foreground handle-border-width=1.0
      hovered rail-start=foreground rail-end=background rail-radius-tr=3.0 rail-radius-br=3.0 rail-radius-bl=2.0 handle=rect(12) handle-color=foreground handle-radius=3.0 handle-radius-tl=1.0 handle-radius-tr=2.0 handle-radius-br=3.0 handle-radius-bl=4.0
      dragged rail-start=danger handle=circle(8.0) handle-color=danger
    slider amount min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 vertical width=20.0 height=fill -> changed _
"#;
        analyze(source).unwrap();

        let bad_step = source.replace("step=5.0", "step=0.0");
        let error = analyze(&bad_step).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("slider step"));

        let bad_axis = source.replace("vertical width=20.0", "vertical width=fill");
        let error = analyze(&bad_axis).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("slider width must be fixed"));

        let bad_range = source.replace("min=0.0 max=100.0", "min=101.0 max=100.0");
        let error = analyze(&bad_range).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("min cannot exceed max"));

        let bad_color = source.replace("rail-start=primary", "rail-start=missing");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown slider color"));

        let bad_metric = source.replace("rail-width=4.0", "rail-width=-1.0");
        let error = analyze(&bad_metric).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("slider rail width"));

        let bad_handle = source.replace("handle=rect(12)", "handle=circle(7.0)");
        let error = analyze(&bad_handle).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("requires `handle=rect"));
    }

    #[test]
    fn checks_progress_options_and_rejects_invalid_style() {
        let source = r#"app Controls
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
view
  col
    progress amount min=0.0 max=100.0 length=fill(2) girth=20.0 style=success background=background bar=primary/75 border=foreground border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=4.0 radius-bl=5.0
    progress amount vertical length=120.0 girth=fill style=warning
"#;
        analyze(source).unwrap();

        let bad_range = source.replace("min=0.0 max=100.0", "min=101.0 max=100.0");
        let error = analyze(&bad_range).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("progress min cannot exceed max"));

        let bad_color = source.replace("bar=primary/75", "bar=missing");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown progress color"));

        let bad_radius = source.replace("radius=4.0", "radius=-1.0");
        let error = analyze(&bad_radius).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("progress radius"));
    }

    #[test]
    fn checks_tooltip_style_and_rejects_invalid_values() {
        let source = r#"app Hints
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  tooltip position=bottom style=rounded background=background text=foreground border=primary/75 border-width=1.0 radius=5.0 radius-tl=2.0 radius-tr=3.0 radius-br=4.0 radius-bl=5.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true
    text "Hover"
    text "Tip"
"#;
        analyze(source).unwrap();

        let bad_color = source.replace("shadow=black/50", "shadow=missing");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown tooltip color"));

        let bad_blur = source.replace("shadow-blur=8.0", "shadow-blur=-1.0");
        let error = analyze(&bad_blur).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("tooltip shadow blur"));

        let bad_style = source.replace("style=rounded", "style=unknown");
        let error = analyze(&bad_style).unwrap_err();
        assert_eq!(error.code, "E086");
        assert!(error.message.contains("tooltip style must be"));
    }

    #[test]
    fn rejects_a_negative_space_length() {
        let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  space width=-1.0
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("space length"));

        let invalid_portion = source.replace("width=-1.0", "width=fill(65536)");
        let error = analyze(&invalid_portion).unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("fill portion"));
    }

    #[test]
    fn rejects_a_non_positive_responsive_breakpoint() {
        let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  responsive at=0.0
    text "Narrow"
    text "Wide"
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("responsive breakpoint"));
    }

    #[test]
    fn infers_mouse_move_and_scroll_payloads() {
        let source = r#"app Pointer
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  x = 0.0
  y = 0.0
  pixels = false
on moved(next_x, next_y)
  x = next_x
  y = next_y
on scrolled(delta_x, delta_y, pixel_units)
  x = delta_x
  y = delta_y
  pixels = pixel_units
view
  mouse move=moved scroll=scrolled cursor=crosshair
    text "Track me"
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty.display(), "f64");
        assert_eq!(document.handlers[0].params[1].ty.display(), "f64");
        assert_eq!(document.handlers[1].params[0].ty.display(), "f64");
        assert_eq!(document.handlers[1].params[2].ty.display(), "bool");
    }

    #[test]
    fn rejects_wrong_mouse_move_arity() {
        let source = r#"app Pointer
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on moved(x)
view
  mouse move=moved(_)
    text "Track me"
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("mouse move"));
    }

    #[test]
    fn checks_scrollable_configuration_and_offsets() {
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
        let document = analyze(source).unwrap();
        for param in &document.handlers[0].params {
            assert_eq!(param.ty.display(), "f64");
        }
    }

    #[test]
    fn rejects_negative_scrollbar_size() {
        let source = r#"app Scrolling
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  scroll bar-width=-1.0
    text "Scrollable"
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("scroll bar width"));
    }

    #[test]
    fn checks_extended_text_input_routes_and_properties() {
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
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "str");
    }

    #[test]
    fn rejects_input_icon_options_without_an_icon() {
        let source = r#"app Form
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
view
  input "Value" <-> value icon-size=12.0
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("require `icon="));
    }

    #[test]
    fn checks_button_child_and_typed_properties() {
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
        analyze(source).unwrap();
    }

    #[test]
    fn rejects_button_label_and_child_together() {
        let source = r#"app Actions
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on pressed
view
  button "Save" -> pressed
    text "Duplicate"
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E066");
        assert!(error.message.contains("not both"));
    }

    #[test]
    fn checks_checkbox_and_toggler_typography() {
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
        analyze(source).unwrap();
    }

    #[test]
    fn checks_text_format_options_and_rejects_zero_line_height() {
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
        analyze(source).unwrap();

        let invalid = source.replace("line-height-px=20.0", "line-height=0.0");
        let error = analyze(&invalid).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("text line height"));
    }

    #[test]
    fn checks_complete_font_descriptors_and_references() {
        let source = r#"app Typography
font thin family="Inter" weight=thin stretch=ultra-condensed style=normal default=true
font extra_light family=serif weight=extra-light stretch=extra-condensed style=italic
font light family=sans weight=light stretch=condensed style=oblique
font normal family=cursive weight=normal stretch=semi-condensed style=normal
font medium family=fantasy weight=medium stretch=normal style=normal
font semibold family=mono weight=semibold stretch=semi-expanded style=normal
font bold weight=bold stretch=expanded style=normal
font extra_bold weight=extra-bold stretch=extra-expanded style=normal
font black weight=black stretch=ultra-expanded style=normal
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Fonts" font=black
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.fonts.len(), 9);

        let error = analyze(&source.replace("font=black", "font=missing")).unwrap_err();
        assert_eq!(error.code, "E114");
        assert!(error.message.contains("missing"));

        let error = analyze(&source.replace(
            "font extra_light family=serif",
            "font extra_light family=serif default=true",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E114");
        assert!(error.message.contains("only one"));
    }

    #[test]
    fn rejects_checkbox_icon_options_without_icon() {
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
  checkbox "Checkbox" checked=enabled icon-size=12.0 -> changed _
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("checkbox icon properties"));
    }

    #[test]
    fn rejects_a_utility_that_the_widget_would_ignore() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  text "hello" @gap-4
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E042");
        assert!(error.message.contains("no effect on `text`"));
    }

    #[test]
    fn names_an_undeclared_extern_type() {
        let source = r#"app Demo
extern crate::backend
  load() -> [Missing]
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  text "hello"
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E103");
        assert!(error.message.contains("`Missing`"));
    }

    #[test]
    fn requires_a_route_for_an_emitting_extern_component() {
        let source = r#"app Demo
extern crate::backend
  component native_control() -> bool
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  extern native_control()
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E126");
        assert!(error.message.contains("requires a route"));
    }

    #[test]
    fn rejects_state_capture_in_subscription_routes() {
        let source = r#"app Demo
extern crate::backend
  subscription events() -> bool
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  count = 1
on event(count, next)
subscribe
  events() -> event(count, _)
view
  text count
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E127");
    }

    #[test]
    fn checks_native_keyboard_payload_fields() {
        let source = r#"app Shortcuts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  key = ""
  typed:str? = none
  repeat = false
  command = false
on pressed(event)
  key = event.key
  typed = event.text
  repeat = event.repeat
  command = event.modifiers.command
on released(event)
  key = event.physical_key
  command = event.modifiers.jump
on modifiers_changed(modifiers)
  command = modifiers.macos_command
subscribe
  keyboard press -> pressed _
  keyboard release -> released _
  keyboard modifiers -> modifiers_changed _
view
  text key
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty.display(), "key-press");
        assert_eq!(document.handlers[1].params[0].ty.display(), "key-release");
        assert_eq!(document.handlers[2].params[0].ty.display(), "key-modifiers");

        let error = analyze(&source.replace("event.physical_key", "event.repeat")).unwrap_err();
        assert_eq!(error.code, "E151");
        assert!(error.message.contains("key-release"));
    }

    #[test]
    fn checks_native_system_tasks_and_theme_subscription() {
        let source = r#"app Diagnostics
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  cpu = ""
  memory = 0
  used:i64? = none
  mode = "none"
on inspect
  task system info -> inspected _
on inspected(info)
  cpu = info.cpu_brand
  memory = info.memory_total
  used = info.memory_used
on read_theme
  task system theme -> theme_changed _
on theme_changed(next)
  mode = next
subscribe
  system theme -> theme_changed _
view
  text cpu
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "system-info");
        assert_eq!(document.handlers[3].params[0].ty.display(), "str");

        let error = analyze(&source.replace("info.cpu_brand", "info.unknown")).unwrap_err();
        assert_eq!(error.code, "E151");
        assert!(error.message.contains("system-info"));

        let error = analyze(&source.replace(
            "task system theme -> theme_changed _",
            "task system theme -> theme_changed _ | theme_changed _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E131");
    }

    #[test]
    fn checks_native_clipboard_tasks() {
        let source = r#"app Clipboard
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  standard:str? = none
  primary:str? = none
on read
  task clipboard read -> standard_read _
on standard_read(value)
  standard = value
on read_primary
  task clipboard read-primary -> primary_read _
on primary_read(value)
  primary = value
on write
  task clipboard write "copied"
on write_primary
  task clipboard write-primary "selected"
view
  text "Clipboard"
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "str?");
        assert_eq!(document.handlers[3].params[0].ty.display(), "str?");

        let error = analyze(&source.replace(
            "task clipboard write \"copied\"",
            "task clipboard write true",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `str`"));
    }

    #[test]
    fn checks_all_static_widget_operations() {
        let source = r#"app Operations
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
  focused = false
on previous
  task widget focus-previous
on next
  task widget focus-next
on focus
  task widget focus #field
on check
  task widget focused #field -> checked _
on checked(value)
  focused = value
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
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[4].params[0].ty.display(), "bool");

        let error = analyze(&source.replace("focus #field", "focus #missing")).unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(error.message.contains("#missing"));

        let error =
            analyze(&source.replace("snap #list 0.0 1.0", "snap #list 0.0 1.1")).unwrap_err();
        assert_eq!(error.code, "E128");
    }

    #[test]
    fn rejects_events_routed_to_mount() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on mount
view
  button "Invalid" -> mount
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E135");
    }

    #[test]
    fn rejects_out_of_range_media_opacity() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  image "photo.ppm" opacity=1.5
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("opacity"));
    }
}
