use super::*;

pub(in crate::check) fn infer_components_group(
    node: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
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
        _ => return Ok(false),
    };
    Ok(true)
}
