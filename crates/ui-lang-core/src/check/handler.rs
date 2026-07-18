use super::*;

pub(super) fn check_handler(
    handler: &Handler,
    states: &HashMap<String, Type>,
    document: &Document,
    operation_ids: &[WidgetIdPath],
    pane_grids: &HashMap<String, HashSet<String>>,
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
                if let Type::Combo(inner) = expected {
                    require_type(&actual, &Type::List(inner.clone()), span)?;
                } else {
                    require_type(&actual, expected, span)?;
                }
            }
            Statement::MarkdownAppend {
                target,
                value,
                span,
            } => {
                let expected = states.get(target).ok_or_else(|| {
                    Error::new("E140", span, format!("unknown markdown state `{target}`"))
                })?;
                require_type(expected, &Type::Markdown, span)?;
                require_type(&expr_type(value, &env, document, span)?, &Type::Str, span)?;
            }
            Statement::ComboPush {
                target,
                value,
                span,
            } => {
                let actual = states.get(target).ok_or_else(|| {
                    Error::new("E140", span, format!("unknown combo state `{target}`"))
                })?;
                let Type::Combo(expected) = actual else {
                    return Err(Error::new(
                        "E140",
                        span,
                        format!("`{target}` is not combo state"),
                    ));
                };
                require_type(&expr_type(value, &env, document, span)?, expected, span)?;
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
                    let effect = match kind {
                        EffectKind::Future => "run",
                        EffectKind::Task => "task",
                        EffectKind::Stream => "stream",
                    };
                    return Err(Error::new(
                        "E141",
                        span,
                        format!("{effect} must be the final statement in a handler"),
                    ));
                }
                if builtin_task_output(*kind, function, args, span)?.is_some() {
                    if function == "__ice_font_load" {
                        require_type(
                            &expr_type(&args[0], &env, document, span)?,
                            &Type::Bytes,
                            span,
                        )?;
                    }
                    continue;
                }
                let action = extern_function(document, function, (*kind).into(), span)?;
                check_call_args(action, args, &env, document, span)?;
            }
            Statement::Sip {
                function,
                args,
                span,
                ..
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "sip must be the final statement in a handler",
                    ));
                }
                let action = extern_function(document, function, ExternKind::Sip, span)?;
                check_call_args(action, args, &env, document, span)?;
            }
            Statement::TaskFlow {
                source,
                transforms,
                span,
                ..
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "flow must be the final statement in a handler",
                    ));
                }
                task_flow_type(source, transforms, document, &env)?;
            }
            Statement::TaskGroup { statements, .. } => {
                for statement in statements {
                    check_handler(
                        &Handler {
                            statements: vec![statement.clone()],
                            ..handler.clone()
                        },
                        states,
                        document,
                        operation_ids,
                        pane_grids,
                    )?;
                }
            }
            Statement::Abortable {
                handle, task, span, ..
            } => {
                require_task_handle_state(handle, states, span)?;
                check_handler(
                    &Handler {
                        statements: vec![(**task).clone()],
                        ..handler.clone()
                    },
                    states,
                    document,
                    operation_ids,
                    pane_grids,
                )?;
            }
            Statement::Abort { handle, span } => {
                require_task_handle_state(handle, states, span)?;
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
                    WidgetOperation::FocusPrevious
                    | WidgetOperation::FocusNext
                    | WidgetOperation::Find { .. } => None,
                    WidgetOperation::Focus { target }
                    | WidgetOperation::Focused { target }
                    | WidgetOperation::CursorFront { target }
                    | WidgetOperation::CursorEnd { target }
                    | WidgetOperation::Cursor { target, .. }
                    | WidgetOperation::SelectAll { target }
                    | WidgetOperation::Select { target, .. }
                    | WidgetOperation::Snap { target, .. }
                    | WidgetOperation::SnapEnd { target }
                    | WidgetOperation::ScrollTo { target, .. }
                    | WidgetOperation::ScrollBy { target, .. } => Some(target),
                };
                if let Some(target) = target {
                    check_widget_target(target, &env, document, operation_ids, span)?;
                }
                if let WidgetOperation::Find { selector, .. } = operation {
                    check_widget_selector(selector, &env, document, operation_ids, span)?;
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
                    (WidgetOperation::Find { .. }, None) => {
                        return Err(Error::new(
                            "E172",
                            span,
                            "widget selector requires `-> handler _`",
                        ));
                    }
                    (WidgetOperation::Find { .. }, Some(_)) => {}
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
            Statement::PaneOperation {
                grid,
                operation,
                route,
                span,
            } => {
                let panes = pane_grids.get(grid).ok_or_else(|| {
                    Error::new("E188", span, format!("unknown pane-grid `#{grid}`"))
                })?;
                let referenced = match operation {
                    PaneOperation::Maximize { pane }
                    | PaneOperation::Adjacent { pane, .. }
                    | PaneOperation::Close { pane }
                    | PaneOperation::Move { pane, .. } => vec![pane],
                    PaneOperation::Swap { first, second } => vec![first, second],
                    PaneOperation::Drop { pane, target, .. } => vec![pane, target],
                    PaneOperation::Split { target, pane, .. } => vec![target, pane],
                    PaneOperation::Restore
                    | PaneOperation::Maximized
                    | PaneOperation::Resize { .. } => Vec::new(),
                };
                for pane in referenced {
                    if !panes.contains(pane) {
                        return Err(Error::new(
                            "E188",
                            span,
                            format!("pane-grid `#{grid}` has no pane `{pane}`"),
                        ));
                    }
                }
                if matches!(
                    operation,
                    PaneOperation::Swap { first, second } if first == second
                ) || matches!(
                    operation,
                    PaneOperation::Drop { pane, target, .. } if pane == target
                ) || matches!(
                    operation,
                    PaneOperation::Split { target, pane, .. } if target == pane
                ) {
                    return Err(Error::new(
                        "E188",
                        span,
                        "pane operation requires two different panes",
                    ));
                }
                let query = matches!(
                    operation,
                    PaneOperation::Maximized | PaneOperation::Adjacent { .. }
                );
                if query && index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E188",
                        span,
                        "pane query must be the final statement in a handler",
                    ));
                }
                match (query, route) {
                    (true, None) => {
                        return Err(Error::new("E188", span, "pane query requires a route"));
                    }
                    (false, Some(_)) => {
                        return Err(Error::new(
                            "E188",
                            span,
                            "pane effects do not produce a route",
                        ));
                    }
                    _ => {}
                }
                if let PaneOperation::Resize { ratio } | PaneOperation::Split { ratio, .. } =
                    operation
                {
                    require_type(&expr_type(ratio, &env, document, span)?, &Type::F64, span)?;
                    require_literal_range(ratio, 0.0, Some(1.0), "pane split ratio", span)?;
                }
            }
            Statement::WindowOperation {
                operation,
                target,
                route,
                span,
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E173",
                        span,
                        "window task must be the final statement in a handler",
                    ));
                }
                let query = matches!(
                    operation,
                    WindowOperation::Open(_)
                        | WindowOperation::Oldest
                        | WindowOperation::Latest
                        | WindowOperation::Size
                        | WindowOperation::IsMaximized
                        | WindowOperation::IsMinimized
                        | WindowOperation::Position
                        | WindowOperation::ScaleFactor
                        | WindowOperation::Mode
                        | WindowOperation::RawId
                        | WindowOperation::Screenshot
                        | WindowOperation::MonitorSize
                        | WindowOperation::Callback { .. }
                );
                if let WindowOperation::Open(Some(name)) = operation
                    && !document
                        .settings
                        .windows
                        .iter()
                        .any(|window| window.name == *name)
                {
                    return Err(Error::new(
                        "E173",
                        span,
                        format!("unknown app window `{name}`"),
                    ));
                }
                if let Some(target) = target {
                    if matches!(
                        operation,
                        WindowOperation::Open(_)
                            | WindowOperation::Oldest
                            | WindowOperation::Latest
                            | WindowOperation::AutomaticTabbing(_)
                    ) {
                        return Err(Error::new(
                            "E173",
                            span,
                            "this window task does not accept `target=`",
                        ));
                    }
                    require_type(
                        &expr_type(target, &env, document, span)?,
                        &Type::WindowId,
                        span,
                    )?;
                }
                match (query, route) {
                    (true, None) => {
                        return Err(Error::new("E173", span, "window query requires a route"));
                    }
                    (false, Some(_)) => {
                        return Err(Error::new(
                            "E173",
                            span,
                            "window effects do not produce a route",
                        ));
                    }
                    _ => {}
                }
                for value in match operation {
                    WindowOperation::Resizable(value)
                    | WindowOperation::Maximize(value)
                    | WindowOperation::Minimize(value)
                    | WindowOperation::MousePassthrough(value)
                    | WindowOperation::AutomaticTabbing(value) => vec![value],
                    _ => Vec::new(),
                } {
                    require_type(&expr_type(value, &env, document, span)?, &Type::Bool, span)?;
                }
                for value in match operation {
                    WindowOperation::Resize(width, height) => vec![width, height],
                    WindowOperation::MinSize(Some((width, height)))
                    | WindowOperation::MaxSize(Some((width, height)))
                    | WindowOperation::ResizeIncrements(Some((width, height))) => {
                        vec![width, height]
                    }
                    _ => Vec::new(),
                } {
                    require_type(&expr_type(value, &env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, f64::EPSILON, None, "window size", span)?;
                }
                if let WindowOperation::Move(x, y) = operation {
                    require_type(&expr_type(x, &env, document, span)?, &Type::F64, span)?;
                    require_type(&expr_type(y, &env, document, span)?, &Type::F64, span)?;
                }
                if let WindowOperation::Icon {
                    pixels,
                    width,
                    height,
                } = operation
                {
                    require_type(
                        &expr_type(pixels, &env, document, span)?,
                        &Type::Bytes,
                        span,
                    )?;
                    for (dimension, label) in
                        [(width, "window icon width"), (height, "window icon height")]
                    {
                        require_type(
                            &expr_type(dimension, &env, document, span)?,
                            &Type::I64,
                            span,
                        )?;
                        require_literal_range(dimension, 1.0, Some(u32::MAX as f64), label, span)?;
                    }
                    if let (Expr::I64(width), Expr::I64(height)) = (width, height)
                        && (*width as u128) * (*height as u128) > u32::MAX as u128
                    {
                        return Err(Error::new(
                            "E173",
                            span,
                            "window icon dimensions are too large",
                        ));
                    }
                    if let (Expr::Bytes(pixels), Expr::I64(width), Expr::I64(height)) =
                        (pixels, width, height)
                    {
                        let expected = (*width as u128) * (*height as u128) * 4;
                        if expected != pixels.len() as u128 {
                            return Err(Error::new(
                                "E173",
                                span,
                                "window icon pixels must contain width × height × 4 bytes",
                            ));
                        }
                    }
                }
                if let WindowOperation::Callback { function, args } = operation {
                    let callback = extern_function(document, function, ExternKind::Window, span)?;
                    check_call_args(callback, args, &env, document, span)?;
                }
            }
        }
    }
    Ok(())
}

pub(super) fn check_structured_tasks(handler: &Handler) -> Result<(), Error> {
    for (index, statement) in handler.statements.iter().enumerate() {
        match statement {
            Statement::TaskGroup {
                statements, span, ..
            } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "task group must be the final statement in a handler",
                    ));
                }
                if let Some(span) = statements.iter().find_map(invalid_task_producer) {
                    return Err(Error::new(
                        "E143",
                        span,
                        "task groups only accept task-producing statements",
                    ));
                }
            }
            Statement::Abortable { task, span, .. } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "abortable task must be the final statement in a handler",
                    ));
                }
                if let Some(span) = invalid_task_producer(task) {
                    return Err(Error::new(
                        "E143",
                        span,
                        "abortable requires a task-producing statement",
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn invalid_task_producer(statement: &Statement) -> Option<&Span> {
    match statement {
        Statement::Run { .. }
        | Statement::Sip { .. }
        | Statement::TaskFlow { .. }
        | Statement::ClipboardWrite { .. }
        | Statement::WidgetOperation { .. }
        | Statement::WindowOperation { .. }
        | Statement::PaneOperation {
            operation: PaneOperation::Maximized | PaneOperation::Adjacent { .. },
            ..
        } => None,
        Statement::Abortable { task, .. } => invalid_task_producer(task),
        Statement::TaskGroup { statements, .. } => {
            statements.iter().find_map(invalid_task_producer)
        }
        Statement::Assign { .. }
        | Statement::MarkdownAppend { .. }
        | Statement::ComboPush { .. }
        | Statement::ReturnIf { .. }
        | Statement::Abort { .. }
        | Statement::PaneOperation { .. } => Some(statement_span(statement)),
    }
}

pub(super) fn require_task_handle_state(
    handle: &str,
    states: &HashMap<String, Type>,
    span: &Span,
) -> Result<(), Error> {
    let Some(actual) = states.get(handle) else {
        return Err(Error::new(
            "E143",
            span,
            format!("unknown task handle state `{handle}`"),
        )
        .hint(format!("declare `{handle}:task-handle? = none` in state")));
    };
    require_type(actual, &Type::Option(Box::new(Type::TaskHandle)), span)
}

pub(super) fn statement_span(statement: &Statement) -> &Span {
    match statement {
        Statement::Assign { span, .. }
        | Statement::MarkdownAppend { span, .. }
        | Statement::ComboPush { span, .. }
        | Statement::ReturnIf { span, .. }
        | Statement::Run { span, .. }
        | Statement::Sip { span, .. }
        | Statement::TaskFlow { span, .. }
        | Statement::TaskGroup { span, .. }
        | Statement::Abortable { span, .. }
        | Statement::Abort { span, .. }
        | Statement::ClipboardWrite { span, .. }
        | Statement::WidgetOperation { span, .. }
        | Statement::WindowOperation { span, .. }
        | Statement::PaneOperation { span, .. } => span,
    }
}

impl From<EffectKind> for ExternKind {
    fn from(value: EffectKind) -> Self {
        match value {
            EffectKind::Future => Self::Future,
            EffectKind::Task => Self::Task,
            EffectKind::Stream => Self::Stream,
        }
    }
}

pub(super) fn extern_function<'a>(
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
                ExternKind::Shader => "shader",
                ExternKind::Task => "task",
                ExternKind::Stream => "stream",
                ExternKind::Sip => "sip",
                ExternKind::Recipe => "recipe",
                ExternKind::Selector => "selector",
                ExternKind::EventFilter => "event filter",
                ExternKind::Sync => "sync function",
                ExternKind::Subscription => "subscription",
                ExternKind::Window => "window callback",
                ExternKind::MarkdownViewer => "markdown viewer",
                ExternKind::EditorBinding => "editor binding",
                ExternKind::EditorHighlighter => "editor highlighter",
                ExternKind::EditorStyle => "editor style",
                ExternKind::TextStyle => "text style",
                ExternKind::SliderStyle => "slider style",
                ExternKind::ProgressStyle => "progress style",
                ExternKind::ButtonStyle => "button style",
                ExternKind::CheckboxStyle => "checkbox style",
                ExternKind::TogglerStyle => "toggler style",
                ExternKind::RadioStyle => "radio style",
                ExternKind::ContainerStyle => "container style",
                ExternKind::SvgStyle => "svg style",
                ExternKind::InputStyle => "input style",
                ExternKind::ScrollStyle => "scroll style",
                ExternKind::PickListStyle => "pick-list style",
                ExternKind::MenuStyle => "menu style",
            };
            Error::new("E130", span, format!("unknown extern {label} `{name}`"))
        })
}

pub(super) fn check_call_args(
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

pub(super) fn builtin_task_output(
    kind: EffectKind,
    function: &str,
    args: &[Expr],
    span: &Span,
) -> Result<Option<Type>, Error> {
    let output = match (kind, function) {
        (EffectKind::Task, "__ice_system_info") => Some(Type::SystemInfo),
        (EffectKind::Task, "__ice_system_theme") => Some(Type::Str),
        (EffectKind::Task, "__ice_time_now") => Some(Type::Instant),
        (EffectKind::Task, "__ice_clipboard_read" | "__ice_clipboard_read_primary") => {
            Some(Type::Option(Box::new(Type::Str)))
        }
        (EffectKind::Task, "__ice_font_load") => Some(Type::Unit),
        _ => None,
    };
    if function == "__ice_font_load" && args.len() != 1 {
        return Err(Error::new("E142", span, "font load expects one argument"));
    }
    if output.is_some() && function != "__ice_font_load" && !args.is_empty() {
        return Err(Error::new(
            "E142",
            span,
            "this built-in task takes no arguments",
        ));
    }
    Ok(output)
}

pub(super) fn task_source_type(
    source: &TaskSource,
    document: &Document,
    env: &HashMap<String, Type>,
) -> Result<(Type, Option<Type>), Error> {
    match source {
        TaskSource::Done { value, span } => Ok((expr_type(value, env, document, span)?, None)),
        TaskSource::None { output, span } => {
            let known = document
                .structs
                .iter()
                .map(|item| item.name.as_str())
                .collect::<HashSet<_>>();
            check_declared_type(output, span, &known)?;
            Ok((output.clone(), None))
        }
        TaskSource::Effect {
            kind,
            function,
            args,
            span,
        } => {
            if let Some(output) = builtin_task_output(*kind, function, args, span)? {
                if function == "__ice_font_load" {
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Bytes,
                        span,
                    )?;
                }
                return Ok((output, None));
            }
            let action = extern_function(document, function, (*kind).into(), span)?;
            check_call_args(action, args, env, document, span)?;
            Ok((action.output.clone(), action.error.clone()))
        }
    }
}

pub(crate) fn task_flow_type(
    source: &TaskSource,
    transforms: &[TaskTransform],
    document: &Document,
    root_env: &HashMap<String, Type>,
) -> Result<(Option<Type>, Option<Type>), Error> {
    let (mut output, mut error_ty) = task_source_type(source, document, root_env)?;
    for (index, transform) in transforms.iter().enumerate() {
        match transform {
            TaskTransform::Then {
                binding,
                source,
                span,
            } => {
                if error_ty.is_some() {
                    return Err(Error::new(
                        "E144",
                        span,
                        "then cannot unwrap a fallible task; use and-then",
                    ));
                }
                let env = HashMap::from([(binding.clone(), output)]);
                let next = task_source_type(source, document, &env).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!(
                            "a flow transform may only read its `{binding}` binding"
                        ))
                    } else {
                        error
                    }
                })?;
                output = next.0;
                error_ty = next.1;
            }
            TaskTransform::AndThen {
                binding,
                source,
                span,
            } => {
                let (binding_ty, result_error) = if let Some(error) = &error_ty {
                    (output.clone(), Some(error.clone()))
                } else if let Type::Option(inner) = &output {
                    ((**inner).clone(), None)
                } else {
                    return Err(Error::new(
                        "E144",
                        span,
                        "and-then requires an optional or fallible task output",
                    ));
                };
                let env = HashMap::from([(binding.clone(), binding_ty)]);
                let next = task_source_type(source, document, &env).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!(
                            "a flow transform may only read its `{binding}` binding"
                        ))
                    } else {
                        error
                    }
                })?;
                if let Some(expected) = result_error {
                    let Some(actual) = &next.1 else {
                        return Err(Error::new(
                            "E144",
                            span,
                            "and-then after a fallible task must return a fallible task",
                        ));
                    };
                    require_type(actual, &expected, span)?;
                    error_ty = Some(expected);
                } else {
                    error_ty = next.1;
                }
                output = next.0;
            }
            TaskTransform::MapError {
                binding,
                value,
                span,
            } => {
                let Some(error) = error_ty.take() else {
                    return Err(Error::new(
                        "E144",
                        span,
                        "map-error requires a fallible flow",
                    ));
                };
                let env = HashMap::from([(binding.clone(), error)]);
                let mapped = expr_type(value, &env, document, span).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!("map-error may only read its `{binding}` binding"))
                    } else {
                        error
                    }
                })?;
                error_ty = Some(mapped);
            }
            TaskTransform::Collect { .. } => {
                let item = match error_ty.take() {
                    Some(error) => Type::Result(Box::new(output), Box::new(error)),
                    None => output,
                };
                output = Type::List(Box::new(item));
            }
            TaskTransform::Discard { span } => {
                if index + 1 != transforms.len() {
                    return Err(Error::new(
                        "E144",
                        span,
                        "discard must be the final flow transform",
                    ));
                }
                return Ok((None, None));
            }
        }
    }
    Ok((Some(output), error_ty))
}
