use super::*;

pub(super) fn native_subscription_payloads(
    source: &SubscriptionSource,
    window_id: bool,
) -> Option<Vec<Type>> {
    let mut payloads = match source {
        SubscriptionSource::Every { .. } => vec![Type::Instant],
        SubscriptionSource::Event { .. } => vec![Type::Event],
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => Vec::new(),
            InputMethodEvent::Preedit => vec![
                Type::Str,
                Type::Option(Box::new(Type::I64)),
                Type::Option(Box::new(Type::I64)),
            ],
            InputMethodEvent::Commit => vec![Type::Str],
        },
        SubscriptionSource::Keyboard(KeyboardEvent::Press) => vec![Type::KeyPress],
        SubscriptionSource::Keyboard(KeyboardEvent::Release) => vec![Type::KeyRelease],
        SubscriptionSource::Keyboard(KeyboardEvent::Modifiers) => vec![Type::KeyModifiers],
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => Vec::new(),
            MouseEvent::Moved => vec![Type::F64, Type::F64],
            MouseEvent::Pressed | MouseEvent::Released => vec![Type::MouseButton],
            MouseEvent::Wheel => vec![Type::F64, Type::F64, Type::Bool],
        },
        SubscriptionSource::SystemTheme => vec![Type::Str],
        SubscriptionSource::Touch(_) => vec![Type::TouchFinger, Type::F64, Type::F64],
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => Vec::new(),
            WindowEvent::Opened => vec![
                Type::Option(Box::new(Type::F64)),
                Type::Option(Box::new(Type::F64)),
                Type::F64,
                Type::F64,
            ],
            WindowEvent::Moved | WindowEvent::Resized => vec![Type::F64, Type::F64],
            WindowEvent::Rescaled => vec![Type::F64],
            WindowEvent::FileHovered | WindowEvent::FileDropped => vec![Type::Str],
        },
        SubscriptionSource::Repeat { .. }
        | SubscriptionSource::Run { .. }
        | SubscriptionSource::Recipe { .. }
        | SubscriptionSource::Events { .. }
        | SubscriptionSource::Extern { .. } => return None,
    };
    if window_id {
        payloads.insert(0, Type::WindowId);
    }
    Some(payloads)
}

pub(super) fn canvas_event_name(source: &SubscriptionSource) -> Option<&'static str> {
    Some(match source {
        SubscriptionSource::InputMethod(InputMethodEvent::Opened) => "input-method opened",
        SubscriptionSource::InputMethod(InputMethodEvent::Preedit) => "input-method preedit",
        SubscriptionSource::InputMethod(InputMethodEvent::Commit) => "input-method commit",
        SubscriptionSource::InputMethod(InputMethodEvent::Closed) => "input-method closed",
        SubscriptionSource::Keyboard(KeyboardEvent::Press) => "keyboard press",
        SubscriptionSource::Keyboard(KeyboardEvent::Release) => "keyboard release",
        SubscriptionSource::Keyboard(KeyboardEvent::Modifiers) => "keyboard modifiers",
        SubscriptionSource::Mouse(MouseEvent::Entered) => "mouse entered",
        SubscriptionSource::Mouse(MouseEvent::Left) => "mouse left",
        SubscriptionSource::Mouse(MouseEvent::Moved) => "mouse moved",
        SubscriptionSource::Mouse(MouseEvent::Pressed) => "mouse pressed",
        SubscriptionSource::Mouse(MouseEvent::Released) => "mouse released",
        SubscriptionSource::Mouse(MouseEvent::Wheel) => "mouse wheel",
        SubscriptionSource::Touch(TouchEvent::Pressed) => "touch pressed",
        SubscriptionSource::Touch(TouchEvent::Moved) => "touch moved",
        SubscriptionSource::Touch(TouchEvent::Lifted) => "touch lifted",
        SubscriptionSource::Touch(TouchEvent::Lost) => "touch lost",
        SubscriptionSource::Window(WindowEvent::Frame) => "window frame",
        SubscriptionSource::Window(WindowEvent::Opened) => "window opened",
        SubscriptionSource::Window(WindowEvent::Closed) => "window closed",
        SubscriptionSource::Window(WindowEvent::Moved) => "window moved",
        SubscriptionSource::Window(WindowEvent::Resized) => "window resized",
        SubscriptionSource::Window(WindowEvent::Rescaled) => "window rescaled",
        SubscriptionSource::Window(WindowEvent::CloseRequested) => "window close-request",
        SubscriptionSource::Window(WindowEvent::Focused) => "window focused",
        SubscriptionSource::Window(WindowEvent::Unfocused) => "window unfocused",
        SubscriptionSource::Window(WindowEvent::FileHovered) => "window file-hovered",
        SubscriptionSource::Window(WindowEvent::FileDropped) => "window file-dropped",
        SubscriptionSource::Window(WindowEvent::FilesHoveredLeft) => "window files-hovered-left",
        _ => return None,
    })
}

pub(super) fn valid_canvas_cursor(value: &str) -> bool {
    matches!(
        value,
        "none"
            | "hidden"
            | "idle"
            | "context-menu"
            | "help"
            | "pointer"
            | "progress"
            | "wait"
            | "cell"
            | "crosshair"
            | "text"
            | "alias"
            | "copy"
            | "move"
            | "no-drop"
            | "not-allowed"
            | "grab"
            | "grabbing"
            | "resize-horizontal"
            | "resize-vertical"
            | "resize-diagonal-up"
            | "resize-diagonal-down"
            | "resize-column"
            | "resize-row"
            | "all-scroll"
            | "zoom-in"
            | "zoom-out"
    )
}

pub(super) fn infer_subscriptions(
    document: &Document,
    states: &HashMap<String, Type>,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
) -> Result<(), Error> {
    for subscription in &document.subscriptions {
        if let Some(condition) = &subscription.condition {
            require_type(
                &expr_type(condition, states, document, &subscription.span)?,
                &Type::Bool,
                &subscription.span,
            )?;
        }
        let mut payloads = match &subscription.source {
            SubscriptionSource::Repeat { function, .. } => {
                let source =
                    extern_function(document, function, ExternKind::Future, &subscription.span)?;
                check_call_args(source, &[], states, document, &subscription.span)?;
                vec![source.error.as_ref().map_or_else(
                    || source.output.clone(),
                    |error| Type::Result(Box::new(source.output.clone()), Box::new(error.clone())),
                )]
            }
            SubscriptionSource::Run { function, args } => {
                let source =
                    extern_function(document, function, ExternKind::Stream, &subscription.span)?;
                check_call_args(source, args, states, document, &subscription.span)?;
                for arg in args {
                    let ty = expr_type(arg, states, document, &subscription.span)?;
                    if !lazy_hashable(&ty) {
                        return Err(Error::new(
                            "E129",
                            &subscription.span,
                            format!(
                                "subscription run data must be hashable, got `{}`",
                                ty.display()
                            ),
                        ));
                    }
                }
                vec![source.error.as_ref().map_or_else(
                    || source.output.clone(),
                    |error| Type::Result(Box::new(source.output.clone()), Box::new(error.clone())),
                )]
            }
            SubscriptionSource::Recipe { function, args } => {
                let source =
                    extern_function(document, function, ExternKind::Recipe, &subscription.span)?;
                check_call_args(source, args, states, document, &subscription.span)?;
                vec![source.output.clone()]
            }
            SubscriptionSource::Events { id, filter } => {
                let source = extern_function(
                    document,
                    filter,
                    ExternKind::EventFilter,
                    &subscription.span,
                )?;
                let id = expr_type(id, states, document, &subscription.span)?;
                if !lazy_hashable(&id) {
                    return Err(Error::new(
                        "E129",
                        &subscription.span,
                        format!(
                            "raw event identity must be hashable, got `{}`",
                            id.display()
                        ),
                    ));
                }
                vec![source.output.clone()]
            }
            SubscriptionSource::Extern { function, args } => {
                let source = extern_function(
                    document,
                    function,
                    ExternKind::Subscription,
                    &subscription.span,
                )?;
                check_call_args(source, args, states, document, &subscription.span)?;
                vec![source.output.clone()]
            }
            source => native_subscription_payloads(source, subscription.window_id)
                .expect("native subscription payloads"),
        };
        if let Some(filter) = &subscription.filter {
            let function = extern_function(document, filter, ExternKind::Sync, &subscription.span)?;
            if function.params.len() != payloads.len() {
                return Err(Error::new(
                    "E142",
                    &subscription.span,
                    format!(
                        "subscription filter `{filter}` expects {} payloads, got {}",
                        function.params.len(),
                        payloads.len()
                    ),
                ));
            }
            for (actual, (_, expected)) in payloads.iter().zip(&function.params) {
                require_type(actual, expected, &subscription.span)?;
            }
            let Type::Option(output) = &function.output else {
                return Err(Error::new(
                    "E142",
                    &subscription.span,
                    format!("subscription filter `{filter}` must return an optional value"),
                ));
            };
            payloads = vec![(**output).clone()];
        }
        if let Some(context) = &subscription.context {
            let context = expr_type(context, states, document, &subscription.span)?;
            if !lazy_hashable(&context) {
                return Err(Error::new(
                    "E129",
                    &subscription.span,
                    format!(
                        "subscription context must be hashable, got `{}`",
                        context.display()
                    ),
                ));
            }
            payloads.insert(0, context);
        }
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
        if subscription.route.args.is_empty() {
            infer_route(&subscription.route, None, states, document, signatures)?;
        } else {
            infer_ordered_payload_route(
                &subscription.route,
                &payloads,
                states,
                document,
                signatures,
                "subscription",
            )?;
        }
    }
    Ok(())
}

pub(super) fn infer_runs(
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
        let nested = match statement {
            Statement::TaskGroup { statements, .. } => Some(statements.clone()),
            Statement::Abortable { task, .. } => Some(vec![(**task).clone()]),
            _ => None,
        };
        if let Some(statements) = nested {
            infer_runs(
                &Handler {
                    statements,
                    ..handler.clone()
                },
                document,
                signatures,
            )?;
            continue;
        }
        if let Statement::WidgetOperation {
            operation: WidgetOperation::Focused { .. },
            route: Some(route),
            ..
        } = statement
        {
            infer_route(route, Some(Type::Bool), &unknown_env, document, signatures)?;
        }
        if let Statement::WidgetOperation {
            operation: WidgetOperation::Find { selector, all },
            route: Some(route),
            span,
        } = statement
        {
            let output = widget_selector_output(selector, document, span)?;
            infer_route(
                route,
                Some(if *all {
                    Type::List(Box::new(output))
                } else {
                    Type::Option(Box::new(output))
                }),
                &unknown_env,
                document,
                signatures,
            )?;
        }
        if let Statement::PaneOperation {
            operation: PaneOperation::Maximized | PaneOperation::Adjacent { .. },
            route: Some(route),
            ..
        } = statement
        {
            infer_route(
                route,
                Some(Type::Option(Box::new(Type::Str))),
                &unknown_env,
                document,
                signatures,
            )?;
        }
        if let Statement::WindowOperation {
            operation,
            route: Some(route),
            span,
            ..
        } = statement
        {
            match operation {
                WindowOperation::Open(_) => infer_route(
                    route,
                    Some(Type::WindowId),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                WindowOperation::Oldest | WindowOperation::Latest => infer_route(
                    route,
                    Some(Type::Option(Box::new(Type::WindowId))),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                WindowOperation::RawId => {
                    infer_route(route, Some(Type::Str), &unknown_env, document, signatures)?
                }
                WindowOperation::Screenshot => infer_ordered_payload_route(
                    route,
                    &[Type::Bytes, Type::I64, Type::I64, Type::F64],
                    &unknown_env,
                    document,
                    signatures,
                    "window screenshot",
                )?,
                WindowOperation::Size => infer_ordered_payload_route(
                    route,
                    &[Type::F64, Type::F64],
                    &unknown_env,
                    document,
                    signatures,
                    "window size",
                )?,
                WindowOperation::Position | WindowOperation::MonitorSize => {
                    infer_ordered_payload_route(
                        route,
                        &[
                            Type::Option(Box::new(Type::F64)),
                            Type::Option(Box::new(Type::F64)),
                        ],
                        &unknown_env,
                        document,
                        signatures,
                        "optional window coordinates",
                    )?
                }
                WindowOperation::IsMaximized => {
                    infer_route(route, Some(Type::Bool), &unknown_env, document, signatures)?
                }
                WindowOperation::IsMinimized => infer_route(
                    route,
                    Some(Type::Option(Box::new(Type::Bool))),
                    &unknown_env,
                    document,
                    signatures,
                )?,
                WindowOperation::ScaleFactor => {
                    infer_route(route, Some(Type::F64), &unknown_env, document, signatures)?
                }
                WindowOperation::Mode => {
                    infer_route(route, Some(Type::Str), &unknown_env, document, signatures)?
                }
                WindowOperation::Callback { function, .. } => {
                    let callback = extern_function(document, function, ExternKind::Window, span)?;
                    infer_route(
                        route,
                        Some(callback.output.clone()),
                        &unknown_env,
                        document,
                        signatures,
                    )?
                }
                _ => {}
            }
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
            if *kind == EffectKind::Stream
                && std::iter::once(success).chain(error.iter()).any(|route| {
                    route.args.len() > 1
                        || route
                            .args
                            .iter()
                            .any(|arg| !matches!(arg, RouteArg::Payload))
                })
            {
                return Err(Error::new(
                    "E127",
                    span,
                    "stream routes accept at most one `_`; read other state in the handler",
                ));
            }
            if let Some(output) = builtin_task_output(*kind, function, args, span)? {
                infer_route(success, Some(output), &unknown_env, document, signatures)?;
                if error.is_some() {
                    return Err(Error::new(
                        "E131",
                        span,
                        "built-in tasks are infallible and cannot have an error route",
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
        if let Statement::Sip {
            function,
            progress,
            success,
            error,
            span,
            ..
        } = statement
        {
            if std::iter::once(progress)
                .chain(std::iter::once(success))
                .chain(error.iter())
                .any(|route| {
                    route.args.len() > 1
                        || route
                            .args
                            .iter()
                            .any(|arg| !matches!(arg, RouteArg::Payload))
                })
            {
                return Err(Error::new(
                    "E127",
                    span,
                    "sip routes accept at most one `_`; read other state in the handler",
                ));
            }
            let action = extern_function(document, function, ExternKind::Sip, span)?;
            let progress_ty = action
                .progress
                .clone()
                .expect("sip extern has a progress type");
            infer_route(
                progress,
                Some(progress_ty),
                &unknown_env,
                document,
                signatures,
            )?;
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
                        "fallible extern sip requires an error route",
                    ));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "infallible extern sip cannot have an error route",
                    ));
                }
                (None, None) => {}
            }
        }
        if let Statement::TaskFlow {
            source,
            transforms,
            success,
            error,
            units,
            span,
        } = statement
        {
            if success
                .iter()
                .chain(error.iter())
                .chain(units.iter())
                .any(|route| {
                    route.args.len() > 1
                        || route
                            .args
                            .iter()
                            .any(|arg| !matches!(arg, RouteArg::Payload))
                })
            {
                return Err(Error::new(
                    "E127",
                    span,
                    "flow routes accept at most one `_`; read other state in the handler",
                ));
            }
            let mut flow_env = document
                .states
                .iter()
                .map(|state| (state.name.clone(), state.ty.clone()))
                .collect::<HashMap<_, _>>();
            flow_env.extend(unknown_env.clone());
            let (output, error_ty) = task_flow_type(source, transforms, document, &flow_env)?;
            if let Some(route) = units {
                infer_route(route, Some(Type::I64), &unknown_env, document, signatures)?;
            }
            match (output, success) {
                (Some(output), Some(route)) => {
                    infer_route(route, Some(output), &unknown_env, document, signatures)?
                }
                (Some(_), None) => {
                    return Err(Error::new("E131", span, "flow requires a done route"));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "discarded flow cannot have a done route",
                    ));
                }
                (None, None) => {}
            }
            match (error_ty, error) {
                (Some(error_ty), Some(route)) => {
                    infer_route(route, Some(error_ty), &unknown_env, document, signatures)?
                }
                (Some(_), None) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "fallible flow requires an error route",
                    ));
                }
                (None, Some(_)) => {
                    return Err(Error::new(
                        "E131",
                        span,
                        "infallible or discarded flow cannot have an error route",
                    ));
                }
                (None, None) => {}
            }
        }
    }
    Ok(())
}

pub(super) fn infer_route(
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

pub(super) fn infer_ordered_payload_route(
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
