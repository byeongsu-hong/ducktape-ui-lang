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
        if actual != Type::Unknown && !compatible(&state.ty, &actual) {
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
        Type::List(inner) => check_declared_type(inner, span, known),
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
            id,
            styles,
            children,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            check_styles(styles, document, span, StyleTarget::Layout(*kind))?;
            for child in children {
                infer_view(child, env, document, signatures, ids)?;
            }
        }
        ViewNode::Text {
            value,
            styles,
            span,
        } => {
            let ty = expr_type(value, env, document, span)?;
            if !matches!(ty, Type::Str | Type::I64 | Type::F64) {
                return Err(type_error(span, &Type::Str, &ty).hint("text accepts str, i64, or f64"));
            }
            check_styles(styles, document, span, StyleTarget::Text)?;
        }
        ViewNode::Input {
            id,
            binding,
            disabled,
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
            check_styles(styles, document, span, StyleTarget::Input)?;
        }
        ViewNode::Button {
            id,
            disabled,
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
            infer_route(route, None, env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Button)?;
        }
        ViewNode::Checkbox {
            label,
            id,
            checked,
            disabled,
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
            infer_route(route, Some(Type::Bool), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Checkbox)?;
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
            function,
            success,
            error,
            span,
            ..
        } = statement
        {
            let action = document
                .functions
                .iter()
                .find(|item| item.name == *function)
                .ok_or_else(|| {
                    Error::new("E130", span, format!("unknown extern fn `{function}`"))
                })?;
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
                let action = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function)
                    .ok_or_else(|| {
                        Error::new("E130", span, format!("unknown extern fn `{function}`"))
                    })?;
                if args.len() != action.params.len() {
                    return Err(Error::new(
                        "E142",
                        span,
                        format!(
                            "extern fn `{function}` expects {} arguments, got {}",
                            action.params.len(),
                            args.len()
                        ),
                    ));
                }
                for (arg, (_, expected)) in args.iter().zip(&action.params) {
                    let actual = expr_type(arg, &env, document, span)?;
                    require_type(&actual, expected, span)?;
                }
            }
        }
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
        Expr::EmptyList => Ok(Type::Unknown),
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
    let is_flow = matches!(target, StyleTarget::Layout(Layout::Column | Layout::Row));
    let target_name = match target {
        StyleTarget::Layout(Layout::Column) => "col",
        StyleTarget::Layout(Layout::Row) => "row",
        StyleTarget::Layout(Layout::Scroll) => "scroll",
        StyleTarget::Text => "text",
        StyleTarget::Input => "input",
        StyleTarget::Button => "button",
        StyleTarget::Checkbox => "checkbox",
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
                "max-w-sm" | "max-w-md" | "max-w-lg" | "max-w-xl" | "max-w-2xl"
                | "items-center" | "self-center" => is_flow,
                "text-xs" | "text-sm" | "text-base" | "text-lg" | "text-xl" | "text-2xl"
                | "font-bold" => matches!(target, StyleTarget::Text),
                "border" | "border-2" => is_flow || matches!(target, StyleTarget::Input),
                "rounded-sm" | "rounded" | "rounded-md" | "rounded-lg" | "rounded-full" => {
                    is_flow || matches!(target, StyleTarget::Input | StyleTarget::Button)
                }
                _ if utility.starts_with("gap-") => is_flow,
                _ if utility.starts_with("p-")
                    || utility.starts_with("px-")
                    || utility.starts_with("py-") =>
                {
                    is_flow || matches!(target, StyleTarget::Input | StyleTarget::Button)
                }
                _ if utility.starts_with("bg-") => {
                    is_flow || matches!(target, StyleTarget::Input | StyleTarget::Button)
                }
                _ if utility.starts_with("text-") => {
                    is_flow || matches!(target, StyleTarget::Text | StyleTarget::Button)
                }
                _ if utility.starts_with("border-") => {
                    is_flow || matches!(target, StyleTarget::Input)
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
    if (is_flow || matches!(target, StyleTarget::Input)) && has_border_color && !has_border {
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
    if is_flow && has_radius && !has_background && !has_border {
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
    left == right || *left == Type::Unknown || *right == Type::Unknown
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
}
