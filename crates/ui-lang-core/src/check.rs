use crate::Error;
use crate::ast::*;
use std::collections::{HashMap, HashSet};

pub fn check(document: &mut Document) -> Result<(), Error> {
    check_unique(document)?;
    check_declared_types(document)?;
    check_theme(document)?;

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
        } else if actual != Type::Unknown && !compatible(&state.ty, &actual) {
            return Err(type_error(&state.span, &state.ty, &actual));
        }
    }

    let mut signatures: HashMap<String, Vec<Option<Type>>> = document
        .handlers
        .iter()
        .map(|handler| (handler.name.clone(), vec![None; handler.params.len()]))
        .collect();

    let mut ids = HashSet::new();
    infer_view(&document.view, &states, document, &mut signatures, &mut ids)?;
    for component in &document.components {
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
        check_handler(handler, &states, document)?;
    }
    Ok(())
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
    let mut states = HashSet::new();
    for state in &document.states {
        if !states.insert(&state.name) {
            return Err(Error::new(
                "E100",
                &state.span,
                format!("duplicate state `{}`", state.name),
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
            if let Some(clip) = &options.clip {
                require_type(&expr_type(clip, env, document, span)?, &Type::Bool, span)?;
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
            styles,
            route,
            release,
            span,
            ..
        } => {
            for expr in [value, min, max, step] {
                require_type(&expr_type(expr, env, document, span)?, &Type::F64, span)?;
            }
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
            styles,
            span,
            ..
        } => {
            for expr in [value, min, max] {
                require_type(&expr_type(expr, env, document, span)?, &Type::F64, span)?;
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
            styles,
            span,
            ..
        } => {
            require_type(
                &expr_type(thickness, env, document, span)?,
                &Type::F64,
                span,
            )?;
            check_styles(styles, document, span, StyleTarget::Rule)?;
        }
        ViewNode::Space {
            width,
            height,
            styles,
            span,
        } => {
            for size in [width, height].into_iter().flatten() {
                require_type(&expr_type(size, env, document, span)?, &Type::F64, span)?;
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
        ViewNode::Component {
            name,
            args,
            id,
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
        }
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
            breakpoint,
            width,
            height,
            narrow,
            wide,
            span,
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
            for length in [width, height].into_iter().flatten() {
                if let LengthValue::Fixed(value) = length {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, "responsive size", span)?;
                }
            }
            infer_view(narrow, env, document, signatures, ids)?;
            infer_view(wide, env, document, signatures, ids)?;
        }
    }
    Ok(())
}

fn require_literal_range(
    expr: &Expr,
    min: f64,
    max: Option<f64>,
    label: &str,
    span: &Span,
) -> Result<(), Error> {
    let literal = match expr {
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
    };
    if literal.is_some_and(|value| value < min || max.is_some_and(|max| value > max)) {
        return Err(Error::new(
            "E128",
            span,
            format!("{label} is outside its valid range"),
        ));
    }
    Ok(())
}

fn check_bool_control_options(
    options: &BoolControlOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
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

fn check_text_options(
    options: &TextOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
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
        let source = extern_function(
            document,
            &subscription.function,
            ExternKind::Subscription,
            &subscription.span,
        )?;
        check_call_args(
            source,
            &subscription.args,
            states,
            document,
            &subscription.span,
        )?;
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
            Some(source.output.clone()),
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
        if let Statement::Run {
            kind,
            function,
            success,
            error,
            span,
            ..
        } = statement
        {
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
                let action = extern_function(document, function, (*kind).into(), span)?;
                check_call_args(action, args, &env, document, span)?;
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
                let Type::Named(name) = &ty else {
                    return Err(Error::new(
                        "E151",
                        span,
                        format!("type `{}` has no field `{field}`", ty.display()),
                    ));
                };
                let item = document
                    .structs
                    .iter()
                    .find(|item| item.name == *name)
                    .ok_or_else(|| {
                        Error::new("E151", span, format!("unknown extern struct `{name}`"))
                    })?;
                ty = item
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
                    })?;
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
        let valid_color = color.is_some_and(|value| {
            let (name, opacity) = value
                .split_once('/')
                .map_or((value, None), |(name, opacity)| (name, Some(opacity)));
            let known = ["white", "black", "transparent"].contains(&name)
                || document.theme.contains_key(name);
            known
                && opacity
                    .is_none_or(|opacity| opacity.parse::<u8>().is_ok_and(|opacity| opacity <= 100))
        });
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
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty.display(), "f64");
        assert_eq!(document.handlers[0].params[1].ty.display(), "f64");
        assert_eq!(document.handlers[1].params[0].ty.display(), "f64");
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
