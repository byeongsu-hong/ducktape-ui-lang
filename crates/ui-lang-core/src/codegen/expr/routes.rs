use super::*;

pub(in crate::codegen) fn widget_target_field_type(field: &str) -> Option<Type> {
    match field {
        "kind" => Some(Type::Str),
        "id" => Some(Type::Option(Box::new(Type::WidgetId))),
        "x" | "y" | "width" | "height" => Some(Type::F64),
        "visible_x" | "visible_y" | "visible_width" | "visible_height" | "content_x"
        | "content_y" | "content_width" | "content_height" | "translation_x" | "translation_y" => {
            Some(Type::Option(Box::new(Type::F64)))
        }
        "content" => Some(Type::Option(Box::new(Type::Str))),
        _ => None,
    }
}

pub(in crate::codegen) fn u32_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "({}).clamp(0, u32::MAX as i64) as u32",
        expr_code(expr, env, document, ValueMode::Owned)?
    ))
}

pub(in crate::codegen) fn route_code(
    route: &Route,
    payload: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    if route.args.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok(payload.into()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}

pub(in crate::codegen) fn size_route_code(
    route: &Route,
    size: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    ordered_route_code(
        route,
        &[
            &format!("{size}.width as f64"),
            &format!("{size}.height as f64"),
        ],
        env,
        document,
        message,
    )
}

pub(in crate::codegen) fn ordered_route_code(
    route: &Route,
    payloads: &[&str],
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    if route.args.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let mut payload = payloads.iter();
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok((*payload.next().expect("checked payload count")).to_owned()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}
