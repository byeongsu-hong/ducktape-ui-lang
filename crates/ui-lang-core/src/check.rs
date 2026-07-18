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
    if let Some(span) = repeated_pane_grid_span(&document.view) {
        return Err(Error::new(
            "E187",
            span,
            "pane-grid cannot be repeated because each static ID owns one persistent layout state",
        ));
    }

    let states: HashMap<String, Type> = document
        .states
        .iter()
        .map(|state| (state.name.clone(), state.ty.clone()))
        .collect();
    let preset_handlers = document
        .presets
        .iter()
        .map(|preset| Handler {
            name: format!("preset_{}", preset.name),
            params: Vec::new(),
            statements: preset.statements.clone(),
            span: preset.span.clone(),
        })
        .collect::<Vec<_>>();
    for state in &document.states {
        let actual = expr_type(&state.initial, &HashMap::new(), document, &state.span)?;
        if state.ty == Type::Option(Box::new(Type::DebugSpan))
            && !matches!(state.initial, Expr::None)
        {
            return Err(Error::new(
                "E103",
                &state.span,
                "debug span state must start as `none`",
            ));
        } else if let Type::Combo(expected) = &state.ty {
            let Type::List(actual) = actual else {
                return Err(Error::new(
                    "E104",
                    &state.span,
                    "combo state must be initialized with a list",
                ));
            };
            require_type(&actual, expected, &state.span)?;
        } else if let Type::Animation(expected) = &state.ty {
            require_type(&actual, expected, &state.span)?;
            if let Some(easing) = state
                .animation
                .as_ref()
                .and_then(|options| options.easing.as_deref())
                && !ANIMATION_EASINGS.contains(&easing)
            {
                let function = extern_function(document, easing, ExternKind::Sync, &state.span)?;
                if function.params.len() != 1
                    || function.params[0].1 != Type::F64
                    || function.output != Type::F64
                    || function.error.is_some()
                {
                    return Err(Error::new(
                        "E103",
                        &state.span,
                        format!(
                            "animation easing `{easing}` must be `sync {easing}(value:f64) -> f64`"
                        ),
                    ));
                }
            }
        } else {
            let text_initial =
                matches!(state.ty, Type::Markdown | Type::Editor) && actual == Type::Str;
            if actual != Type::Unknown && !text_initial && !compatible(&state.ty, &actual) {
                return Err(type_error(&state.span, &state.ty, &actual));
            }
        }
    }
    check_app_settings(document, &states)?;
    for handler in document.handlers.iter().chain(&preset_handlers) {
        check_structured_tasks(handler)?;
    }

    let mut signatures: HashMap<String, Vec<Option<Type>>> = document
        .handlers
        .iter()
        .map(|handler| (handler.name.clone(), vec![None; handler.params.len()]))
        .collect();

    let mut ids = HashSet::new();
    let mut view_states = states.clone();
    if document.daemon {
        view_states.insert("window".into(), Type::WindowId);
    }
    infer_view(
        &document.view,
        &view_states,
        document,
        &mut signatures,
        &mut ids,
    )?;
    let pane_grids = static_pane_grids(&document.view, &view_states, document)?;
    for component in &document.components {
        if let Some(span) = pane_grid_span(&component.root) {
            return Err(Error::new(
                "E187",
                span,
                "pane-grid must live in the app view because it owns persistent layout state",
            ));
        }
        let env = component.params.iter().cloned().collect();
        let mut ids = HashSet::new();
        infer_view(&component.root, &env, document, &mut signatures, &mut ids)?;
    }
    let operation_ids = widget_operation_ids(&document.view, &view_states, document)?;
    controlled_state_bindings(document, false)?;
    controlled_state_bindings(document, true)?;
    infer_subscriptions(document, &states, &mut signatures)?;
    for handler in document.handlers.iter().chain(&preset_handlers) {
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

    for handler in document.handlers.iter().chain(&preset_handlers) {
        check_handler(handler, &states, document, &operation_ids, &pane_grids)?;
    }
    Ok(())
}

fn check_app_settings(document: &Document, states: &HashMap<String, Type>) -> Result<(), Error> {
    let mut callback_states = states.clone();
    if document.daemon {
        callback_states.insert("window".into(), Type::WindowId);
    }
    for setting in [&document.settings.background, &document.settings.text_color]
        .into_iter()
        .flatten()
    {
        require_type(
            &expr_type(&setting.value, states, document, &setting.span)?,
            &Type::Str,
            &setting.span,
        )?;
    }
    if let Some(setting) = &document.settings.title {
        require_type(
            &expr_type(&setting.value, &callback_states, document, &setting.span)?,
            &Type::Str,
            &setting.span,
        )?;
    }
    if let Some(setting) = &document.settings.theme {
        if let Expr::Call { name, args } = &setting.value
            && let Some(factory) = document
                .functions
                .iter()
                .find(|function| function.name == *name && function.kind == ExternKind::Theme)
        {
            check_call_args(factory, args, &callback_states, document, &setting.span)?;
        } else {
            require_type(
                &expr_type(&setting.value, &callback_states, document, &setting.span)?,
                &Type::Str,
                &setting.span,
            )?;
        }
    }
    if let Some(setting) = &document.settings.scale_factor {
        require_type(
            &expr_type(&setting.value, &callback_states, document, &setting.span)?,
            &Type::F64,
            &setting.span,
        )?;
        if f64_literal(&setting.value).is_some_and(|value| value <= 0.0) {
            return Err(Error::new(
                "E015",
                &setting.span,
                "scale-factor must be greater than zero",
            ));
        }
    }
    if let Some(AppExpression {
        value: Expr::Str(value),
        span,
    }) = &document.settings.theme
        && value != "app"
        && value != "default"
        && !BUILT_IN_THEMES.contains(&value.as_str())
    {
        return Err(Error::new(
            "E015",
            span,
            format!("unknown iced theme `{value}`"),
        ));
    }
    for setting in [&document.settings.background, &document.settings.text_color]
        .into_iter()
        .flatten()
    {
        if let Expr::Str(value) = &setting.value
            && !valid_app_color(value)
        {
            return Err(Error::new(
                "E015",
                &setting.span,
                "application colors must be 3, 4, 6, or 8 digit hexadecimal strings",
            ));
        }
    }
    Ok(())
}

fn valid_app_color(value: &str) -> bool {
    let hex = value.strip_prefix('#').unwrap_or(value);
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|value| value.is_ascii_hexdigit())
}

type WidgetIdPath = Vec<(String, Option<Type>)>;

#[derive(Clone)]
struct WidgetIdSlot {
    entries: Vec<(String, ViewNode, HashMap<String, Type>)>,
    parent: Option<Box<Self>>,
}

fn widget_operation_ids(
    root: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
) -> Result<Vec<WidgetIdPath>, Error> {
    fn segment(
        id: &Id,
        env: &HashMap<String, Type>,
        document: &Document,
        span: &Span,
    ) -> Result<(String, Option<Type>), Error> {
        Ok((
            id.name.clone(),
            id.key
                .as_ref()
                .map(|key| expr_type(key, env, document, span))
                .transpose()?,
        ))
    }

    fn scoped(
        scope: &WidgetIdPath,
        id: &Option<Id>,
        env: &HashMap<String, Type>,
        document: &Document,
        span: &Span,
    ) -> Result<WidgetIdPath, Error> {
        let mut scope = scope.clone();
        if let Some(id) = id {
            scope.push(segment(id, env, document, span)?);
        }
        Ok(scope)
    }

    fn record(
        scope: &WidgetIdPath,
        id: &Option<Id>,
        env: &HashMap<String, Type>,
        document: &Document,
        span: &Span,
        output: &mut Vec<WidgetIdPath>,
    ) -> Result<(), Error> {
        if id.is_some() {
            let path = scoped(scope, id, env, document, span)?;
            if !output.contains(&path) {
                output.push(path);
            }
        }
        Ok(())
    }

    fn collect(
        node: &ViewNode,
        env: &HashMap<String, Type>,
        document: &Document,
        scope: &WidgetIdPath,
        slot: Option<&WidgetIdSlot>,
        components: &mut Vec<(String, Span)>,
        output: &mut Vec<WidgetIdPath>,
    ) -> Result<(), Error> {
        match node {
            ViewNode::Layout {
                kind,
                id,
                children,
                span,
                ..
            } => {
                if *kind == Layout::Scroll {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                for child in children {
                    collect(child, env, document, &child_scope, slot, components, output)?;
                }
            }
            ViewNode::Input { id, span, .. } | ViewNode::TextEditor { id, span, .. } => {
                record(scope, id, env, document, span, output)?;
            }
            ViewNode::Container {
                id, content, span, ..
            }
            | ViewNode::Button {
                id,
                content: Some(content),
                span,
                ..
            } => {
                let child_scope = scoped(scope, id, env, document, span)?;
                collect(
                    content,
                    env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                )?;
            }
            ViewNode::If { children, .. } => {
                for child in children {
                    collect(child, env, document, scope, slot, components, output)?;
                }
            }
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                let Type::List(inner) = expr_type(items, env, document, span)? else {
                    unreachable!("checker validates for lists")
                };
                let mut child_env = env.clone();
                child_env.insert(item.clone(), *inner);
                for child in children {
                    collect(child, &child_env, document, scope, slot, components, output)?;
                }
            }
            ViewNode::KeyedColumn {
                item,
                items,
                key,
                child,
                span,
                ..
            } => {
                let Type::List(inner) = expr_type(items, env, document, span)? else {
                    unreachable!("checker validates keyed lists")
                };
                let mut child_env = env.clone();
                child_env.insert(item.clone(), *inner);
                let mut child_scope = scope.clone();
                child_scope.push((
                    "key".into(),
                    Some(expr_type(key, &child_env, document, span)?),
                ));
                collect(
                    child,
                    &child_env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                )?;
            }
            ViewNode::Lazy {
                dependency,
                binding,
                child,
                span,
            } => {
                let mut child_env = env.clone();
                child_env.insert(binding.clone(), expr_type(dependency, env, document, span)?);
                collect(child, &child_env, document, scope, slot, components, output)?;
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, env, document, scope, slot, components, output)?;
                collect(tip, env, document, scope, slot, components, output)?;
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, env, document, scope, slot, components, output)?;
                collect(layer, env, document, scope, slot, components, output)?;
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for pane in panes {
                    let mut pane_env = env.clone();
                    if let Some(binding) = &pane.maximized {
                        pane_env.insert(binding.clone(), Type::Bool);
                    }
                    let mut pane_scope = scope.clone();
                    pane_scope.push((pane.name.clone(), None));
                    for node in pane.nodes() {
                        collect(
                            node,
                            &pane_env,
                            document,
                            &pane_scope,
                            slot,
                            components,
                            output,
                        )?;
                    }
                }
                for template in templates {
                    let Type::List(item_type) = env
                        .get(&template.items)
                        .expect("checker validates dynamic pane state")
                    else {
                        unreachable!("checker validates dynamic pane lists")
                    };
                    let mut template_env = env.clone();
                    template_env.insert(template.item.clone(), (**item_type).clone());
                    if let Some(binding) = &template.pane.maximized {
                        template_env.insert(binding.clone(), Type::Bool);
                    }
                    let mut pane_scope = scope.clone();
                    pane_scope.push((
                        template.item.clone(),
                        Some(expr_type(
                            &template.key,
                            &template_env,
                            document,
                            &template.span,
                        )?),
                    ));
                    for node in template.pane.nodes() {
                        collect(
                            node,
                            &template_env,
                            document,
                            &pane_scope,
                            slot,
                            components,
                            output,
                        )?;
                    }
                }
            }
            ViewNode::Table {
                item,
                rows,
                columns,
                span,
                ..
            } => {
                let Type::List(inner) = expr_type(rows, env, document, span)? else {
                    unreachable!("checker validates table rows")
                };
                let mut cell_env = env.clone();
                cell_env.insert(item.clone(), *inner);
                for column in columns {
                    let mut header_scope = scope.clone();
                    header_scope.push(("header".into(), Some(Type::I64)));
                    collect(
                        &column.header,
                        env,
                        document,
                        &header_scope,
                        slot,
                        components,
                        output,
                    )?;
                    let mut cell_scope = scope.clone();
                    cell_scope.push(("row".into(), Some(Type::I64)));
                    cell_scope.push(("column".into(), Some(Type::I64)));
                    collect(
                        &column.cell,
                        &cell_env,
                        document,
                        &cell_scope,
                        slot,
                        components,
                        output,
                    )?;
                }
            }
            ViewNode::Component {
                name,
                id,
                slots,
                span,
                ..
            } => {
                let call = (name.clone(), span.clone());
                if components.contains(&call) {
                    return Err(Error::new(
                        "E122",
                        span,
                        format!("recursive component `{name}` cannot define widget targets"),
                    ));
                }
                let component = document
                    .components
                    .iter()
                    .find(|component| component.name == *name)
                    .expect("checker validates component names");
                let mut component_scope = scope.clone();
                if let Some(id) = id {
                    component_scope.push(segment(id, env, document, span)?);
                } else {
                    component_scope.push((name.clone(), None));
                }
                let component_env = component.params.iter().cloned().collect();
                let component_slot = (!slots.is_empty()).then(|| WidgetIdSlot {
                    entries: slots
                        .iter()
                        .map(|slot| (slot.name.clone(), (*slot.content).clone(), env.clone()))
                        .collect(),
                    parent: slot.cloned().map(Box::new),
                });
                components.push(call);
                collect(
                    &component.root,
                    &component_env,
                    document,
                    &component_scope,
                    component_slot.as_ref(),
                    components,
                    output,
                )?;
                components.pop();
            }
            ViewNode::Slot { name, .. } => {
                if let Some(slot) = slot
                    && let Some((_, content, content_env)) =
                        slot.entries.iter().find(|(entry, ..)| entry == name)
                {
                    collect(
                        content,
                        content_env,
                        document,
                        scope,
                        slot.parent.as_deref(),
                        components,
                        output,
                    )?;
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => {
                collect(content, env, document, scope, slot, components, output)?;
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, env, document, scope, slot, components, output)?;
                    collect(wide, env, document, scope, slot, components, output)?;
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.insert(width.clone(), Type::F64);
                    child_env.insert(height.clone(), Type::F64);
                    collect(
                        content, &child_env, document, scope, slot, components, output,
                    )?;
                }
            },
            _ => {}
        }
        Ok(())
    }

    let mut output = Vec::new();
    collect(
        root,
        env,
        document,
        &Vec::new(),
        None,
        &mut Vec::new(),
        &mut output,
    )?;
    Ok(output)
}

fn check_widget_target(
    target: &WidgetTarget,
    env: &HashMap<String, Type>,
    document: &Document,
    operation_ids: &[WidgetIdPath],
    span: &Span,
) -> Result<(), Error> {
    let mut actual = Vec::with_capacity(target.segments.len());
    for segment in &target.segments {
        let key = segment
            .key
            .as_ref()
            .map(|key| expr_type(key, env, document, span))
            .transpose()?;
        if let Some(key) = &key
            && !matches!(key, Type::Bool | Type::I64 | Type::F64 | Type::Str)
        {
            return Err(Error::new(
                "E172",
                span,
                "widget target keys must be bool, i64, f64, or str",
            ));
        }
        actual.push((segment.name.clone(), key));
    }
    if operation_ids.iter().any(|expected| {
        expected.len() == actual.len()
            && expected
                .iter()
                .zip(&actual)
                .all(|((expected_name, expected_key), (name, key))| {
                    expected_name == name
                        && match (expected_key, key) {
                            (None, None) => true,
                            (Some(expected), Some(actual)) => compatible(expected, actual),
                            _ => false,
                        }
                })
    }) {
        return Ok(());
    }
    let label = format!(
        "#{}",
        target
            .segments
            .iter()
            .map(|segment| if segment.key.is_some() {
                format!("{}(key)", segment.name)
            } else {
                segment.name.clone()
            })
            .collect::<Vec<_>>()
            .join("/")
    );
    let same_shape = operation_ids
        .iter()
        .filter(|expected| {
            expected.len() == actual.len()
                && expected.iter().zip(&actual).all(
                    |((expected_name, expected_key), (name, key))| {
                        expected_name == name && expected_key.is_some() == key.is_some()
                    },
                )
        })
        .collect::<Vec<_>>();
    let mismatch = (!same_shape.is_empty())
        .then(|| {
            (0..actual.len()).find(|index| {
                let Some(actual) = &actual[*index].1 else {
                    return false;
                };
                same_shape.iter().all(|path| {
                    path[*index]
                        .1
                        .as_ref()
                        .is_some_and(|expected| !compatible(expected, actual))
                })
            })
        })
        .flatten();
    if let Some(index) = mismatch {
        let expected = same_shape
            .iter()
            .filter_map(|path| path[index].1.as_ref())
            .map(Type::display)
            .collect::<HashSet<_>>();
        return Err(Error::new(
            "E172",
            span,
            format!(
                "widget target segment `{}` expects key type {}, got `{}`",
                actual[index].0,
                expected
                    .into_iter()
                    .map(|ty| format!("`{ty}`"))
                    .collect::<Vec<_>>()
                    .join(" or "),
                actual[index].1.as_ref().unwrap().display()
            ),
        ));
    }
    Err(
        Error::new("E172", span, format!("unknown app widget target `{label}`")).hint(
            "use the full component, layout, keyed, table, or pane identity path from the app view",
        ),
    )
}

fn widget_selector_output(
    selector: &WidgetSelector,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    match selector {
        WidgetSelector::Extern { function, .. } => {
            Ok(
                extern_function(document, function, ExternKind::Selector, span)?
                    .output
                    .clone(),
            )
        }
        WidgetSelector::Id(_)
        | WidgetSelector::Text(_)
        | WidgetSelector::Point { .. }
        | WidgetSelector::Focused => Ok(Type::WidgetTarget),
    }
}

fn check_widget_selector(
    selector: &WidgetSelector,
    env: &HashMap<String, Type>,
    document: &Document,
    operation_ids: &[WidgetIdPath],
    span: &Span,
) -> Result<Type, Error> {
    match selector {
        WidgetSelector::Id(target) => {
            check_widget_target(target, env, document, operation_ids, span)?;
        }
        WidgetSelector::Text(value) => {
            require_type(&expr_type(value, env, document, span)?, &Type::Str, span)?;
        }
        WidgetSelector::Point { x, y } => {
            for value in [x, y] {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            }
        }
        WidgetSelector::Focused => {}
        WidgetSelector::Extern { function, args } => {
            let function = extern_function(document, function, ExternKind::Selector, span)?;
            check_call_args(function, args, env, document, span)?;
        }
    }
    widget_selector_output(selector, document, span)
}

struct PaneGridNames {
    panes: HashSet<String>,
    templates: HashMap<String, Type>,
    splits: HashSet<String>,
}

fn pane_split_names(configuration: &PaneConfiguration, output: &mut HashSet<String>) {
    if let PaneConfiguration::Split { name, a, b, .. } = configuration {
        if let Some(name) = name {
            output.insert(name.clone());
        }
        pane_split_names(a, output);
        pane_split_names(b, output);
    }
}

fn static_pane_grids(
    root: &ViewNode,
    states: &HashMap<String, Type>,
    document: &Document,
) -> Result<HashMap<String, PaneGridNames>, Error> {
    fn collect(
        node: &ViewNode,
        states: &HashMap<String, Type>,
        document: &Document,
        output: &mut HashMap<String, PaneGridNames>,
    ) -> Result<(), Error> {
        match node {
            ViewNode::PaneGrid {
                name,
                configuration,
                panes,
                templates,
                span,
                ..
            } => {
                let mut splits = HashSet::new();
                pane_split_names(configuration, &mut splits);
                let mut template_types = HashMap::new();
                for template in templates {
                    let Some(Type::List(item_type)) = states.get(&template.items) else {
                        return Err(Error::new(
                            "E187",
                            &template.span,
                            format!(
                                "dynamic pane template `{}` requires list state `{}`",
                                template.item, template.items
                            ),
                        ));
                    };
                    let mut env = states.clone();
                    env.insert(template.item.clone(), (**item_type).clone());
                    template_types.insert(
                        template.item.clone(),
                        expr_type(&template.key, &env, document, &template.span)?,
                    );
                }
                if output
                    .insert(
                        name.clone(),
                        PaneGridNames {
                            panes: panes.iter().map(|pane| pane.name.clone()).collect(),
                            templates: template_types,
                            splits,
                        },
                    )
                    .is_some()
                {
                    return Err(Error::new(
                        "E187",
                        span,
                        format!("duplicate persistent pane-grid `#{name}`"),
                    ));
                }
                for pane in panes {
                    for node in pane.nodes() {
                        collect(node, states, document, output)?;
                    }
                }
                for template in templates {
                    for node in template.pane.nodes() {
                        collect(node, states, document, output)?;
                    }
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, states, document, output)?;
                }
            }
            ViewNode::Tooltip { content, tip, .. }
            | ViewNode::Overlay {
                content,
                layer: tip,
                ..
            } => {
                collect(content, states, document, output)?;
                collect(tip, states, document, output)?;
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, states, document, output)?;
                    collect(&column.cell, states, document, output)?;
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. }
            | ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, states, document, output)?,
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, states, document, output)?;
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, states, document, output)?;
                    collect(wide, states, document, output)?;
                }
                ResponsiveContent::Size { content, .. } => {
                    collect(content, states, document, output)?
                }
            },
            _ => {}
        }
        Ok(())
    }
    let mut output = HashMap::new();
    collect(root, states, document, &mut output)?;
    Ok(output)
}

fn check_declared_types(document: &Document) -> Result<(), Error> {
    let known = document
        .structs
        .iter()
        .map(|item| item.name.as_str())
        .collect::<HashSet<_>>();
    let check = |ty: &Type, span: &Span| check_declared_type(ty, span, &known);
    let reject_debug_span = |ty: &Type, span: &Span| {
        if contains_debug_span(ty) {
            Err(Error::new(
                "E103",
                span,
                "debug-span is non-clone state and must be declared as `debug-span?` state",
            ))
        } else {
            Ok(())
        }
    };

    for item in &document.structs {
        for (_, ty) in &item.fields {
            reject_debug_span(ty, &item.span)?;
            check(ty, &item.span)?;
        }
    }
    for item in &document.functions {
        for (_, ty) in &item.params {
            reject_debug_span(ty, &item.span)?;
            check(ty, &item.span)?;
        }
        if let Some(progress) = &item.progress {
            reject_debug_span(progress, &item.span)?;
            check(progress, &item.span)?;
        }
        reject_debug_span(&item.output, &item.span)?;
        check(&item.output, &item.span)?;
        if let Some(error) = &item.error {
            reject_debug_span(error, &item.span)?;
            check(error, &item.span)?;
        }
    }
    for state in &document.states {
        if contains_debug_span(&state.ty) && state.ty != Type::Option(Box::new(Type::DebugSpan)) {
            return Err(Error::new(
                "E103",
                &state.span,
                "debug span state must have type `debug-span?`",
            ));
        }
        check(&state.ty, &state.span)?;
    }
    for component in &document.components {
        for (_, ty) in &component.params {
            reject_debug_span(ty, &component.span)?;
            check(ty, &component.span)?;
        }
    }
    Ok(())
}

fn contains_debug_span(ty: &Type) -> bool {
    match ty {
        Type::DebugSpan => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            contains_debug_span(inner)
        }
        Type::Result(output, error) => contains_debug_span(output) || contains_debug_span(error),
        _ => false,
    }
}

fn check_declared_type(ty: &Type, span: &Span, known: &HashSet<&str>) -> Result<(), Error> {
    match ty {
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => {
            check_declared_type(inner, span, known)
        }
        Type::Result(output, error) => {
            check_declared_type(output, span, known)?;
            check_declared_type(error, span, known)
        }
        Type::Animation(inner) if matches!(inner.as_ref(), Type::Bool | Type::F64) => Ok(()),
        Type::Animation(inner) if matches!(inner.as_ref(), Type::Named(_)) => {
            check_declared_type(inner, span, known)
        }
        Type::Animation(inner) => Err(Error::new(
            "E103",
            span,
            format!(
                "animation state supports `bool`, `f64`, or a named extern type, not `{}`",
                inner.display()
            ),
        )),
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
    let mut presets = HashSet::new();
    for preset in &document.presets {
        if !presets.insert(&preset.name) {
            return Err(Error::new(
                "E100",
                &preset.span,
                format!("duplicate preset `{}`", preset.name),
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
        if document.daemon && state.name == "window" {
            return Err(
                Error::new("E100", &state.span, "daemon state cannot be named `window`")
                    .hint("`window` is the current window-id inside daemon views and callbacks"),
            );
        }
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
        let mut params = HashSet::new();
        for (param, _) in &component.params {
            if !params.insert(param) {
                return Err(Error::new(
                    "E100",
                    &component.span,
                    format!("duplicate component prop `{param}`"),
                ));
            }
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
    if let Some((_, span)) = view_slots.first() {
        return Err(Error::new(
            "E124",
            span,
            "slot is only valid inside a component definition",
        ));
    }
    for component in &document.components {
        let mut names = HashSet::new();
        for (name, span) in slots(&component.root) {
            if !names.insert(name) {
                return Err(Error::new(
                    "E124",
                    span,
                    format!(
                        "component `{}` declares slot `{name}` more than once",
                        component.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn slots(node: &ViewNode) -> Vec<(&str, &Span)> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<(&'a str, &'a Span)>) {
        match node {
            ViewNode::Slot { name, span } => output.push((name, span)),
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
            | ViewNode::Container { content, .. }
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
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    collect(child, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
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

pub(crate) fn controlled_state_bindings(
    document: &Document,
    editors: bool,
) -> Result<Vec<String>, Error> {
    fn collect(
        node: &ViewNode,
        document: &Document,
        editors: bool,
        env: &HashMap<String, String>,
        components: &mut HashSet<String>,
        output: &mut Vec<String>,
    ) -> Result<(), Error> {
        let binding = match node {
            ViewNode::Input { binding, span, .. } if !editors => Some((binding, "input", span)),
            ViewNode::TextEditor { binding, span, .. } if editors => {
                Some((binding, "editor", span))
            }
            _ => None,
        };
        if let Some((binding, widget, span)) = binding {
            let state = env.get(binding).ok_or_else(|| {
                Error::new(
                    "E139",
                    span,
                    format!("{widget} binding must resolve to an app state"),
                )
            })?;
            if !output.contains(state) {
                output.push(state.clone());
            }
            return Ok(());
        }

        match node {
            ViewNode::Layout { children, .. } | ViewNode::If { children, .. } => {
                for child in children {
                    collect(child, document, editors, env, components, output)?;
                }
            }
            ViewNode::For { item, children, .. } => {
                let mut child_env = env.clone();
                child_env.remove(item);
                for child in children {
                    collect(child, document, editors, &child_env, components, output)?;
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            }
            | ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => {
                collect(content, document, editors, env, components, output)?;
            }
            ViewNode::KeyedColumn { item, child, .. } => {
                let mut child_env = env.clone();
                child_env.remove(item);
                collect(child, document, editors, &child_env, components, output)?;
            }
            ViewNode::Lazy { binding, child, .. } => {
                let mut child_env = env.clone();
                child_env.remove(binding);
                collect(child, document, editors, &child_env, components, output)?;
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, document, editors, env, components, output)?;
                collect(tip, document, editors, env, components, output)?;
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, document, editors, env, components, output)?;
                collect(layer, document, editors, env, components, output)?;
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for pane in panes {
                    let mut child_env = env.clone();
                    if let Some(binding) = &pane.maximized {
                        child_env.remove(binding);
                    }
                    for child in pane.nodes() {
                        collect(child, document, editors, &child_env, components, output)?;
                    }
                }
                for template in templates {
                    let mut child_env = env.clone();
                    child_env.remove(&template.item);
                    if let Some(binding) = &template.pane.maximized {
                        child_env.remove(binding);
                    }
                    for child in template.pane.nodes() {
                        collect(child, document, editors, &child_env, components, output)?;
                    }
                }
            }
            ViewNode::Table { item, columns, .. } => {
                let mut cell_env = env.clone();
                cell_env.remove(item);
                for column in columns {
                    collect(&column.header, document, editors, env, components, output)?;
                    collect(
                        &column.cell,
                        document,
                        editors,
                        &cell_env,
                        components,
                        output,
                    )?;
                }
            }
            ViewNode::Component {
                name,
                args,
                slots,
                span,
                ..
            } => {
                for slot in slots {
                    collect(&slot.content, document, editors, env, components, output)?;
                }
                if !components.insert(name.clone()) {
                    return Err(Error::new(
                        "E122",
                        span,
                        format!("recursive component `{name}` cannot contain controlled state"),
                    ));
                }
                let component = document
                    .components
                    .iter()
                    .find(|item| item.name == *name)
                    .expect("checker validates component names");
                let named = args.iter().any(|arg| arg.name.is_some());
                let mut component_env = HashMap::new();
                for (index, (param, _)) in component.params.iter().enumerate() {
                    let arg = if named {
                        args.iter()
                            .find(|arg| arg.name.as_ref() == Some(param))
                            .expect("checker validates named component arguments")
                    } else {
                        &args[index]
                    };
                    if let Expr::Path(path) = &arg.value
                        && path.len() == 1
                        && let Some(state) = env.get(&path[0])
                    {
                        component_env.insert(param.clone(), state.clone());
                    }
                }
                collect(
                    &component.root,
                    document,
                    editors,
                    &component_env,
                    components,
                    output,
                )?;
                components.remove(name);
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, document, editors, env, components, output)?;
                    collect(wide, document, editors, env, components, output)?;
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.remove(width);
                    child_env.remove(height);
                    collect(content, document, editors, &child_env, components, output)?;
                }
            },
            _ => {}
        }
        Ok(())
    }

    let env = document
        .states
        .iter()
        .map(|state| (state.name.clone(), state.name.clone()))
        .collect();
    let mut output = Vec::new();
    collect(
        &document.view,
        document,
        editors,
        &env,
        &mut HashSet::new(),
        &mut output,
    )?;
    Ok(output)
}

fn pane_grid_span(node: &ViewNode) -> Option<&Span> {
    match node {
        ViewNode::PaneGrid { span, .. } => Some(span),
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => children.iter().find_map(pane_grid_span),
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::Container { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => pane_grid_span(content),
        ViewNode::Tooltip { content, tip, .. } => {
            pane_grid_span(content).or_else(|| pane_grid_span(tip))
        }
        ViewNode::Overlay { content, layer, .. } => {
            pane_grid_span(content).or_else(|| pane_grid_span(layer))
        }
        ViewNode::Table { columns, .. } => columns.iter().find_map(|column| {
            pane_grid_span(&column.header).or_else(|| pane_grid_span(&column.cell))
        }),
        ViewNode::Component { slots, .. } => {
            slots.iter().find_map(|slot| pane_grid_span(&slot.content))
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                pane_grid_span(narrow).or_else(|| pane_grid_span(wide))
            }
            ResponsiveContent::Size { content, .. } => pane_grid_span(content),
        },
        _ => None,
    }
}

fn repeated_pane_grid_span(node: &ViewNode) -> Option<&Span> {
    match node {
        ViewNode::For { children, .. } => children.iter().find_map(pane_grid_span),
        ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => pane_grid_span(child),
        ViewNode::Table { columns, .. } => columns.iter().find_map(|column| {
            pane_grid_span(&column.header).or_else(|| pane_grid_span(&column.cell))
        }),
        ViewNode::Layout { children, .. } | ViewNode::If { children, .. } => {
            children.iter().find_map(repeated_pane_grid_span)
        }
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::Container { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. } => repeated_pane_grid_span(content),
        ViewNode::Tooltip { content, tip, .. } => {
            repeated_pane_grid_span(content).or_else(|| repeated_pane_grid_span(tip))
        }
        ViewNode::Overlay { content, layer, .. } => {
            repeated_pane_grid_span(content).or_else(|| repeated_pane_grid_span(layer))
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => panes
            .iter()
            .flat_map(PaneView::nodes)
            .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            .find_map(repeated_pane_grid_span),
        ViewNode::Component { slots, .. } => slots
            .iter()
            .find_map(|slot| repeated_pane_grid_span(&slot.content)),
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                repeated_pane_grid_span(narrow).or_else(|| repeated_pane_grid_span(wide))
            }
            ResponsiveContent::Size { content, .. } => repeated_pane_grid_span(content),
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
                    GridSizing::EvenlyDistribute(length) => {
                        check_length_value(length, env, document, span, "grid height")?;
                    }
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
            if let Some(width) = &options.width {
                if *kind == Layout::Grid {
                    let LengthValue::Fixed(value) = width else {
                        unreachable!("parser keeps grid widths fixed")
                    };
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_literal_range(value, 0.0, None, layout_metric, span)?;
                } else {
                    check_length_value(width, env, document, span, layout_metric)?;
                }
            }
            if let Some(height) = &options.height {
                check_length_value(height, env, document, span, layout_metric)?;
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
                    check_length_value(length, env, document, span, "scroll size")?;
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
                check_length_value(length, env, document, span, "container size")?;
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
            templates,
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
                check_length_value(length, env, document, span, "pane-grid bounds")?;
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
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::PaneGridStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
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
                infer_pane_view(pane, env, document, signatures, ids)?;
            }
            for template in templates {
                let Some(Type::List(item_type)) = env.get(&template.items) else {
                    return Err(Error::new(
                        "E187",
                        &template.span,
                        format!(
                            "dynamic pane template `{}` requires list state `{}`",
                            template.item, template.items
                        ),
                    ));
                };
                let mut template_env = env.clone();
                template_env.insert(template.item.clone(), (**item_type).clone());
                let key_type = expr_type(&template.key, &template_env, document, &template.span)?;
                if !matches!(key_type, Type::Bool | Type::I64 | Type::F64 | Type::Str) {
                    return Err(Error::new(
                        "E187",
                        &template.span,
                        "dynamic pane keys must be bool, i64, f64, or str values",
                    ));
                }
                infer_pane_view(&template.pane, &template_env, document, signatures, ids)?;
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
            if let Some(length) = &options.width {
                check_length_value(length, env, document, span, "input width")?;
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
                check_length_value(length, env, document, span, "button size")?;
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
                        LengthValue::Fixed(value) if !fluid => {
                            require_type(
                                &expr_type(value, env, document, span)?,
                                &Type::F64,
                                span,
                            )?;
                            require_literal_range(value, 0.0, None, label, span)?;
                        }
                        LengthValue::Fixed(_) => {
                            check_length_value(length, env, document, span, label)?;
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
                check_length_value(length, env, document, span, "progress size")?;
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
                check_length_value(length, env, document, span, "pick size")?;
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
                check_length_value(length, env, document, span, "combo size")?;
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
                check_length_value(length, env, document, span, "space length")?;
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
                check_length_value(length, env, document, span, "keyed size")?;
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
            if let Some(length) = &options.height {
                check_length_value(length, env, document, span, "editor height")?;
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
            if let Some(length) = &options.width {
                check_length_value(length, env, document, span, "table width")?;
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
                if let Some(length) = &column.width {
                    check_length_value(length, env, document, &column.span, "table column width")?;
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
                check_length_value(length, env, document, span, "shader size")?;
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
                check_length_value(length, env, document, span, "media size")?;
            }
            if let Some(rotation) = &options.rotation {
                let actual = expr_type(rotation, env, document, span)?;
                if !matches!(actual, Type::F64 | Type::Rotation) {
                    return Err(Error::new(
                        "E101",
                        span,
                        format!("expected `f64` or `rotation`, got `{}`", actual.display()),
                    ));
                }
            }
            if let Some(fit) = &options.fit {
                require_type(
                    &expr_type(fit, env, document, span)?,
                    &Type::ContentFit,
                    span,
                )?;
            }
            for (value, label, min, max) in [
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
                check_length_value(length, env, document, span, "canvas size")?;
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
                if matches!(local.ty, Type::Animation(_)) {
                    return Err(Error::new(
                        "E190",
                        &local.span,
                        "canvas-local animation is not supported; declare it in app state",
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
    }
    Ok(())
}

fn lazy_hashable(ty: &Type) -> bool {
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
        | Type::ContentFit
        | Type::Alignment
        | Type::HorizontalAlignment
        | Type::VerticalAlignment
        | Type::Named(_) => true,
        Type::List(inner) | Type::Option(inner) => lazy_hashable(inner),
        Type::Result(output, error) => lazy_hashable(output) && lazy_hashable(error),
        Type::F64
        | Type::Combo(_)
        | Type::Animation(_)
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
        | Type::Rotation
        | Type::Color
        | Type::Length
        | Type::Shadow
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
        | Type::ImageAllocation
        | Type::ImageMemory
        | Type::ImageError
        | Type::DebugSpan
        | Type::SizeU32
        | Type::Unit
        | Type::Unknown => false,
    }
}

fn check_canvas_commands(
    commands: &[CanvasCommand],
    env: &HashMap<String, Type>,
    document: &Document,
) -> Result<(), Error> {
    for command in commands {
        match command {
            CanvasCommand::Rectangle {
                x,
                y,
                width,
                height,
                radius,
                paint,
                span,
            } => {
                check_canvas_number(x, env, document, span, "rectangle x", None)?;
                check_canvas_number(y, env, document, span, "rectangle y", None)?;
                check_canvas_number(width, env, document, span, "rectangle width", Some(0.0))?;
                check_canvas_number(height, env, document, span, "rectangle height", Some(0.0))?;
                check_canvas_radius(radius, env, document, span)?;
                check_canvas_paint(paint, env, document, span)?;
            }
            CanvasCommand::Circle {
                x,
                y,
                radius,
                paint,
                span,
            } => {
                check_canvas_number(x, env, document, span, "circle x", None)?;
                check_canvas_number(y, env, document, span, "circle y", None)?;
                check_canvas_number(radius, env, document, span, "circle radius", Some(0.0))?;
                check_canvas_paint(paint, env, document, span)?;
            }
            CanvasCommand::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                span,
            } => {
                for (value, label) in [
                    (x1, "line x1"),
                    (y1, "line y1"),
                    (x2, "line x2"),
                    (y2, "line y2"),
                ] {
                    check_canvas_number(value, env, document, span, label, None)?;
                }
                check_canvas_stroke(stroke, env, document, span)?;
            }
            CanvasCommand::Text {
                value,
                x,
                y,
                max_width,
                color,
                size,
                line_height,
                font,
                span,
                ..
            } => {
                let ty = expr_type(value, env, document, span)?;
                if !matches!(ty, Type::Str | Type::I64 | Type::F64) {
                    return Err(type_error(span, &Type::Str, &ty)
                        .hint("canvas text accepts str, i64, or f64"));
                }
                check_canvas_number(x, env, document, span, "text x", None)?;
                check_canvas_number(y, env, document, span, "text y", None)?;
                if let Some(value) = max_width {
                    check_canvas_number(value, env, document, span, "text max width", Some(0.0))?;
                }
                if let Some(value) = size {
                    check_canvas_number(
                        value,
                        env,
                        document,
                        span,
                        "text size",
                        Some(f64::EPSILON),
                    )?;
                }
                if let Some(height) = line_height {
                    let (value, label) = match height {
                        TextLineHeight::Relative(value) => (value, "text line height"),
                        TextLineHeight::Absolute(value) => (value, "text line height pixels"),
                    };
                    check_canvas_number(value, env, document, span, label, Some(f64::EPSILON))?;
                }
                if color
                    .as_ref()
                    .is_some_and(|color| !valid_theme_color(color, document))
                {
                    return Err(Error::new("E190", span, "unknown canvas text color"));
                }
                check_font(font.as_ref(), document, span)?;
            }
            CanvasCommand::Image {
                source,
                x,
                y,
                width,
                height,
                rotation,
                opacity,
                snap,
                radius,
                span,
                ..
            } => {
                let source_ty = expr_type(source, env, document, span)?;
                if !matches!(source_ty, Type::Str | Type::Image) {
                    return Err(type_error(span, &Type::Image, &source_ty)
                        .hint("canvas image accepts a path string or image handle"));
                }
                for (value, label, min) in [
                    (x, "image x", None),
                    (y, "image y", None),
                    (width, "image width", Some(0.0)),
                    (height, "image height", Some(0.0)),
                    (rotation, "image rotation", None),
                    (opacity, "image opacity", Some(0.0)),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
                require_literal_range(opacity, 0.0, Some(1.0), "image opacity", span)?;
                require_type(&expr_type(snap, env, document, span)?, &Type::Bool, span)?;
                check_canvas_radius(radius, env, document, span)?;
            }
            CanvasCommand::Svg {
                source,
                memory,
                x,
                y,
                width,
                height,
                color,
                rotation,
                opacity,
                span,
            } => {
                let source_ty = expr_type(source, env, document, span)?;
                let valid_source = if *memory {
                    matches!(source_ty, Type::Str | Type::Bytes)
                } else {
                    source_ty == Type::Str
                };
                if !valid_source {
                    return Err(type_error(
                        span,
                        if *memory { &Type::Bytes } else { &Type::Str },
                        &source_ty,
                    )
                    .hint("canvas svg accepts a path string, or UTF-8/raw bytes with `memory`"));
                }
                for (value, label, min) in [
                    (x, "svg x", None),
                    (y, "svg y", None),
                    (width, "svg width", Some(0.0)),
                    (height, "svg height", Some(0.0)),
                    (rotation, "svg rotation", None),
                    (opacity, "svg opacity", Some(0.0)),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
                require_literal_range(opacity, 0.0, Some(1.0), "svg opacity", span)?;
                if color
                    .as_ref()
                    .is_some_and(|color| !valid_theme_color(color, document))
                {
                    return Err(Error::new("E190", span, "unknown canvas svg color"));
                }
            }
            CanvasCommand::Path {
                segments,
                paint,
                span,
            } => {
                check_canvas_path(segments, env, document, span)?;
                check_canvas_paint(paint, env, document, span)?;
            }
            CanvasCommand::Group {
                transform,
                commands,
                span,
            } => {
                for (value, label) in [
                    (&transform.x, "group x"),
                    (&transform.y, "group y"),
                    (&transform.rotate, "group rotation"),
                ] {
                    if let Some(value) = value {
                        check_canvas_number(value, env, document, span, label, None)?;
                    }
                }
                for (value, label) in [
                    (&transform.scale, "group scale"),
                    (&transform.scale_x, "group x scale"),
                    (&transform.scale_y, "group y scale"),
                ] {
                    if let Some(value) = value {
                        check_canvas_number(value, env, document, span, label, Some(f64::EPSILON))?;
                    }
                }
                if let Some([x, y, width, height]) = &transform.clip {
                    check_canvas_number(x, env, document, span, "clip x", None)?;
                    check_canvas_number(y, env, document, span, "clip y", None)?;
                    check_canvas_number(width, env, document, span, "clip width", Some(0.0))?;
                    check_canvas_number(height, env, document, span, "clip height", Some(0.0))?;
                }
                check_canvas_commands(commands, env, document)?;
            }
            CanvasCommand::If {
                condition,
                commands,
                span,
            } => {
                require_type(
                    &expr_type(condition, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
                check_canvas_commands(commands, env, document)?;
            }
            CanvasCommand::For {
                item,
                items,
                commands,
                span,
            } => {
                let Type::List(inner) = expr_type(items, env, document, span)? else {
                    return Err(Error::new(
                        "E190",
                        span,
                        "canvas for expects a list expression",
                    ));
                };
                let mut child_env = env.clone();
                child_env.insert(item.clone(), *inner);
                check_canvas_commands(commands, &child_env, document)?;
            }
        }
    }
    Ok(())
}

fn check_canvas_path(
    segments: &[CanvasPathSegment],
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for segment in segments {
        match segment {
            CanvasPathSegment::Move(x, y) | CanvasPathSegment::Line(x, y) => {
                check_canvas_number(x, env, document, span, "path x", None)?;
                check_canvas_number(y, env, document, span, "path y", None)?;
            }
            CanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => {
                for (value, label, min) in [
                    (x, "arc x", None),
                    (y, "arc y", None),
                    (radius, "arc radius", Some(0.0)),
                    (start, "arc start", None),
                    (end, "arc end", None),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
            }
            CanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => {
                for (value, label, min) in [
                    (ax, "arc-to ax", None),
                    (ay, "arc-to ay", None),
                    (bx, "arc-to bx", None),
                    (by, "arc-to by", None),
                    (radius, "arc-to radius", Some(0.0)),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
            }
            CanvasPathSegment::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                rotation,
                start,
                end,
            } => {
                for (value, label, min) in [
                    (x, "ellipse x", None),
                    (y, "ellipse y", None),
                    (radius_x, "ellipse x radius", Some(0.0)),
                    (radius_y, "ellipse y radius", Some(0.0)),
                    (rotation, "ellipse rotation", None),
                    (start, "ellipse start", None),
                    (end, "ellipse end", None),
                ] {
                    check_canvas_number(value, env, document, span, label, min)?;
                }
            }
            CanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => {
                for value in [control_ax, control_ay, control_bx, control_by, x, y] {
                    check_canvas_number(value, env, document, span, "bezier coordinate", None)?;
                }
            }
            CanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => {
                for value in [control_x, control_y, x, y] {
                    check_canvas_number(value, env, document, span, "quadratic coordinate", None)?;
                }
            }
            CanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            }
            | CanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                ..
            } => {
                check_canvas_number(x, env, document, span, "path rectangle x", None)?;
                check_canvas_number(y, env, document, span, "path rectangle y", None)?;
                check_canvas_number(
                    width,
                    env,
                    document,
                    span,
                    "path rectangle width",
                    Some(0.0),
                )?;
                check_canvas_number(
                    height,
                    env,
                    document,
                    span,
                    "path rectangle height",
                    Some(0.0),
                )?;
                if let CanvasPathSegment::RoundedRectangle { radius, .. } = segment {
                    check_canvas_radius(radius, env, document, span)?;
                }
            }
            CanvasPathSegment::Circle { x, y, radius } => {
                check_canvas_number(x, env, document, span, "path circle x", None)?;
                check_canvas_number(y, env, document, span, "path circle y", None)?;
                check_canvas_number(radius, env, document, span, "path circle radius", Some(0.0))?;
            }
            CanvasPathSegment::Close => {}
        }
    }
    Ok(())
}

fn check_canvas_paint(
    paint: &CanvasPaint,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if let Some(fill) = &paint.fill {
        check_background_value(fill, env, document, span, "E190", "canvas fill")?;
    }
    if let Some(stroke) = &paint.stroke {
        check_canvas_stroke(stroke, env, document, span)?;
    }
    Ok(())
}

fn check_canvas_stroke(
    stroke: &CanvasStroke,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    check_background_value(&stroke.style, env, document, span, "E190", "canvas stroke")?;
    check_canvas_number(
        &stroke.width,
        env,
        document,
        span,
        "stroke width",
        Some(0.0),
    )?;
    require_type(
        &expr_type(&stroke.dash_offset, env, document, span)?,
        &Type::I64,
        span,
    )?;
    require_literal_range(&stroke.dash_offset, 0.0, None, "dash offset", span)?;
    for value in &stroke.dash {
        check_canvas_number(value, env, document, span, "dash segment", Some(0.0))?;
    }
    Ok(())
}

fn check_canvas_radius(
    radius: &CanvasRadius,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for value in [
        &radius.all,
        &radius.top_left,
        &radius.top_right,
        &radius.bottom_right,
        &radius.bottom_left,
    ]
    .into_iter()
    .flatten()
    {
        check_canvas_number(value, env, document, span, "corner radius", Some(0.0))?;
    }
    Ok(())
}

fn check_canvas_number(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    label: &str,
    min: Option<f64>,
) -> Result<(), Error> {
    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
    if let Some(min) = min {
        require_literal_range(value, min, None, label, span)?;
    }
    Ok(())
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
        ViewNode::Slot { span, .. } if !supplied_slot => Err(Error::new(
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
        | ViewNode::Container { content, .. }
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
        ViewNode::Overlay { content, layer, .. } => {
            check_lazy_subtree(content, document, components, supplied_slot)?;
            check_lazy_subtree(layer, document, components, supplied_slot)
        }
        ViewNode::PaneGrid { span, .. } => Err(Error::new(
            "E187",
            span,
            "pane-grid cannot live in lazy because its layout state is persistent",
        )),
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
            name, slots, span, ..
        } => {
            for slot in slots {
                check_lazy_subtree(&slot.content, document, components, supplied_slot)?;
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
                check_lazy_subtree(&component.root, document, components, !slots.is_empty());
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

fn check_background_value(
    background: &BackgroundValue,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    code: &'static str,
    label: &str,
) -> Result<(), Error> {
    match background {
        BackgroundValue::Color(color) => {
            if !valid_theme_color(color, document) {
                return Err(Error::new(
                    code,
                    span,
                    format!("unknown {label} color `{color}`"),
                ));
            }
        }
        BackgroundValue::Linear { angle, stops } => {
            require_type(&expr_type(angle, env, document, span)?, &Type::F64, span)?;
            for stop in stops {
                if !valid_theme_color(&stop.color, document) {
                    return Err(Error::new(
                        code,
                        span,
                        format!("unknown {label} color `{}`", stop.color),
                    ));
                }
                require_type(
                    &expr_type(&stop.offset, env, document, span)?,
                    &Type::F64,
                    span,
                )?;
                require_literal_range(&stop.offset, 0.0, Some(1.0), "gradient stop", span)?;
            }
        }
    }
    Ok(())
}

fn infer_pane_view(
    pane: &PaneView,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<(), Error> {
    let mut pane_env = env.clone();
    if let Some(binding) = &pane.maximized {
        pane_env.insert(binding.clone(), Type::Bool);
    }
    let env = &pane_env;
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
        check_container_style_options(&title.style, env, document, &title.span, "E187")?;
    }
    for node in pane.nodes() {
        infer_view(node, env, document, signatures, ids)?;
    }
    Ok(())
}

fn check_container_style_options(
    style: &ContainerStyleOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    code: &'static str,
) -> Result<(), Error> {
    if let Some(background) = &style.background {
        check_background_value(background, env, document, span, code, "surface")?;
    }
    for (color, label) in [
        (&style.text_color, "surface text"),
        (&style.border_color, "surface border"),
        (&style.shadow_color, "surface shadow"),
    ] {
        if let Some(color) = color
            && !valid_theme_color(color, document)
        {
            return Err(Error::new(
                code,
                span,
                format!("unknown {label} color `{color}`"),
            ));
        }
    }
    for value in [
        &style.border_width,
        &style.radius,
        &style.radius_top_left,
        &style.radius_top_right,
        &style.radius_bottom_right,
        &style.radius_bottom_left,
        &style.shadow_blur,
    ]
    .into_iter()
    .flatten()
    {
        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
        require_literal_range(value, 0.0, None, "surface style metric", span)?;
    }
    for value in [&style.shadow_x, &style.shadow_y].into_iter().flatten() {
        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
    }
    if let Some(snap) = &style.pixel_snap {
        require_type(&expr_type(snap, env, document, span)?, &Type::Bool, span)?;
    }
    Ok(())
}

fn check_markdown_style(
    style: &MarkdownStyleOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for font in [
        style.font.as_ref(),
        style.inline_code_font.as_ref(),
        style.code_block_font.as_ref(),
    ] {
        check_font(font, document, span)?;
    }
    if let Some(background) = &style.inline_code_background {
        check_background_value(
            background,
            env,
            document,
            span,
            "E139",
            "markdown inline code",
        )?;
    }
    for (color, label) in [
        (&style.inline_code_color, "markdown inline code"),
        (
            &style.inline_code_border_color,
            "markdown inline code border",
        ),
        (&style.link_color, "markdown link"),
    ] {
        if let Some(color) = color
            && !valid_theme_color(color, document)
        {
            return Err(Error::new(
                "E139",
                span,
                format!("unknown {label} color `{color}`"),
            ));
        }
    }
    for value in [
        style.inline_code_padding.all.as_ref(),
        style.inline_code_padding.x.as_ref(),
        style.inline_code_padding.y.as_ref(),
        style.inline_code_padding.top.as_ref(),
        style.inline_code_padding.right.as_ref(),
        style.inline_code_padding.bottom.as_ref(),
        style.inline_code_padding.left.as_ref(),
        style.inline_code_border_width.as_ref(),
        style.inline_code_radius.as_ref(),
        style.inline_code_radius_top_left.as_ref(),
        style.inline_code_radius_top_right.as_ref(),
        style.inline_code_radius_bottom_right.as_ref(),
        style.inline_code_radius_bottom_left.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
        require_literal_range(value, 0.0, None, "markdown style metric", span)?;
    }
    Ok(())
}

fn check_float_style_options(
    style: &FloatStyleOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if let Some(color) = &style.shadow_color
        && !valid_theme_color(color, document)
    {
        return Err(Error::new(
            "E128",
            span,
            format!("unknown float shadow color `{color}`"),
        ));
    }
    for value in [
        &style.shadow_blur,
        &style.radius,
        &style.radius_top_left,
        &style.radius_top_right,
        &style.radius_bottom_right,
        &style.radius_bottom_left,
    ]
    .into_iter()
    .flatten()
    {
        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
        require_literal_range(value, 0.0, None, "float style metric", span)?;
    }
    for value in [&style.shadow_x, &style.shadow_y].into_iter().flatten() {
        require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
    }
    Ok(())
}

fn f64_literal(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::F64(value) => Some(*value),
        Expr::I64(value) => Some(*value as f64),
        Expr::Unary {
            op: UnaryOp::Neg,
            value,
        } => f64_literal(value).map(|value| -value),
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
    if let Some(length) = &options.width {
        check_length_value(length, env, document, span, "control width")?;
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

fn check_checkbox_styles(
    styles: &CheckboxStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [
        &styles.active_checked,
        &styles.active_unchecked,
        &styles.hovered_checked,
        &styles.hovered_unchecked,
        &styles.disabled_checked,
        &styles.disabled_unchecked,
    ]
    .into_iter()
    .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        if let Some(background) = &style.background {
            check_background_value(
                background,
                env,
                document,
                span,
                "E129",
                "checkbox background",
            )?;
        }
        for (color, label) in [
            (&style.icon_color, "checkbox icon"),
            (&style.text_color, "checkbox text"),
            (&style.border_color, "checkbox border"),
        ] {
            if let Some(color) = color
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("unknown {label} color `{color}`"),
                ));
            }
        }
        for value in [
            &style.border_width,
            &style.radius,
            &style.radius_top_left,
            &style.radius_top_right,
            &style.radius_bottom_right,
            &style.radius_bottom_left,
        ]
        .into_iter()
        .flatten()
        {
            require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            require_literal_range(value, 0.0, None, "checkbox style metric", span)?;
        }
    }
    Ok(())
}

fn check_toggler_styles(
    styles: &TogglerStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [
        &styles.active_checked,
        &styles.active_unchecked,
        &styles.hovered_checked,
        &styles.hovered_unchecked,
        &styles.disabled_checked,
        &styles.disabled_unchecked,
    ]
    .into_iter()
    .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        for (background, label) in [
            (&style.background, "toggler background"),
            (&style.foreground, "toggler foreground"),
        ] {
            if let Some(background) = background {
                check_background_value(background, env, document, span, "E129", label)?;
            }
        }
        for (color, label) in [
            (&style.background_border_color, "toggler background border"),
            (&style.foreground_border_color, "toggler foreground border"),
            (&style.text_color, "toggler text"),
        ] {
            if let Some(color) = color
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("unknown {label} color `{color}`"),
                ));
            }
        }
        for value in [
            &style.background_border_width,
            &style.foreground_border_width,
            &style.radius,
            &style.radius_top_left,
            &style.radius_top_right,
            &style.radius_bottom_right,
            &style.radius_bottom_left,
        ]
        .into_iter()
        .flatten()
        {
            require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            require_literal_range(value, 0.0, None, "toggler style metric", span)?;
        }
        if let Some(ratio) = &style.padding_ratio {
            require_type(&expr_type(ratio, env, document, span)?, &Type::F64, span)?;
            require_literal_range(ratio, 0.0, Some(0.5), "toggler padding ratio", span)?;
        }
    }
    Ok(())
}

fn check_radio_styles(
    styles: &RadioStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    parent_span: &Span,
) -> Result<(), Error> {
    for style in [
        &styles.active_selected,
        &styles.active_unselected,
        &styles.hovered_selected,
        &styles.hovered_unselected,
    ]
    .into_iter()
    .flatten()
    {
        let span = style.span.as_ref().unwrap_or(parent_span);
        if let Some(background) = &style.background {
            check_background_value(background, env, document, span, "E129", "radio background")?;
        }
        for (color, label) in [
            (&style.dot_color, "radio dot"),
            (&style.border_color, "radio border"),
            (&style.text_color, "radio text"),
        ] {
            if let Some(color) = color
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("unknown {label} color `{color}`"),
                ));
            }
        }
        if let Some(width) = &style.border_width {
            require_type(&expr_type(width, env, document, span)?, &Type::F64, span)?;
            require_literal_range(width, 0.0, None, "radio border width", span)?;
        }
    }
    Ok(())
}

fn check_pick_list_handle(
    handle: Option<&PickListHandle>,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    let Some(handle) = handle else { return Ok(()) };
    let icons = match handle {
        PickListHandle::Arrow { size } => {
            if let Some(size) = size {
                require_type(&expr_type(size, env, document, span)?, &Type::F64, span)?;
                require_literal_range(size, 0.0, None, "pick handle size", span)?;
            }
            return Ok(());
        }
        PickListHandle::Static(icon) => [Some(icon), None],
        PickListHandle::Dynamic { closed, open } => [Some(closed), Some(open)],
        PickListHandle::None => return Ok(()),
    };
    for icon in icons.into_iter().flatten() {
        check_font(icon.font.as_ref(), document, &icon.span)?;
        for (value, label) in [
            (&icon.size, "pick handle icon size"),
            (&icon.line_height, "pick handle icon line height"),
        ] {
            if let Some(value) = value {
                require_type(
                    &expr_type(value, env, document, &icon.span)?,
                    &Type::F64,
                    &icon.span,
                )?;
                require_literal_range(value, 0.0, None, label, &icon.span)?;
            }
        }
    }
    Ok(())
}

fn check_pick_list_styles(
    options: &PickListOptions,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    for style in [
        &options.style.active,
        &options.style.hovered,
        &options.style.opened,
        &options.style.opened_hovered,
    ]
    .into_iter()
    .flatten()
    {
        let style_span = style.span.as_ref().unwrap_or(span);
        check_container_style_options(&style.options, env, document, style_span, "E129")?;
        for (color, label) in [
            (&style.placeholder_color, "pick placeholder"),
            (&style.handle_color, "pick handle"),
        ] {
            if let Some(color) = color
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E129",
                    style_span,
                    format!("unknown {label} color `{color}`"),
                ));
            }
        }
    }
    check_menu_style(options.menu_style.as_deref(), env, document, span)?;
    Ok(())
}

fn check_menu_style(
    style: Option<&MenuStyleOptions>,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    let Some(style) = style else { return Ok(()) };
    let style_span = style.span.as_ref().unwrap_or(span);
    check_container_style_options(&style.options, env, document, style_span, "E129")?;
    if let Some(color) = &style.selected_text_color
        && !valid_theme_color(color, document)
    {
        return Err(Error::new(
            "E129",
            style_span,
            format!("unknown selected text color `{color}`"),
        ));
    }
    if let Some(background) = &style.selected_background {
        check_background_value(
            background,
            env,
            document,
            style_span,
            "E129",
            "selected background",
        )?;
    }
    Ok(())
}

fn check_text_input_icon(
    icon: Option<&TextInputIcon>,
    env: &HashMap<String, Type>,
    document: &Document,
    widget: &str,
) -> Result<(), Error> {
    let Some(icon) = icon else { return Ok(()) };
    check_font(icon.font.as_ref(), document, &icon.span)?;
    for (value, label) in [
        (&icon.size, format!("{widget} icon size")),
        (&icon.spacing, format!("{widget} icon spacing")),
    ] {
        if let Some(value) = value {
            require_type(
                &expr_type(value, env, document, &icon.span)?,
                &Type::F64,
                &icon.span,
            )?;
            require_literal_range(value, 0.0, None, &label, &icon.span)?;
        }
    }
    Ok(())
}

fn check_text_input_styles(
    styles: &TextInputStyleSet,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    widget: &str,
) -> Result<(), Error> {
    for style in [
        &styles.active,
        &styles.hovered,
        &styles.focused,
        &styles.focused_hovered,
        &styles.disabled,
    ]
    .into_iter()
    .flatten()
    {
        let style_span = style.span.as_ref().unwrap_or(span);
        check_container_style_options(&style.options, env, document, style_span, "E129")?;
        for (color, label) in [
            (&style.icon_color, "icon"),
            (&style.placeholder_color, "placeholder"),
            (&style.value_color, "value"),
            (&style.selection_color, "selection"),
        ] {
            if let Some(color) = color
                && !valid_theme_color(color, document)
            {
                return Err(Error::new(
                    "E129",
                    style_span,
                    format!("unknown {widget} {label} color `{color}`"),
                ));
            }
        }
    }
    Ok(())
}

fn check_scroll_styles(
    styles: &[ScrollStatusStyle],
    env: &HashMap<String, Type>,
    document: &Document,
) -> Result<(), Error> {
    for style in styles {
        for surface in [
            &style.container,
            &style.horizontal_rail.rail,
            &style.horizontal_rail.scroller,
            &style.vertical_rail.rail,
            &style.vertical_rail.scroller,
            &style.auto_scroll,
        ] {
            check_container_style_options(surface, env, document, &style.span, "E129")?;
        }
        if let Some(gap) = &style.gap {
            check_background_value(gap, env, document, &style.span, "E129", "scroll gap")?;
        }
        if let Some(color) = &style.auto_scroll_icon
            && !valid_theme_color(color, document)
        {
            return Err(Error::new(
                "E129",
                &style.span,
                format!("unknown scroll auto icon color `{color}`"),
            ));
        }
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
        for (background, label) in [
            (&style.rail_start, "slider rail start"),
            (&style.rail_end, "slider rail end"),
            (&style.handle_color, "slider handle"),
        ] {
            if let Some(background) = background {
                check_background_value(background, env, document, span, "E129", label)?;
            }
        }
        for color in [&style.rail_border_color, &style.handle_border_color]
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
    if let Some(style) = &options.custom_style {
        let function = extern_function(document, &style.function, ExternKind::TextStyle, span)?;
        check_call_args(function, &style.args, env, document, span)?;
    }
    for length in [&options.width, &options.height].into_iter().flatten() {
        check_length_value(length, env, document, span, "text bounds")?;
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

fn native_subscription_payloads(source: &SubscriptionSource, window_id: bool) -> Option<Vec<Type>> {
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

fn canvas_event_name(source: &SubscriptionSource) -> Option<&'static str> {
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

fn valid_canvas_cursor(value: &str) -> bool {
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

fn infer_subscriptions(
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
            if let Some((output, builtin_error)) = builtin_task_type(*kind, function, args, span)? {
                infer_route(success, Some(output), &unknown_env, document, signatures)?;
                match (builtin_error, error) {
                    (Some(error_ty), Some(route)) => {
                        infer_route(route, Some(error_ty), &unknown_env, document, signatures)?
                    }
                    (Some(_), None) => {
                        return Err(Error::new(
                            "E131",
                            span,
                            "fallible built-in task requires an error route",
                        ));
                    }
                    (None, Some(_)) => {
                        return Err(Error::new(
                            "E131",
                            span,
                            "infallible built-in task cannot have an error route",
                        ));
                    }
                    (None, None) => {}
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
        if contains_debug_span(&ty) {
            return Err(Error::new(
                "E135",
                &route.span,
                "debug spans cannot cross a handler route; use `debug.active(state)` for status",
            ));
        }
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
    operation_ids: &[WidgetIdPath],
    pane_grids: &HashMap<String, PaneGridNames>,
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
                at,
                span,
            } => {
                let expected = states.get(target).ok_or_else(|| {
                    Error::new("E140", span, format!("`{target}` is not writable state"))
                })?;
                if contains_debug_span(expected) {
                    return Err(Error::new(
                        "E144",
                        span,
                        "debug span state is owned by `debug start` and `debug finish`",
                    ));
                }
                let actual = expr_type(value, &env, document, span)?;
                if let Type::Combo(inner) = expected {
                    require_type(&actual, &Type::List(inner.clone()), span)?;
                } else if let Type::Animation(inner) = expected {
                    require_type(&actual, inner, span)?;
                } else {
                    require_type(&actual, expected, span)?;
                }
                if let Some(at) = at {
                    if !matches!(expected, Type::Animation(_)) {
                        return Err(Error::new(
                            "E140",
                            span,
                            "`at` is only valid when assigning animation state",
                        ));
                    }
                    require_type(&expr_type(at, &env, document, span)?, &Type::Instant, span)?;
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
            Statement::Exit { span } => {
                if index + 1 != handler.statements.len() {
                    return Err(Error::new(
                        "E141",
                        span,
                        "exit must be the final statement in a handler",
                    ));
                }
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
                if builtin_task_type(*kind, function, args, span)?.is_some() {
                    if function == "__ice_font_load" {
                        require_type(
                            &expr_type(&args[0], &env, document, span)?,
                            &Type::Bytes,
                            span,
                        )?;
                    } else if function == "__ice_image_allocate" {
                        require_type(
                            &expr_type(&args[0], &env, document, span)?,
                            &Type::Image,
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
            Statement::DebugStart { name, target, span } => {
                require_debug_span_state(target, states, span)?;
                require_type(&expr_type(name, &env, document, span)?, &Type::Str, span)?;
            }
            Statement::DebugFinish { target, span } => {
                require_debug_span_state(target, states, span)?;
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
                let names = pane_grids.get(grid).ok_or_else(|| {
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
                    match pane {
                        PaneReference::Static(pane) => {
                            if !names.panes.contains(pane) {
                                return Err(Error::new(
                                    "E188",
                                    span,
                                    format!("pane-grid `#{grid}` has no pane `{pane}`"),
                                ));
                            }
                        }
                        PaneReference::Dynamic { template, key } => {
                            let expected = names.templates.get(template).ok_or_else(|| {
                                Error::new(
                                    "E188",
                                    span,
                                    format!(
                                        "pane-grid `#{grid}` has no dynamic pane template `{template}`"
                                    ),
                                )
                            })?;
                            require_type(&expr_type(key, &env, document, span)?, expected, span)?;
                        }
                    }
                }
                if let PaneOperation::Resize {
                    split: Some(split), ..
                } = operation
                    && !names.splits.contains(split)
                {
                    return Err(Error::new(
                        "E188",
                        span,
                        format!("pane-grid `#{grid}` has no split `{split}`"),
                    ));
                }
                let same_static = |first: &PaneReference, second: &PaneReference| matches!((first, second), (PaneReference::Static(first), PaneReference::Static(second)) if first == second);
                if matches!(
                    operation,
                    PaneOperation::Swap { first, second } if same_static(first, second)
                ) || matches!(
                    operation,
                    PaneOperation::Drop { pane, target, .. } if same_static(pane, target)
                ) || matches!(
                    operation,
                    PaneOperation::Split { target, pane, .. } if same_static(target, pane)
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
                if let PaneOperation::Resize { ratio, .. } | PaneOperation::Split { ratio, .. } =
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

fn check_structured_tasks(handler: &Handler) -> Result<(), Error> {
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

fn invalid_task_producer(statement: &Statement) -> Option<&Span> {
    match statement {
        Statement::Exit { .. }
        | Statement::Run { .. }
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
        | Statement::DebugStart { .. }
        | Statement::DebugFinish { .. }
        | Statement::PaneOperation { .. } => Some(statement_span(statement)),
    }
}

fn require_task_handle_state(
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

fn require_debug_span_state(
    target: &str,
    states: &HashMap<String, Type>,
    span: &Span,
) -> Result<(), Error> {
    let Some(actual) = states.get(target) else {
        return Err(
            Error::new("E144", span, format!("unknown debug span state `{target}`"))
                .hint(format!("declare `{target}:debug-span? = none` in state")),
        );
    };
    require_type(actual, &Type::Option(Box::new(Type::DebugSpan)), span)
}

fn statement_span(statement: &Statement) -> &Span {
    match statement {
        Statement::Assign { span, .. }
        | Statement::MarkdownAppend { span, .. }
        | Statement::ComboPush { span, .. }
        | Statement::ReturnIf { span, .. }
        | Statement::Exit { span }
        | Statement::Run { span, .. }
        | Statement::Sip { span, .. }
        | Statement::TaskFlow { span, .. }
        | Statement::TaskGroup { span, .. }
        | Statement::Abortable { span, .. }
        | Statement::Abort { span, .. }
        | Statement::DebugStart { span, .. }
        | Statement::DebugFinish { span, .. }
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
                ExternKind::Shader => "shader",
                ExternKind::Task => "task",
                ExternKind::Stream => "stream",
                ExternKind::Sip => "sip",
                ExternKind::Recipe => "recipe",
                ExternKind::Selector => "selector",
                ExternKind::EventFilter => "event filter",
                ExternKind::Sync => "sync function",
                ExternKind::Subscription => "subscription",
                ExternKind::Theme => "theme factory",
                ExternKind::Themer => "themer",
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
                ExternKind::PaneGridStyle => "pane-grid style",
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

fn builtin_task_type(
    kind: EffectKind,
    function: &str,
    args: &[Expr],
    span: &Span,
) -> Result<Option<(Type, Option<Type>)>, Error> {
    let output = match (kind, function) {
        (EffectKind::Task, "__ice_system_info") => Some((Type::SystemInfo, None)),
        (EffectKind::Task, "__ice_system_theme") => Some((Type::Str, None)),
        (EffectKind::Task, "__ice_time_now") => Some((Type::Instant, None)),
        (EffectKind::Task, "__ice_clipboard_read" | "__ice_clipboard_read_primary") => {
            Some((Type::Option(Box::new(Type::Str)), None))
        }
        (EffectKind::Task, "__ice_font_load") => Some((Type::Unit, None)),
        (EffectKind::Task, "__ice_image_allocate") => {
            Some((Type::ImageAllocation, Some(Type::ImageError)))
        }
        _ => None,
    };
    if matches!(function, "__ice_font_load" | "__ice_image_allocate") && args.len() != 1 {
        return Err(Error::new(
            "E142",
            span,
            "this built-in task expects one argument",
        ));
    }
    if output.is_some()
        && !matches!(function, "__ice_font_load" | "__ice_image_allocate")
        && !args.is_empty()
    {
        return Err(Error::new(
            "E142",
            span,
            "this built-in task takes no arguments",
        ));
    }
    Ok(output)
}

fn task_source_type(
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
            if let Some((output, error)) = builtin_task_type(*kind, function, args, span)? {
                if function == "__ice_font_load" {
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Bytes,
                        span,
                    )?;
                } else if function == "__ice_image_allocate" {
                    require_type(
                        &expr_type(&args[0], env, document, span)?,
                        &Type::Image,
                        span,
                    )?;
                }
                return Ok((output, error));
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
            TaskTransform::Map {
                binding,
                value,
                span,
            } => {
                let env = HashMap::from([(binding.clone(), output)]);
                output = expr_type(value, &env, document, span).map_err(|error| {
                    if error.code == "E150" {
                        error.hint(format!("map may only read its `{binding}` binding"))
                    } else {
                        error
                    }
                })?;
            }
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

fn keyboard_variant<'a>(name: &str, args: &'a [Expr], span: &Span) -> Result<&'a str, Error> {
    if args.len() != 1 {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects one string literal"),
        ));
    }
    let Expr::Str(value) = &args[0] else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects a string literal"),
        ));
    };
    let mut chars = value.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_uppercase())
        || !chars.all(|ch| ch.is_ascii_alphanumeric())
    {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects an exact iced Rust variant like `Enter` or `KeyA`"),
        ));
    }
    Ok(value)
}

fn animation_inner(
    expr: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    let Type::Animation(inner) = expr_type(expr, env, document, span)? else {
        return Err(Error::new("E152", span, "expected animation state"));
    };
    Ok(*inner)
}

fn check_animation_instant(
    name: &str,
    args: &[Expr],
    required: usize,
    optional_instant: bool,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    let valid = args.len() == required || optional_instant && args.len() == required + 1;
    if !valid {
        return Err(Error::new(
            "E152",
            span,
            format!(
                "{name} expects {required}{} argument(s)",
                if optional_instant {
                    " or one more instant"
                } else {
                    ""
                }
            ),
        ));
    }
    if args.len() > required {
        require_type(
            &expr_type(&args[required], env, document, span)?,
            &Type::Instant,
            span,
        )?;
    }
    Ok(())
}

fn check_builtin_args(
    name: &str,
    args: &[Expr],
    expected: &[Type],
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if args.len() != expected.len() {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects {} argument(s)", expected.len()),
        ));
    }
    for (value, expected) in args.iter().zip(expected) {
        require_type(&expr_type(value, env, document, span)?, expected, span)?;
    }
    Ok(())
}

fn check_u32_literals(name: &str, args: &[Expr], span: &Span) -> Result<(u32, u32), Error> {
    let [Expr::I64(first), Expr::I64(second)] = args else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects two integer literals"),
        ));
    };
    let (Ok(first), Ok(second)) = (u32::try_from(*first), u32::try_from(*second)) else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} dimensions must be in 0..={}", u32::MAX),
        ));
    };
    Ok((first, second))
}

fn check_u32_literal(name: &str, args: &[Expr], span: &Span) -> Result<u32, Error> {
    let [Expr::I64(value)] = args else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects one integer literal"),
        ));
    };
    u32::try_from(*value).map_err(|_| {
        Error::new(
            "E152",
            span,
            format!("{name} value must be in 0..={}", u32::MAX),
        )
    })
}

fn check_u16_literal(name: &str, args: &[Expr], span: &Span) -> Result<u16, Error> {
    let [Expr::I64(value)] = args else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects one integer literal"),
        ));
    };
    u16::try_from(*value).map_err(|_| {
        Error::new(
            "E152",
            span,
            format!("{name} value must be in 0..={}", u16::MAX),
        )
    })
}

fn check_u8_literals(name: &str, args: &[Expr], count: usize, span: &Span) -> Result<(), Error> {
    if args.len() < count
        || !args[..count]
            .iter()
            .all(|value| matches!(value, Expr::I64(_)))
    {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects {count} integer literal channel(s)"),
        ));
    }
    if args[..count]
        .iter()
        .any(|value| matches!(value, Expr::I64(channel) if u8::try_from(*channel).is_err()))
    {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} channels must be in 0..={}", u8::MAX),
        ));
    }
    Ok(())
}

fn require_pixel_value(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    let actual = expr_type(value, env, document, span)?;
    if matches!(actual, Type::F64 | Type::Pixels) {
        Ok(actual)
    } else {
        Err(Error::new(
            "E101",
            span,
            format!("expected `f64` or `pixels`, got `{}`", actual.display()),
        ))
    }
}

fn check_length_value(
    length: &LengthValue,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    label: &str,
) -> Result<(), Error> {
    let LengthValue::Fixed(value) = length else {
        return Ok(());
    };
    let actual = expr_type(value, env, document, span)?;
    if actual == Type::Length {
        return Ok(());
    }
    if actual != Type::F64 {
        return Err(Error::new(
            "E101",
            span,
            format!(
                "expected `f64` or `length`, got `{}` for {label}",
                actual.display()
            ),
        ));
    }
    require_literal_range(value, 0.0, None, label, span)
}

fn require_radians_value(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    let actual = expr_type(value, env, document, span)?;
    if matches!(actual, Type::F64 | Type::Radians) {
        Ok(actual)
    } else {
        Err(Error::new(
            "E101",
            span,
            format!("expected `f64` or `radians`, got `{}`", actual.display()),
        ))
    }
}

fn arithmetic_type(left: &Type, op: BinaryOp, right: &Type) -> Option<Type> {
    if matches!(left, Type::I64 | Type::F64) && left == right {
        return Some(left.clone());
    }
    match (left, op, right) {
        (Type::Pixels, BinaryOp::Add | BinaryOp::Mul | BinaryOp::Div, Type::Pixels)
        | (Type::Pixels, BinaryOp::Add | BinaryOp::Mul | BinaryOp::Div, Type::F64) => {
            Some(Type::Pixels)
        }
        (Type::Degrees, BinaryOp::Mul, Type::F64) => Some(Type::Degrees),
        (Type::Radians, BinaryOp::Add, Type::Degrees)
        | (
            Type::Radians,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            Type::Radians,
        )
        | (Type::Radians, BinaryOp::Mul | BinaryOp::Div, Type::F64)
        | (Type::F64, BinaryOp::Mul, Type::Radians) => Some(Type::Radians),
        (Type::Point, BinaryOp::Add | BinaryOp::Sub, Type::Vector) => Some(Type::Point),
        (Type::Point, BinaryOp::Sub, Type::Point) => Some(Type::Vector),
        (Type::Vector, BinaryOp::Add | BinaryOp::Sub, Type::Vector) => Some(Type::Vector),
        (Type::Vector, BinaryOp::Mul | BinaryOp::Div, Type::F64) => Some(Type::Vector),
        (Type::Size, BinaryOp::Add | BinaryOp::Sub, Type::Size) => Some(Type::Size),
        (Type::Size, BinaryOp::Mul, Type::Vector) => Some(Type::Size),
        (Type::Size, BinaryOp::Mul | BinaryOp::Div, Type::F64) => Some(Type::Size),
        (Type::Rectangle, BinaryOp::Add | BinaryOp::Sub, Type::Vector) => Some(Type::Rectangle),
        (Type::Rectangle, BinaryOp::Mul, Type::F64) => Some(Type::Rectangle),
        (Type::Transformation, BinaryOp::Mul, Type::Transformation) => Some(Type::Transformation),
        (
            Type::Point
            | Type::Vector
            | Type::Size
            | Type::Rectangle
            | Type::MouseCursor
            | Type::MouseClick,
            BinaryOp::Mul,
            Type::Transformation,
        ) => Some(left.clone()),
        _ => None,
    }
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
        Expr::Bytes(_) => Ok(Type::Bytes),
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
            "color.default" | "color.black" | "color.white" | "color.transparent" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Color)
            }
            "color.rgb" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Color)
            }
            "color.rgba" | "color.linear_rgba" | "color.from4" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::F64, Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Color)
            }
            "color.from3" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Color)
            }
            "color.parse" => {
                check_builtin_args(name, args, &[Type::Str], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Color)))
            }
            "color.rgb8" => {
                if args.len() != 3 {
                    return Err(Error::new("E152", span, "color.rgb8 expects 3 arguments"));
                }
                check_u8_literals(name, args, 3, span)?;
                Ok(Type::Color)
            }
            "color.rgba8" => {
                if args.len() != 4 {
                    return Err(Error::new("E152", span, "color.rgba8 expects 4 arguments"));
                }
                check_u8_literals(name, args, 3, span)?;
                require_type(&expr_type(&args[3], env, document, span)?, &Type::F64, span)?;
                Ok(Type::Color)
            }
            "color.try_rgb8" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::I64, Type::I64, Type::I64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Color)))
            }
            "color.try_rgba8" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::I64, Type::I64, Type::I64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Color)))
            }
            "color.inverse" | "color.invert" => {
                check_builtin_args(name, args, &[Type::Color], env, document, span)?;
                Ok(Type::Color)
            }
            "color.scale_alpha" => {
                check_builtin_args(name, args, &[Type::Color, Type::F64], env, document, span)?;
                Ok(Type::Color)
            }
            "color.luminance" => {
                check_builtin_args(name, args, &[Type::Color], env, document, span)?;
                Ok(Type::F64)
            }
            "color.contrast" => {
                check_builtin_args(name, args, &[Type::Color, Type::Color], env, document, span)?;
                Ok(Type::F64)
            }
            "color.readable" => {
                check_builtin_args(name, args, &[Type::Color, Type::Color], env, document, span)?;
                Ok(Type::Bool)
            }
            "length.fill" | "length.shrink" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Length)
            }
            "length.fill_portion" => {
                check_u16_literal(name, args, span)?;
                Ok(Type::Length)
            }
            "length.try_fill_portion" => {
                check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Length)))
            }
            "length.fixed" | "length.from_f64" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Length)
            }
            "length.from_pixels" => {
                check_builtin_args(name, args, &[Type::Pixels], env, document, span)?;
                Ok(Type::Length)
            }
            "length.from_u32" => {
                check_u32_literal(name, args, span)?;
                Ok(Type::Length)
            }
            "length.try_from_u32" => {
                check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Length)))
            }
            "length.fluid" => {
                check_builtin_args(name, args, &[Type::Length], env, document, span)?;
                Ok(Type::Length)
            }
            "length.enclose" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Length, Type::Length],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Length)
            }
            "alignment.start" | "alignment.center" | "alignment.end" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Alignment)
            }
            "horizontal.left" | "horizontal.center" | "horizontal.right" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::HorizontalAlignment)
            }
            "vertical.top" | "vertical.center" | "vertical.bottom" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::VerticalAlignment)
            }
            "alignment.from_horizontal" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::HorizontalAlignment],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Alignment)
            }
            "alignment.from_vertical" => {
                check_builtin_args(name, args, &[Type::VerticalAlignment], env, document, span)?;
                Ok(Type::Alignment)
            }
            "horizontal.from_alignment" => {
                check_builtin_args(name, args, &[Type::Alignment], env, document, span)?;
                Ok(Type::HorizontalAlignment)
            }
            "vertical.from_alignment" => {
                check_builtin_args(name, args, &[Type::Alignment], env, document, span)?;
                Ok(Type::VerticalAlignment)
            }
            "shadow.default" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Shadow)
            }
            "shadow.new" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Color, Type::Vector, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Shadow)
            }
            "fit.default" | "fit.contain" | "fit.cover" | "fit.fill" | "fit.none"
            | "fit.scale_down" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::ContentFit)
            }
            "fit.apply" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::ContentFit, Type::Size, Type::Size],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Size)
            }
            "rotation.default" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Rotation)
            }
            "rotation.floating" | "rotation.solid" => {
                check_builtin_args(name, args, &[Type::Radians], env, document, span)?;
                Ok(Type::Rotation)
            }
            "rotation.from" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Rotation)
            }
            "rotation.with_radians" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rotation, Type::Radians],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rotation)
            }
            "rotation.apply" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rotation, Type::Size],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Size)
            }
            "debug.active" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Option(Box::new(Type::DebugSpan))],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Bool)
            }
            "debug.time_with" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "debug.time_with expects a name and one value",
                    ));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                let output = expr_type(&args[1], env, document, span)?;
                if contains_debug_span(&output) {
                    return Err(Error::new(
                        "E152",
                        span,
                        "debug.time_with cannot move debug span state",
                    ));
                }
                Ok(output)
            }
            "image.downgrade" => {
                check_builtin_args(name, args, &[Type::ImageAllocation], env, document, span)?;
                Ok(Type::ImageMemory)
            }
            "image.upgrade" => {
                check_builtin_args(name, args, &[Type::ImageMemory], env, document, span)?;
                Ok(Type::Option(Box::new(Type::ImageAllocation)))
            }
            "animation.value" => {
                check_animation_instant(name, args, 1, false, env, document, span)?;
                animation_inner(&args[0], env, document, span)
            }
            "animation.animating" => {
                check_animation_instant(name, args, 1, true, env, document, span)?;
                animation_inner(&args[0], env, document, span)?;
                Ok(Type::Bool)
            }
            "animation.interpolate" => {
                check_animation_instant(name, args, 3, true, env, document, span)?;
                require_type(
                    &animation_inner(&args[0], env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
                let output = expr_type(&args[1], env, document, span)?;
                let output = if output == Type::F64 {
                    Type::F64
                } else {
                    let optional = Type::Option(Box::new(Type::F64));
                    require_type(&output, &optional, span).map_err(|_| {
                        Error::new(
                            "E152",
                            span,
                            "animation.interpolate values must be f64 or f64?",
                        )
                    })?;
                    optional
                };
                require_type(&expr_type(&args[2], env, document, span)?, &output, span)?;
                Ok(output)
            }
            "animation.remaining" => {
                check_animation_instant(name, args, 1, true, env, document, span)?;
                require_type(
                    &animation_inner(&args[0], env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
                Ok(Type::F64)
            }
            "animation.project" => {
                check_animation_instant(name, args, 3, true, env, document, span)?;
                let inner = animation_inner(&args[0], env, document, span)?;
                let Expr::Path(binding) = &args[1] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "animation.project second argument must be a binding name",
                    ));
                };
                if binding.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "animation.project second argument must be a binding name",
                    ));
                }
                let mut projection_env = env.clone();
                projection_env.insert(binding[0].clone(), inner);
                let output = expr_type(&args[2], &projection_env, document, span)?;
                if output != Type::F64 && output != Type::Option(Box::new(Type::F64)) {
                    return Err(Error::new(
                        "E152",
                        span,
                        "animation.project expression must produce f64 or f64?",
                    ));
                }
                Ok(output)
            }
            "pixels" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Pixels)
            }
            "pixels.zero" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Pixels)
            }
            "pixels.from_u32" => {
                check_u32_literal(name, args, span)?;
                Ok(Type::Pixels)
            }
            "pixels.try_from_u32" => {
                check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Pixels)))
            }
            "padding" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::F64, Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Padding)
            }
            "padding.zero" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.all" | "padding.top" | "padding.right" | "padding.bottom" | "padding.left"
            | "padding.horizontal" | "padding.vertical" => {
                if args.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("{name} expects one argument"),
                    ));
                }
                require_pixel_value(&args[0], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.axes" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.from_pixels" => {
                check_builtin_args(name, args, &[Type::Pixels], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.with_top"
            | "padding.with_right"
            | "padding.with_bottom"
            | "padding.with_left"
            | "padding.with_horizontal"
            | "padding.with_vertical" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("{name} expects padding and a pixel value"),
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Padding,
                    span,
                )?;
                require_pixel_value(&args[1], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.fit" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Padding, Type::Size, Type::Size],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Padding)
            }
            "degrees" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Degrees)
            }
            "radians" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Radians)
            }
            "degrees.range_start" | "degrees.range_end" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Degrees)
            }
            "radians.range_start" | "radians.range_end" | "radians.pi" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Radians)
            }
            "degrees.in_range" => {
                check_builtin_args(name, args, &[Type::Degrees], env, document, span)?;
                Ok(Type::Bool)
            }
            "radians.in_range" => {
                check_builtin_args(name, args, &[Type::Radians], env, document, span)?;
                Ok(Type::Bool)
            }
            "radians.from_degrees" => {
                check_builtin_args(name, args, &[Type::Degrees], env, document, span)?;
                Ok(Type::Radians)
            }
            "radians.distance_start" | "radians.distance_end" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Radians, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Point)
            }
            "point" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Point)
            }
            "vector" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Vector)
            }
            "size" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Size)
            }
            "rectangle" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::F64, Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "point.origin" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Point)
            }
            "point.distance" => {
                check_builtin_args(name, args, &[Type::Point, Type::Point], env, document, span)?;
                Ok(Type::F64)
            }
            "point.snap" => {
                check_builtin_args(name, args, &[Type::Point], env, document, span)?;
                Ok(Type::PointU32)
            }
            "vector.zero" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Vector)
            }
            "size.zero" | "size.unit" | "size.infinite" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Size)
            }
            "size.min" | "size.max" | "size.expand" => {
                check_builtin_args(name, args, &[Type::Size, Type::Size], env, document, span)?;
                Ok(Type::Size)
            }
            "size.rotate" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "size.rotate expects a size and radians",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Size,
                    span,
                )?;
                require_radians_value(&args[1], env, document, span)?;
                Ok(Type::Size)
            }
            "size.ratio" => {
                check_builtin_args(name, args, &[Type::Size, Type::F64], env, document, span)?;
                Ok(Type::Size)
            }
            "size.from_vector" => {
                check_builtin_args(name, args, &[Type::Vector], env, document, span)?;
                Ok(Type::Size)
            }
            "vector.from_size" => {
                check_builtin_args(name, args, &[Type::Size], env, document, span)?;
                Ok(Type::Vector)
            }
            "size.from_padding" => {
                check_builtin_args(name, args, &[Type::Padding], env, document, span)?;
                Ok(Type::Size)
            }
            "size.from_u32" => {
                check_u32_literals(name, args, span)?;
                Ok(Type::Size)
            }
            "size.try_from_u32" => {
                check_builtin_args(name, args, &[Type::I64, Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Size)))
            }
            "rectangle.zero" | "rectangle.infinite" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.with_size" => {
                check_builtin_args(name, args, &[Type::Size], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.with_radius" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.with_vertices"
            | "rectangle.vertices_rotation"
            | "rectangle.vertices_angle" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Point, Type::Point, Type::Point],
                    env,
                    document,
                    span,
                )?;
                Ok(match name.as_str() {
                    "rectangle.with_vertices" => Type::Rectangle,
                    "rectangle.vertices_rotation" => Type::F64,
                    _ => Type::Radians,
                })
            }
            "rectangle.contains" | "rectangle.distance" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Point],
                    env,
                    document,
                    span,
                )?;
                Ok(if name == "rectangle.contains" {
                    Type::Bool
                } else {
                    Type::F64
                })
            }
            "rectangle.offset" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Vector)
            }
            "rectangle.is_within" | "rectangle.intersects" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Bool)
            }
            "rectangle.intersection" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Rectangle)))
            }
            "rectangle.union" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.snap" => {
                check_builtin_args(name, args, &[Type::Rectangle], env, document, span)?;
                Ok(Type::Option(Box::new(Type::RectangleU32)))
            }
            "rectangle.expand" | "rectangle.shrink" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::F64, Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.expand_padding" | "rectangle.shrink_padding" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Padding],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.rotate" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.rotate expects a rectangle and radians",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Rectangle,
                    span,
                )?;
                require_radians_value(&args[1], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.zoom" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.anchor" => {
                if args.len() != 4 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor expects a rectangle, size, horizontal alignment, and vertical alignment",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Rectangle,
                    span,
                )?;
                require_type(
                    &expr_type(&args[1], env, document, span)?,
                    &Type::Size,
                    span,
                )?;
                let Expr::Str(horizontal) = &args[2] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor horizontal alignment must be left, center, or right",
                    ));
                };
                let Expr::Str(vertical) = &args[3] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor vertical alignment must be top, center, or bottom",
                    ));
                };
                if !matches!(horizontal.as_str(), "left" | "center" | "right") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor horizontal alignment must be left, center, or right",
                    ));
                }
                if !matches!(vertical.as_str(), "top" | "center" | "bottom") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor vertical alignment must be top, center, or bottom",
                    ));
                }
                Ok(Type::Point)
            }
            "rectangle.from_u32" => {
                check_builtin_args(name, args, &[Type::RectangleU32], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "transform.identity" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.orthographic" => {
                check_u32_literals(name, args, span)?;
                Ok(Type::Transformation)
            }
            "transform.try_orthographic" => {
                check_builtin_args(name, args, &[Type::I64, Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Transformation)))
            }
            "transform.translate" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.scale" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.inverse" => {
                check_builtin_args(name, args, &[Type::Transformation], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.compose" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Transformation, Type::Transformation],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Transformation)
            }
            "transform.point"
            | "transform.vector"
            | "transform.size"
            | "transform.rectangle"
            | "transform.cursor"
            | "transform.click" => {
                let value = match name.as_str() {
                    "transform.point" => Type::Point,
                    "transform.vector" => Type::Vector,
                    "transform.size" => Type::Size,
                    "transform.rectangle" => Type::Rectangle,
                    "transform.cursor" => Type::MouseCursor,
                    "transform.click" => Type::MouseClick,
                    _ => unreachable!(),
                };
                check_builtin_args(
                    name,
                    args,
                    &[value.clone(), Type::Transformation],
                    env,
                    document,
                    span,
                )?;
                Ok(value)
            }
            "mouse.button" => {
                let [Expr::Str(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "mouse.button expects one string literal",
                    ));
                };
                if !matches!(
                    value.as_str(),
                    "left" | "right" | "middle" | "back" | "forward"
                ) {
                    return Err(Error::new(
                        "E152",
                        span,
                        "mouse.button must be left, right, middle, back, or forward",
                    ));
                }
                Ok(Type::MouseButton)
            }
            "mouse.other_button" => {
                let [Expr::I64(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "mouse.other_button expects one integer literal",
                    ));
                };
                if !(0..=u16::MAX as i64).contains(value) {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("mouse.other_button must be in 0..={}", u16::MAX),
                    ));
                }
                Ok(Type::MouseButton)
            }
            "mouse.try_other_button" => {
                check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::MouseButton)))
            }
            "mouse.cursor" | "mouse.levitating" => {
                check_builtin_args(name, args, &[Type::Point], env, document, span)?;
                Ok(Type::MouseCursor)
            }
            "mouse.unavailable" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::MouseCursor)
            }
            "mouse.cursor_position" => {
                check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Point)))
            }
            "mouse.cursor_over" | "mouse.cursor_in" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Point)))
            }
            "mouse.cursor_from" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::Point],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Point)))
            }
            "mouse.cursor_is_over" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Bool)
            }
            "mouse.cursor_is_levitating" => {
                check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                Ok(Type::Bool)
            }
            "mouse.cursor_levitate" | "mouse.cursor_land" => {
                check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                Ok(Type::MouseCursor)
            }
            "mouse.cursor_translate" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::MouseCursor)
            }
            "mouse.click" => {
                check_builtin_args(
                    name,
                    args,
                    &[
                        Type::Point,
                        Type::MouseButton,
                        Type::Option(Box::new(Type::MouseClick)),
                    ],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::MouseClick)
            }
            "touch.finger" => {
                let [Expr::Str(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "touch.finger expects one decimal string literal",
                    ));
                };
                if value.is_empty()
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                    || value.parse::<u64>().is_err()
                {
                    return Err(Error::new(
                        "E152",
                        span,
                        "touch.finger must contain a decimal u64",
                    ));
                }
                Ok(Type::TouchFinger)
            }
            "touch.try_finger" => {
                check_builtin_args(name, args, &[Type::Str], env, document, span)?;
                Ok(Type::Option(Box::new(Type::TouchFinger)))
            }
            "key.named" => {
                keyboard_variant(name, args, span)?;
                Ok(Type::Key)
            }
            "key.character" => {
                if args.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.character expects one str argument",
                    ));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Key)
            }
            "key.unidentified" | "key.native_unidentified" | "key.command_modifiers" => {
                if !args.is_empty() {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("{name} expects no arguments"),
                    ));
                }
                Ok(match name.as_str() {
                    "key.unidentified" => Type::Key,
                    "key.native_unidentified" => Type::PhysicalKey,
                    _ => Type::KeyModifiers,
                })
            }
            "key.code" => {
                keyboard_variant(name, args, span)?;
                Ok(Type::PhysicalKey)
            }
            "key.try_native" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.try_native expects a platform and integer code",
                    ));
                }
                let Expr::Str(platform) = &args[0] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.try_native platform must be a string literal",
                    ));
                };
                if !matches!(platform.as_str(), "android" | "macos" | "windows" | "xkb") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.try_native platform must be android, macos, windows, or xkb",
                    ));
                }
                require_type(&expr_type(&args[1], env, document, span)?, &Type::I64, span)?;
                Ok(Type::Option(Box::new(Type::PhysicalKey)))
            }
            "key.native" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.native expects a platform and integer code",
                    ));
                }
                let Expr::Str(platform) = &args[0] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.native platform must be a string literal",
                    ));
                };
                let Expr::I64(code) = args[1] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.native code must be an integer literal",
                    ));
                };
                let maximum = match platform.as_str() {
                    "android" | "xkb" => u32::MAX as i64,
                    "macos" | "windows" => u16::MAX as i64,
                    _ => {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.native platform must be android, macos, windows, or xkb",
                        ));
                    }
                };
                if !(0..=maximum).contains(&code) {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("key.native {platform} code must be in 0..={maximum}"),
                    ));
                }
                Ok(Type::PhysicalKey)
            }
            "key.location" => {
                let [Expr::Str(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.location expects one string literal",
                    ));
                };
                if !matches!(value.as_str(), "standard" | "left" | "right" | "numpad") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.location must be standard, left, right, or numpad",
                    ));
                }
                Ok(Type::KeyLocation)
            }
            "key.modifiers" => {
                if args.len() != 4 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.modifiers expects shift, control, alt, and logo booleans",
                    ));
                }
                for value in args {
                    require_type(&expr_type(value, env, document, span)?, &Type::Bool, span)?;
                }
                Ok(Type::KeyModifiers)
            }
            "key.latin" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.latin expects a logical key and physical key",
                    ));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Key, span)?;
                require_type(
                    &expr_type(&args[1], env, document, span)?,
                    &Type::PhysicalKey,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Str)))
            }
            "len" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "len expects one argument"));
                }
                match expr_type(&args[0], env, document, span)? {
                    Type::List(_) | Type::Str | Type::Bytes => Ok(Type::I64),
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
                    Type::List(_) | Type::Str | Type::Bytes => Ok(Type::Bool),
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
            "markdown_images" => {
                if args.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "markdown_images expects one argument",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Markdown,
                    span,
                )?;
                Ok(Type::List(Box::new(Type::Str)))
            }
            "editor" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "editor expects one argument"));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Editor)
            }
            "encoded" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "encoded expects one argument"));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Bytes,
                    span,
                )?;
                Ok(Type::Image)
            }
            "rgba" => {
                if args.len() != 3 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rgba expects width, height, and pixel bytes",
                    ));
                }
                for (value, label) in [(&args[0], "rgba width"), (&args[1], "rgba height")] {
                    require_type(&expr_type(value, env, document, span)?, &Type::I64, span)?;
                    require_literal_range(value, 0.0, Some(u32::MAX as f64), label, span)?;
                }
                require_type(
                    &expr_type(&args[2], env, document, span)?,
                    &Type::Bytes,
                    span,
                )?;
                if let (Expr::I64(width), Expr::I64(height), Expr::Bytes(pixels)) =
                    (&args[0], &args[1], &args[2])
                {
                    let expected = (*width as u128) * (*height as u128) * 4;
                    if expected != pixels.len() as u128 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rgba pixel data must contain width × height × 4 bytes",
                        ));
                    }
                }
                Ok(Type::Image)
            }
            "aborted" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "aborted expects one argument"));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Option(Box::new(Type::TaskHandle)),
                    span,
                )?;
                Ok(Type::Bool)
            }
            _ => {
                let function = extern_function(document, name, ExternKind::Sync, span)?;
                check_call_args(function, args, env, document, span)?;
                Ok(function.output.clone())
            }
        },
        Expr::Unary { op, value } => {
            let actual = expr_type(value, env, document, span)?;
            match op {
                UnaryOp::Not => {
                    require_type(&actual, &Type::Bool, span)?;
                    Ok(Type::Bool)
                }
                UnaryOp::Neg if matches!(actual, Type::I64 | Type::F64 | Type::Vector) => {
                    Ok(actual)
                }
                UnaryOp::Neg => Err(Error::new(
                    "E153",
                    span,
                    "negation expects i64, f64, or vector",
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
                    if contains_task_handle(&left) || contains_task_handle(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "task handles are opaque; use `aborted(handle)`",
                        ));
                    }
                    if contains_debug_span(&left) || contains_debug_span(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "debug spans are opaque; use `debug.active(state)`",
                        ));
                    }
                    if contains_mouse_click(&left) || contains_mouse_click(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "mouse-click values are opaque; compare their kind or position",
                        ));
                    }
                    if !matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
                        && matches!(
                            left,
                            Type::Padding
                                | Type::Point
                                | Type::PointU32
                                | Type::Vector
                                | Type::Size
                                | Type::Rectangle
                                | Type::RectangleU32
                                | Type::Transformation
                        )
                    {
                        return Err(Error::new(
                            "E153",
                            span,
                            format!(
                                "operator `{op:?}` does not accept `{}` and `{}`",
                                left.display(),
                                right.display()
                            ),
                        ));
                    }
                    if !matches!((&left, &right), (Type::Degrees | Type::Radians, Type::F64)) {
                        require_type(&left, &right, span)?;
                    }
                    Ok(Type::Bool)
                }
                _ => arithmetic_type(&left, *op, &right).ok_or_else(|| {
                    Error::new(
                        "E153",
                        span,
                        format!(
                            "operator `{op:?}` does not accept `{}` and `{}`",
                            left.display(),
                            right.display()
                        ),
                    )
                }),
            }
        }
    }
}

fn contains_task_handle(ty: &Type) -> bool {
    match ty {
        Type::TaskHandle => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => contains_task_handle(inner),
        Type::Result(output, error) => contains_task_handle(output) || contains_task_handle(error),
        _ => false,
    }
}

fn contains_mouse_click(ty: &Type) -> bool {
    match ty {
        Type::MouseClick => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => contains_mouse_click(inner),
        Type::Result(output, error) => contains_mouse_click(output) || contains_mouse_click(error),
        _ => false,
    }
}

fn field_type(ty: &Type, field: &str, document: &Document, span: &Span) -> Result<Type, Error> {
    if let Type::Option(inner) = ty
        && **inner == Type::WidgetTarget
    {
        return Ok(Type::Option(Box::new(field_type(
            inner, field, document, span,
        )?)));
    }
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
            "key" | "modified_key" => Some(Type::Key),
            "physical_key" => Some(Type::PhysicalKey),
            "location" => Some(Type::KeyLocation),
            "modifiers" => Some(Type::KeyModifiers),
            "text" => Some(Type::Option(Box::new(Type::Str))),
            "repeat" => Some(Type::Bool),
            _ => None,
        },
        Type::KeyRelease => match field {
            "key" | "modified_key" => Some(Type::Key),
            "physical_key" => Some(Type::PhysicalKey),
            "location" => Some(Type::KeyLocation),
            "modifiers" => Some(Type::KeyModifiers),
            _ => None,
        },
        Type::Key => match field {
            "kind" => Some(Type::Str),
            "named" | "character" => Some(Type::Option(Box::new(Type::Str))),
            _ => None,
        },
        Type::PhysicalKey => match field {
            "kind" => Some(Type::Str),
            "code" | "native_platform" => Some(Type::Option(Box::new(Type::Str))),
            "native_code" => Some(Type::Option(Box::new(Type::I64))),
            _ => None,
        },
        Type::KeyLocation => match field {
            "name" => Some(Type::Str),
            _ => None,
        },
        Type::KeyModifiers => match field {
            "shift" | "control" | "alt" | "logo" | "command" | "jump" | "macos_command" => {
                Some(Type::Bool)
            }
            _ => None,
        },
        Type::Pixels => match field {
            "value" => Some(Type::F64),
            _ => None,
        },
        Type::Padding => match field {
            "top" | "right" | "bottom" | "left" | "x" | "y" => Some(Type::F64),
            _ => None,
        },
        Type::Degrees => match field {
            "value" => Some(Type::F64),
            _ => None,
        },
        Type::Radians => match field {
            "value" => Some(Type::F64),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::Rotation => match field {
            "radians" => Some(Type::Radians),
            "degrees" => Some(Type::Degrees),
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::ContentFit => match field {
            "kind" | "display" => Some(Type::Str),
            _ => None,
        },
        Type::Color => match field {
            "r" | "g" | "b" | "a" | "luminance" => Some(Type::F64),
            "rgba8" => Some(Type::List(Box::new(Type::I64))),
            "linear" => Some(Type::List(Box::new(Type::F64))),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::Length => match field {
            "fill_factor" => Some(Type::I64),
            "is_fill" => Some(Type::Bool),
            "kind" => Some(Type::Str),
            "portion" => Some(Type::Option(Box::new(Type::I64))),
            "fixed" => Some(Type::Option(Box::new(Type::F64))),
            _ => None,
        },
        Type::Alignment | Type::HorizontalAlignment | Type::VerticalAlignment => match field {
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::Shadow => match field {
            "color" => Some(Type::Color),
            "offset" => Some(Type::Vector),
            "blur" => Some(Type::F64),
            _ => None,
        },
        Type::Point => match field {
            "x" | "y" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::PointU32 => match field {
            "x" | "y" => Some(Type::I64),
            _ => None,
        },
        Type::Vector => match field {
            "x" | "y" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::Size => match field {
            "width" | "height" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::SizeU32 => match field {
            "width" | "height" => Some(Type::I64),
            _ => None,
        },
        Type::Rectangle => match field {
            "x" | "y" | "width" | "height" => Some(Type::F64),
            "center" | "position" => Some(Type::Point),
            "center_x" | "center_y" | "area" => Some(Type::F64),
            "size" => Some(Type::Size),
            _ => None,
        },
        Type::RectangleU32 => match field {
            "x" | "y" | "width" | "height" => Some(Type::I64),
            _ => None,
        },
        Type::Transformation => match field {
            "scale_factor" => Some(Type::F64),
            "translation" => Some(Type::Vector),
            "matrix" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::MouseButton => match field {
            "kind" => Some(Type::Str),
            "number" => Some(Type::Option(Box::new(Type::I64))),
            _ => None,
        },
        Type::MouseCursor => match field {
            "kind" => Some(Type::Str),
            "position" => Some(Type::Option(Box::new(Type::Point))),
            "levitating" => Some(Type::Bool),
            _ => None,
        },
        Type::MouseClick => match field {
            "kind" => Some(Type::Str),
            "position" => Some(Type::Point),
            _ => None,
        },
        Type::TouchFinger => match field {
            "id" => Some(Type::Str),
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
        Type::ImageAllocation => match field {
            "handle" => Some(Type::Image),
            "size" => Some(Type::SizeU32),
            _ => None,
        },
        Type::ImageError => match field {
            "kind" | "message" => Some(Type::Str),
            _ => None,
        },
        Type::WidgetTarget => match field {
            "kind" => Some(Type::Str),
            "id" => Some(Type::Option(Box::new(Type::WidgetId))),
            "x" | "y" | "width" | "height" => Some(Type::F64),
            "visible_x" | "visible_y" | "visible_width" | "visible_height" | "content_x"
            | "content_y" | "content_width" | "content_height" | "translation_x"
            | "translation_y" => Some(Type::Option(Box::new(Type::F64))),
            "content" => Some(Type::Option(Box::new(Type::Str))),
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
    Container,
    PaneContent,
    PaneTitle,
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
            | StyleTarget::Container
    );
    let is_visual_box =
        is_box || matches!(target, StyleTarget::PaneContent | StyleTarget::PaneTitle);
    let target_name = match target {
        StyleTarget::Layout(Layout::Column) => "col",
        StyleTarget::Layout(Layout::Row) => "row",
        StyleTarget::Layout(Layout::Scroll) => "scroll",
        StyleTarget::Layout(Layout::Grid) => "grid",
        StyleTarget::Layout(Layout::Stack) => "stack",
        StyleTarget::Container => "container",
        StyleTarget::PaneContent => "pane",
        StyleTarget::PaneTitle => "pane title",
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
                "w-full" => matches!(
                    target,
                    StyleTarget::Layout(_) | StyleTarget::Container | StyleTarget::Input
                ),
                "h-full" => matches!(target, StyleTarget::Layout(_) | StyleTarget::Container),
                "max-w-sm" | "max-w-md" | "max-w-lg" | "max-w-xl" | "max-w-2xl" | "self-center" => {
                    is_box
                }
                "items-center" => is_linear,
                "text-xs" | "text-sm" | "text-base" | "text-lg" | "text-xl" | "text-2xl"
                | "font-bold" => matches!(target, StyleTarget::Text),
                "border" | "border-2" => is_visual_box || matches!(target, StyleTarget::Input),
                "rounded-sm" | "rounded" | "rounded-md" | "rounded-lg" | "rounded-full" => {
                    is_visual_box || matches!(target, StyleTarget::Input | StyleTarget::Button)
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
                    is_visual_box || matches!(target, StyleTarget::Input | StyleTarget::Button)
                }
                _ if utility.starts_with("text-") => {
                    is_visual_box || matches!(target, StyleTarget::Text | StyleTarget::Button)
                }
                _ if utility.starts_with("border-") => {
                    is_visual_box || matches!(target, StyleTarget::Input)
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
    if (is_visual_box || matches!(target, StyleTarget::Input)) && has_border_color && !has_border {
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
    if is_visual_box && has_radius && !has_background && !has_border {
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
            (Type::Result(left_output, left_error), Type::Result(right_output, right_error)) => {
                compatible(left_output, right_output) && compatible(left_error, right_error)
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
    use crate::{PaneConfiguration, Type, ViewNode, analyze};

    #[test]
    fn checks_native_alignment_values_and_hashing() {
        let source = include_str!("../../../examples/iced-app/src/ui/alignment.ice");
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "to_vertical = vertical.from_alignment(alignment_round_trip(end))",
            "to_vertical = vertical.from_alignment(right)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `alignment`"));
    }

    #[test]
    fn checks_native_shadow_values_and_fields() {
        let source = include_str!("../../../examples/iced-app/src/ui/shadow.ice");
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "shadow.new(color.rgba(0.1, 0.2, 0.3, 0.4), vector(4.0, 8.0), 12.0)",
            "shadow.new(true, vector(4.0, 8.0), 12.0)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `color`"));
    }

    #[test]
    fn checks_native_length_values_and_widget_passage() {
        let source = include_str!("../../../examples/iced-app/src/ui/length.ice");
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "portion_length = length.fill_portion(3)",
            "portion_length = length.fill_portion(65536)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E152");
        assert!(error.message.contains("0..=65535"));

        let error = analyze(&source.replace(
            "col width=fill_length height=shrink_length",
            "col width=true height=shrink_length",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `f64` or `length`"));

        let error = analyze(&source.replace(
            "grid columns=1 width=96.0",
            "grid columns=1 width=round_trip",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `f64`"));
        assert!(error.message.contains("got `length`"));
    }

    #[test]
    fn checks_native_color_values_and_boundaries() {
        let source = include_str!("../../../examples/iced-app/src/ui/color.ice");
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "rgb8 = color.rgb8(12, 34, 56)",
            "rgb8 = color.rgb8(256, 34, 56)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E152");
        assert!(error.message.contains("channels must be in 0..=255"));

        let error = analyze(&source.replace(
            "contrast = color.contrast(black, white)",
            "contrast = color.contrast(black, true)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `color`"));
    }

    #[test]
    fn checks_native_content_fit_values_and_widgets() {
        let source = include_str!("../../../examples/iced-app/src/ui/content_fit.ice");
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "fit.apply(contain_fit, size(100.0, 50.0), size(80.0, 80.0))",
            "fit.apply(contain_fit, true, size(80.0, 80.0))",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `size`"));

        let error = analyze(&source.replace("fit=round_trip", "fit=true")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `content-fit`"));
    }

    #[test]
    fn checks_native_rotation_values_and_widgets() {
        let source = include_str!("../../../examples/iced-app/src/ui/rotation.ice");
        analyze(source).unwrap();

        let error =
            analyze(&source.replace("rotation.solid(radians(0.5))", "rotation.solid(true)"))
                .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `radians`"));

        let error =
            analyze(&source.replace("rotation=solid_rotation", "rotation=true")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `f64` or `rotation`"));
    }

    #[test]
    fn checks_owned_native_debug_timing_boundaries() {
        let source = include_str!("../../../examples/iced-app/src/ui/debug_timing.ice");
        analyze(source).unwrap();

        let error = analyze(&source.replace("timer:debug-span?", "timer:str?")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("debug-span?"));

        let error = analyze(&source.replace("label = \"interaction\"", "label = 1")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `str`"));

        let error =
            analyze(&source.replace("timer:debug-span?", "timer:[debug-span]")).unwrap_err();
        assert_eq!(error.code, "E103");
        assert!(error.message.contains("must have type `debug-span?`"));

        let error = analyze(&source.replace("debug finish timer", "timer = none")).unwrap_err();
        assert_eq!(error.code, "E144");
        assert!(error.message.contains("`debug start` and `debug finish`"));

        let error = analyze(&source.replace("on begin", "on begin(span)").replace(
            "button \"Begin\" -> begin",
            "button \"Begin\" -> begin timer",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E135");
        assert!(error.message.contains("cannot cross a handler route"));

        let error = analyze(
            &source
                .replace("measured = 0", "measured = 0\n  active = false")
                .replace(
                    "measured = debug.time_with(\"compute\", value + 1)",
                    "active = timer == none",
                ),
        )
        .unwrap_err();
        assert_eq!(error.code, "E153");
        assert!(error.message.contains("debug spans are opaque"));
    }

    #[test]
    fn checks_native_image_allocation_results_and_errors() {
        let source = include_str!("../../../examples/iced-app/src/ui/image_allocation.ice");
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty, Type::ImageAllocation);
        assert_eq!(document.handlers[2].params[0].ty, Type::ImageError);

        let error = analyze(&source.replace(" | failed _", "")).unwrap_err();
        assert_eq!(error.code, "E131");
        assert!(error.message.contains("requires an error route"));

        let error = analyze(&source.replace("allocate handle", "allocate width")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `image`"));
    }

    #[test]
    fn rejects_invalid_animation_boundaries_before_codegen() {
        let source = r#"app Motion
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  label:animation[str] = ""
view
  text "Motion"
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E103");
        assert!(error.message.contains("supports `bool`, `f64`"));

        let source = source
            .replace("label:animation[str] = \"\"", "label = \"\"")
            .replace(
                "view",
                "on change\n  label = \"next\" at instant.now()\nview",
            );
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E140");
        assert!(
            error
                .message
                .contains("only valid when assigning animation")
        );
    }

    #[test]
    fn checks_exit_is_a_final_native_task() {
        let source = r#"daemon Agent
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on quit
  exit
  ready = true
state
  ready = false
view
  button "Quit" -> quit
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E141");
        assert!(error.message.contains("exit must be the final statement"));

        analyze(&source.replace("  exit\n  ready = true", "  exit")).unwrap();
    }

    #[test]
    fn exposes_the_current_window_only_to_daemon_views_and_callbacks() {
        let source = r#"daemon Agent
  title label(window)
  scale-factor scale(window)
extern crate::backend
  sync label(id:window-id) -> str
  sync scale(id:window-id) -> f64
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component WindowBody(id:window-id)
  text "Window"
view
  WindowBody id=window
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "component WindowBody(id:window-id)",
            "state\n  window:window-id? = none\ncomponent WindowBody(id:window-id)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E100");
        assert!(error.message.contains("cannot be named `window`"));

        let error = analyze(&source.replace("daemon Agent", "app Agent")).unwrap_err();
        assert_eq!(error.code, "E150");
        assert!(error.message.contains("unknown value `window`"));
    }

    #[test]
    fn checks_native_timer_subscription() {
        let source = include_str!("../../../examples/iced-app/src/ui/timer.ice");
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty, Type::Instant);
        assert_eq!(document.handlers[2].params[0].ty, Type::I64);
        assert_eq!(document.handlers[2].params[1].ty, Type::I64);
        assert_eq!(document.handlers[3].params[0].ty, Type::I64);
        assert_eq!(document.handlers[3].params[1].ty, Type::Str);
        assert_eq!(document.handlers[4].params[0].ty, Type::Bool);

        let error = analyze(&source.replace(
            "every 250ms when auto_refresh -> tick _",
            "every 250ms when auto_refresh -> tick",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E133");
        assert!(error.message.contains("expects 1 arguments, got 0"));

        let error =
            analyze(&source.replace("refresh_time() -> i64", "refresh_time(seed:i64) -> i64"))
                .unwrap_err();
        assert_eq!(error.code, "E142");

        for invalid in ["0ms", "1m", "1.5s"] {
            let error = analyze(&source.replace("250ms", invalid)).unwrap_err();
            assert_eq!(error.code, "E084");
        }

        let error = analyze(&source.replace("when auto_refresh", "when 1")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `bool`"));

        let error = analyze(&source.replace(
            "every 250ms when auto_refresh",
            "every 250ms status=captured when auto_refresh",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(
            error
                .message
                .contains("only available on non-frame runtime events")
        );

        let error = analyze(&source.replace(
            "sync even_refresh(value:i64) -> i64?",
            "sync even_refresh(value:i64, extra:i64) -> i64?",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E142");
        assert!(error.message.contains("expects 2 payloads, got 1"));

        let error = analyze(&source.replace(
            "sync even_refresh(value:i64) -> i64?",
            "sync even_refresh(value:i64) -> i64",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E142");
        assert!(error.message.contains("must return an optional value"));

        let error = analyze(&source.replace("with=generation", "with=1.5")).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("context must be hashable"));
    }

    #[test]
    fn checks_native_theme_factories() {
        let source = r#"extern crate::backend
  theme native_theme(dark:bool)
app Themes
  theme native_theme(dark)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  dark = true
view
  theme native_theme(!dark)
    text "Nested"
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("native_theme(dark)", "native_theme(1)")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `bool`"));

        let error = analyze(&source.replace("native_theme(!dark)", "missing(!dark)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("theme factory"));
    }

    #[test]
    fn checks_alternate_theme_subtrees() {
        let source = r#"extern crate::backend
  themer alternate_panel(active:bool) -> bool
app Themes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
on changed(value)
  active = value
view
  themer alternate_panel(active) -> changed _
"#;
        analyze(source).unwrap();

        let error =
            analyze(&source.replace("alternate_panel(active)", "alternate_panel(1)")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `bool`"));

        let error = analyze(&source.replace(" -> changed _", "")).unwrap_err();
        assert_eq!(error.code, "E126");
        assert!(error.message.contains("requires a route"));

        let error =
            analyze(&source.replace("themer alternate_panel(active)", "themer missing(active)"))
                .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("unknown extern themer `missing`"));
    }

    #[test]
    fn checks_generic_event_values_and_filters() {
        let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
  sync event_label(value:event) -> str?
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  last = "none"
  last_window:window-id? = none
on received(value)
  last = event_name(value)
on labeled(value)
  last = value
on identified(id, value)
  last_window = some(id)
  last = event_name(value)
subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event raw with-id status=captured -> identified _ _
view
  text last
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty, Type::Event);
        assert_eq!(document.handlers[1].params[0].ty, Type::Str);
        assert_eq!(document.handlers[2].params[0].ty, Type::WindowId);
        assert_eq!(document.handlers[2].params[1].ty, Type::Event);

        let error = analyze(&source.replace(
            "sync event_label(value:event) -> str?",
            "sync event_label(value:str) -> str?",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `str`, got `event`"));
    }

    #[test]
    fn checks_all_native_input_method_subscription_payloads() {
        let source = include_str!("../../../examples/iced-app/src/ui/input_method_events.ice");
        let document = analyze(source).unwrap();
        let preedit = document
            .handlers
            .iter()
            .find(|handler| handler.name == "preedit")
            .unwrap();
        assert_eq!(
            preedit
                .params
                .iter()
                .map(|param| param.ty.display())
                .collect::<Vec<_>>(),
            ["str", "i64?", "i64?"]
        );

        let error = analyze(&source.replace(
            "input-method preedit status=any -> preedit _ _ _",
            "input-method preedit status=any -> preedit _ _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("expects 3 payloads"));

        let error = analyze(&source.replace(
            "input-method closed -> closed",
            "input-method disabled -> closed",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("input-method event must be"));
    }

    #[test]
    fn checks_all_native_mouse_subscription_payloads() {
        let source = include_str!("../../../examples/iced-app/src/ui/mouse_events.ice");
        let document = analyze(source).unwrap();
        let handlers = document
            .handlers
            .iter()
            .map(|handler| {
                (
                    handler.name.as_str(),
                    handler
                        .params
                        .iter()
                        .map(|param| param.ty.display())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(handlers["entered"], Vec::<String>::new());
        assert_eq!(handlers["left"], Vec::<String>::new());
        assert_eq!(handlers["moved"], ["f64", "f64"]);
        assert_eq!(handlers["pressed"], ["mouse-button"]);
        assert_eq!(handlers["released"], ["mouse-button"]);
        assert_eq!(handlers["wheel"], ["f64", "f64", "bool"]);

        let error = analyze(&source.replace(
            "mouse moved status=captured -> moved _ _",
            "mouse moved status=captured -> moved _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("expects 2 payloads"));

        let error = analyze(&source.replace(
            "mouse wheel -> wheel _ _ _",
            "mouse wheel -> wheel 1.0 2.0 true",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E127");

        let error =
            analyze(&source.replace("mouse left -> left", "mouse dragged -> left")).unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("mouse event must be"));

        let error = analyze(&source.replace("status=captured", "status=handled")).unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("status must be"));
    }

    #[test]
    fn checks_all_native_touch_subscription_payloads() {
        let source = include_str!("../../../examples/iced-app/src/ui/touch_events.ice");
        let document = analyze(source).unwrap();
        for handler in &document.handlers {
            assert_eq!(
                handler
                    .params
                    .iter()
                    .map(|param| param.ty.display())
                    .collect::<Vec<_>>(),
                ["touch-finger", "f64", "f64"]
            );
        }

        let error =
            analyze(&source.replace("touch moved -> moved _ _ _", "touch moved -> moved _ _"))
                .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("expects 3 payloads"));

        let error =
            analyze(&source.replace("touch lost -> lost _ _ _", "touch ended -> lost _ _ _"))
                .unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("touch event must be"));
    }

    #[test]
    fn checks_typed_pointer_values() {
        let source = include_str!("../../../examples/iced-app/src/ui/pointer_values.ice");
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty, Type::MouseButton);
        assert_eq!(document.handlers[2].params[0].ty, Type::TouchFinger);

        for (from, to, code, message) in [
            (
                "mouse.button(\"left\")",
                "mouse.button(\"side\")",
                "E152",
                "must be left, right, middle, back, or forward",
            ),
            (
                "mouse.other_button(9)",
                "mouse.other_button(65536)",
                "E152",
                "must be in 0..=65535",
            ),
            (
                "mouse.cursor(point(12.0, 24.0))",
                "mouse.cursor(true)",
                "E101",
                "expected `point`, got `bool`",
            ),
            (
                "mouse.button(\"left\"), none",
                "mouse.button(\"left\"), some(mouse.unavailable())",
                "E101",
                "expected `mouse-click?`, got `mouse-cursor?`",
            ),
            (
                "touch.finger(\"18446744073709551615\")",
                "touch.finger(\"18446744073709551616\")",
                "E152",
                "must contain a decimal u64",
            ),
            (
                "touch.finger(\"18446744073709551615\")",
                "touch.finger(\"+42\")",
                "E152",
                "must contain a decimal u64",
            ),
            (
                "cursor.kind",
                "cursor.missing",
                "E151",
                "has no field `missing`",
            ),
            (
                "cursor_levitating = mouse.cursor_is_levitating(mouse.cursor_levitate(cursor))",
                "cursor_levitating = click == click",
                "E153",
                "mouse-click values are opaque",
            ),
        ] {
            let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
            assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
            assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
        }
    }

    #[test]
    fn checks_native_transformations() {
        let source = include_str!("../../../examples/iced-app/src/ui/transformation_values.ice");
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty, Type::Transformation);
        assert_eq!(document.states[6].ty, Type::Vector);
        assert_eq!(document.states[11].ty, Type::Size);

        for (from, to, code, message) in [
            (
                "transform.identity()",
                "transform.identity(1)",
                "E152",
                "expects 0 argument(s)",
            ),
            (
                "transform.orthographic(640, 480)",
                "transform.orthographic(640.0, 480)",
                "E152",
                "expects two integer literals",
            ),
            (
                "transform.orthographic(640, 480)",
                "transform.orthographic(4294967296, 480)",
                "E152",
                "dimensions must be in 0..=4294967295",
            ),
            (
                "transform.translate(10.0, 20.0)",
                "transform.translate(true, 20.0)",
                "E101",
                "expected `f64`, got `bool`",
            ),
            (
                "transform.scale(2.0))",
                "point(2.0, 2.0))",
                "E101",
                "expected `transformation`, got `point`",
            ),
            (
                "transform.point(point(1.0, 2.0), combined)",
                "transform.point(combined, point(1.0, 2.0))",
                "E101",
                "expected `point`, got `transformation`",
            ),
            (
                "translation = combined.translation",
                "translation = combined.missing",
                "E151",
                "has no field `missing`",
            ),
        ] {
            let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
            assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
            assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
        }
    }

    #[test]
    fn checks_native_geometry_values() {
        let source = include_str!("../../../examples/iced-app/src/ui/geometry_values.ice");
        let document = analyze(source).unwrap();
        assert_eq!(document.functions[0].output, Type::RectangleU32);
        assert_eq!(document.functions[1].params[1].1, Type::PointU32);
        assert_eq!(
            document.functions[1].params[5].1,
            Type::Option(Box::new(Type::RectangleU32))
        );

        for (from, to, code, message) in [
            (
                "point.origin()",
                "point.origin(1)",
                "E152",
                "expects 0 argument(s)",
            ),
            (
                "size.from_u32(640, 480)",
                "size.from_u32(point_distance, 480)",
                "E152",
                "expects two integer literals",
            ),
            (
                "size.from_u32(640, 480)",
                "size.from_u32(4294967296, 480)",
                "E152",
                "dimensions must be in 0..=4294967295",
            ),
            (
                "\"right\", \"bottom\"",
                "\"around\", \"bottom\"",
                "E152",
                "horizontal alignment must be left, center, or right",
            ),
            (
                "point_value = (point.origin() + vector(3.25, 4.75)) - vector.zero()",
                "point_value = point.origin() + point.origin()",
                "E153",
                "does not accept `point` and `point`",
            ),
            (
                "vector_value = ((-vector(1.0, 2.0)",
                "vector_value = ((-point(1.0, 2.0)",
                "E153",
                "negation expects i64, f64, or vector",
            ),
            (
                "snapped_x = snapped_point.x",
                "snapped_x = snapped_point.missing",
                "E151",
                "has no field `missing`",
            ),
            (
                "scaled_bounds = bounds * 2.0",
                "scaled_bounds = bounds / 2.0",
                "E153",
                "does not accept `rectangle` and `f64`",
            ),
            (
                "contains_point = rectangle.contains(bounds, point(20.0, 30.0))",
                "contains_point = point_value < point.origin()",
                "E153",
                "does not accept `point` and `point`",
            ),
        ] {
            let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
            assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
            assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
        }
    }

    #[test]
    fn checks_native_padding_and_angles() {
        let source = include_str!("../../../examples/iced-app/src/ui/padding_angles.ice");
        let document = analyze(source).unwrap();
        assert_eq!(document.functions[0].params[0].1, Type::Pixels);
        assert_eq!(document.functions[0].params[1].1, Type::Padding);
        assert_eq!(document.functions[0].params[2].1, Type::Degrees);
        assert_eq!(document.functions[0].params[3].1, Type::Radians);

        for (from, to, code, message) in [
            (
                "pixels.from_u32(4294967295)",
                "pixels.from_u32(4294967296)",
                "E152",
                "value must be in 0..=4294967295",
            ),
            (
                "pixels.from_u32(4294967295)",
                "pixels.from_u32(pixel_value.value)",
                "E152",
                "expects one integer literal",
            ),
            (
                "all_padding = padding.all(5.0)",
                "all_padding = padding.all(true)",
                "E101",
                "expected `f64` or `pixels`, got `bool`",
            ),
            (
                "all_padding = padding.all(5.0)",
                "all_padding = padding.all(5.0, 6.0)",
                "E152",
                "expects one argument",
            ),
            (
                "padding_equal = direct_padding == padding(1.0, 2.0, 3.0, 4.0)",
                "padding_equal = direct_padding < padding(1.0, 2.0, 3.0, 4.0)",
                "E153",
                "does not accept `padding` and `padding`",
            ),
            (
                "pixel_value = ((((pixels(4.0) + pixels(2.0))",
                "pixel_value = ((((pixels(4.0) - pixels(2.0))",
                "E153",
                "does not accept `pixels` and `pixels`",
            ),
            (
                "degree_value = degrees(45.0) * 2.0",
                "degree_value = degrees(45.0) + degrees(2.0)",
                "E153",
                "does not accept `degrees` and `degrees`",
            ),
            (
                "radians_reverse = 2.0 * radians(1.5)",
                "radians_reverse = 2.0 + radians(1.5)",
                "E153",
                "does not accept `f64` and `radians`",
            ),
            (
                "radians(5.0) % radians(2.0)",
                "radians(5.0) % degrees(2.0)",
                "E153",
                "does not accept `radians` and `degrees`",
            ),
            (
                "rotated_size = size.rotate(size(10.0, 20.0), radians_value)",
                "rotated_size = size.rotate(size(10.0, 20.0), degree_value)",
                "E101",
                "expected `f64` or `radians`, got `degrees`",
            ),
            (
                "radians_equal_scalar = radians_value == 1.0",
                "radians_equal_scalar = 1.0 == radians_value",
                "E101",
                "expected `radians`, got `f64`",
            ),
            (
                "radians_display = radians_value.display",
                "radians_display = radians_value.missing",
                "E151",
                "has no field `missing`",
            ),
        ] {
            let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
            assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
            assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
        }
    }

    #[test]
    fn checks_all_native_window_subscription_payloads() {
        let source = include_str!("../../../examples/iced-app/src/ui/window_events.ice");
        let document = analyze(source).unwrap();
        let opened = document
            .handlers
            .iter()
            .find(|handler| handler.name == "opened")
            .unwrap();
        assert_eq!(
            opened
                .params
                .iter()
                .map(|param| param.ty.display())
                .collect::<Vec<_>>(),
            ["window-id", "f64?", "f64?", "f64", "f64"]
        );

        let error = analyze(&source.replace(
            "window moved with-id status=captured -> moved _ _ _",
            "window moved with-id status=captured -> moved _ _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("expects 3 payloads"));

        let error = analyze(&source.replace(
            "window resized with-id -> resized _ _ _",
            "window resized with-id -> resized 1.0 2.0 3.0",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E127");

        let error = analyze(&source.replace(
            "window frame when listen_frames -> frame",
            "window frame with-id when listen_frames -> frame _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("does not expose a window ID"));
    }

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
    fn checks_structured_task_groups() {
        let source = r#"app Grouped
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  mode = ""
on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _
on theme_read(next)
  mode = next
on clipboard_read(next)
on info_read(info)
view
  text mode
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "str");
        assert_eq!(document.handlers[2].params[0].ty.display(), "str?");
        assert_eq!(document.handlers[3].params[0].ty.display(), "system-info");

        let error = analyze(&source.replace(
            "      task clipboard read -> clipboard_read _",
            "      mode = \"invalid\"",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E143");
        assert!(error.message.contains("task-producing"));

        let error = analyze(&source.replace(
            "on theme_read(next)",
            "  mode = \"too late\"\non theme_read(next)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E141");
        assert!(error.message.contains("final statement"));
    }

    #[test]
    fn checks_native_task_cancellation() {
        let source = r#"app Cancel
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  request:task-handle? = none
  canceled = false
on start
  abortable request abort-on-drop
    task system theme -> loaded _
on loaded(next)
on cancel
  abort request
  canceled = aborted(request)
view
  col
    if aborted(request)
      text "Canceled"
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty.display(), "task-handle?");

        let error = analyze(&source.replace("request:task-handle?", "request:str?")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("task-handle?"));

        let error = analyze(&source.replace("abort request", "abort missing")).unwrap_err();
        assert_eq!(error.code, "E143");
        assert!(error.message.contains("unknown task handle"));

        let error =
            analyze(&source.replace("    task system theme -> loaded _", "    canceled = false"))
                .unwrap_err();
        assert_eq!(error.code, "E143");
        assert!(error.message.contains("task-producing"));

        let error = analyze(&source.replace(
            "  abortable request abort-on-drop\n    task system theme -> loaded _",
            "  parallel\n    abortable request\n      canceled = false\n    task system theme -> loaded _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E143");
        assert_eq!(error.line, 13);

        let error =
            analyze(&source.replace("on loaded(next)", "  canceled = false\non loaded(next)"))
                .unwrap_err();
        assert_eq!(error.code, "E141");
        assert!(error.message.contains("final statement"));

        let error = analyze(&source.replace("aborted(request)", "aborted(true)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace("aborted(request)", "request == none")).unwrap_err();
        assert_eq!(error.code, "E153");
        assert!(error.message.contains("opaque"));
    }

    #[test]
    fn checks_typed_task_streams() {
        let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream coordinates(value:f64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(value:i64) -> str
  event-filter raw_event() -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  count = 0
on start
  parallel
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
on number(value)
  count = value
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(count) -> number _
  recipe snapshot(count) -> text _
  events count using=raw_event -> text _
view
  text count
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "i64");
        assert_eq!(document.handlers[2].params[0].ty.display(), "str");
        assert_eq!(document.handlers[3].params[0].ty.display(), "AppError");
        assert_eq!(
            document.handlers[4].params[0].ty.display(),
            "result[str,AppError]"
        );

        let error = analyze(&source.replace("numbers(3)", "numbers(true)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace(
            "stream fallible() -> text _ | failed _",
            "stream fallible() -> text _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E131");

        let error = analyze(&source.replace(
            "stream numbers(3) -> number _",
            "stream numbers(3) -> number count",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E127");
        assert!(error.message.contains("at most one `_`"));

        let error = analyze(&source.replace(
            "stream numbers(3) -> number _",
            "stream numbers(3) -> number _ _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E127");

        let error = analyze(&source.replace("stream numbers(3)", "stream missing(3)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("extern stream"));

        let error = analyze(&source.replace(
            "run numbers(count) -> number _",
            "run coordinates(1.5) -> number _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("run data must be hashable"));

        let error = analyze(&source.replace("recipe snapshot(count)", "recipe missing(count)"))
            .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("extern recipe"));

        let error =
            analyze(&source.replace("events count using=raw_event", "events 1.5 using=raw_event"))
                .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("event identity must be hashable"));

        let error = analyze(&source.replace("using=raw_event", "using=missing")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("event filter"));
    }

    #[test]
    fn checks_typed_task_sips() {
        let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip transfer(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    sip transfer(3)
      progress -> advanced _
      done -> downloaded _
    sip fallible()
      progress -> counted _
      done -> finished _
      error -> failed _
on advanced(value)
on downloaded(value)
on counted(value)
on finished(value)
on failed(error)
view
  text "Sips"
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty, Type::F64);
        assert_eq!(document.handlers[2].params[0].ty, Type::Bytes);
        assert_eq!(document.handlers[3].params[0].ty, Type::I64);
        assert_eq!(document.handlers[4].params[0].ty, Type::Str);
        assert_eq!(
            document.handlers[5].params[0].ty,
            Type::Named("AppError".into())
        );

        let error = analyze(&source.replace("transfer(3)", "transfer(true)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace("      error -> failed _\n", "")).unwrap_err();
        assert_eq!(error.code, "E131");

        let error = analyze(&source.replace(
            "      progress -> advanced _",
            "      progress -> advanced 1.0",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E127");

        let error = analyze(&source.replace("sip transfer(3)", "sip missing(3)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("extern sip"));
    }

    #[test]
    fn checks_structured_task_flows() {
        let source = r#"app Flows
extern crate::backend
  AppError(message:str)
  OtherError(message:str)
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
  task optional(value:i64) -> i64?
  task fallible(value:i64) -> i64 ! AppError
  task fallible_double(value:i64) -> i64 ! AppError
  task wrong_error(value:i64) -> i64 ! OtherError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  limit = 3
on start
  parallel
    flow
      from stream numbers(limit)
      map value -> value + 1
      then value -> task double(value)
      collect
      done -> collected _
      units -> planned _
    flow
      from task optional(2)
      and-then value -> task double(value)
      done -> finished _
    flow
      from task fallible(2)
      map value -> value + 1
      and-then value -> task fallible_double(value)
      done -> finished _
      error -> failed _
on collected(values)
on planned(units)
on finished(value)
on failed(error)
view
  text "Flows"
"#;
        let document = analyze(source).unwrap();
        assert_eq!(
            document.handlers[1].params[0].ty,
            Type::List(Box::new(Type::I64))
        );
        assert_eq!(document.handlers[2].params[0].ty, Type::I64);
        assert_eq!(document.handlers[3].params[0].ty, Type::I64);
        assert_eq!(
            document.handlers[4].params[0].ty,
            Type::Named("AppError".into())
        );

        let error = analyze(&source.replace(
            "and-then value -> task fallible_double(value)",
            "then value -> task fallible_double(value)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E144");
        assert!(error.message.contains("use and-then"));

        let error = analyze(&source.replace(
            "and-then value -> task fallible_double(value)",
            "and-then value -> task wrong_error(value)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace(
            "then value -> task double(value)",
            "then value -> task double(limit)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E150");
        assert!(error.hint.unwrap().contains("only read its `value`"));

        let error =
            analyze(&source.replacen("map value -> value + 1", "map value -> limit + 1", 1))
                .unwrap_err();
        assert_eq!(error.code, "E150");
        assert_eq!(
            error.hint.as_deref(),
            Some("map may only read its `value` binding")
        );
    }

    #[test]
    fn checks_task_error_mapping_and_native_sources() {
        let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  results:[result[i64,AppError]] = []
on start
  parallel
    flow
      from task request()
      map-error reason -> normalize(reason)
      collect
      done -> collected _
    flow
      from done 1
      then value -> done value + 1
      done -> finished _
    flow
      from none i64
      done -> finished _
on collected(values)
  results = values
on finished(value)
view
  text len(results)
"#;
        let document = analyze(source).unwrap();
        assert_eq!(
            document.handlers[1].params[0].ty,
            Type::List(Box::new(Type::Result(
                Box::new(Type::I64),
                Box::new(Type::Named("AppError".into()))
            )))
        );
        assert_eq!(document.handlers[2].params[0].ty, Type::I64);

        let error = analyze(&source.replace(
            "map-error reason -> normalize(reason)",
            "map-error reason -> normalize(1)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace(
            "from task request()\n      map-error reason -> normalize(reason)",
            "from done 1\n      map-error reason -> normalize(reason)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E144");
        assert!(error.message.contains("fallible"));

        let error = analyze(&source.replace("from none i64", "from none Missing")).unwrap_err();
        assert_eq!(error.code, "E103");
    }

    #[test]
    fn checks_optional_selection_values() {
        let source = r#"app Demo
extern crate::backend
  pick-list-style dynamic_pick(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on opened
view
  pick choices selected placeholder="Choose" line-height=1.2 shaping=advanced font=ui open=opened style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
    active text=foreground placeholder=danger handle=primary background=background border=foreground border-width=1.0 radius=4.0
    hovered text=foreground
    opened text=foreground
    opened-hovered text=foreground
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground shadow=danger shadow-y=2.0
    handle dynamic
      closed code="⌄" font=ui size=12.0 line-height=1.0 shaping=basic
      open code="⌃" font=ui size=12.0 line-height=1.0 shaping=advanced
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[1].ty.display(), "[str]");
        assert_eq!(document.states[2].ty.display(), "str?");
        assert_eq!(document.handlers[0].params[0].ty.display(), "str");

        let error = analyze(&source.replace("size=12.0", "size=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("icon size"));

        let error = analyze(&source.replace("dynamic_pick(busy)", "missing(busy)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("pick-list style"));

        let error = analyze(&source.replace("dynamic_menu(busy)", "missing(busy)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("menu style"));

        let error =
            analyze(&source.replace("dynamic_pick(busy)", "dynamic_pick(1.0)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error =
            analyze(&source.replace("style=dynamic_pick(busy)", "style=primary")).unwrap_err();
        assert_eq!(error.code, "E087");
        assert!(error.message.contains("declared style call"));
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

        let source = source.replace(
            "theme dark text=missing",
            "theme dark background=linear(1.57, background@0.0, missing@1.0)",
        );
        let error = analyze(&source).unwrap_err();
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
component Card(title:str, padded:bool)
  col
    text title
    slot
view
  Card padded=true title="Editor"
    input "Name" <-> draft
"#;
        analyze(source).unwrap();
        analyze(&source.replace(
            "Card padded=true title=\"Editor\"",
            "Card(\"Editor\", true)",
        ))
        .unwrap();

        let error = analyze(&source.replace(
            "  Card padded=true title=\"Editor\"\n    input \"Name\" <-> draft",
            "  Card padded=true title=\"Editor\"",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("requires slot `children`"));

        let error =
            analyze(&source.replace("    text title\n    slot", "    text title")).unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("does not declare slot `children`"));

        let error = analyze(&source.replace("padded=true ", "")).unwrap_err();
        assert_eq!(error.code, "E123");
        assert!(error.message.contains("missing prop `padded`"));

        let error = analyze(&source.replace("padded=true", "raised=true")).unwrap_err();
        assert_eq!(error.code, "E123");
        assert!(error.message.contains("no prop `raised`"));

        let error = analyze(&source.replace("padded=true", "title=\"Again\"")).unwrap_err();
        assert_eq!(error.code, "E123");
        assert!(error.message.contains("prop `title` more than once"));

        let error = analyze(&source.replace("title=\"Editor\"", "title=true")).unwrap_err();
        assert!(error.message.contains("expected `str`, got `bool`"));

        let error = analyze(&source.replace("padded:bool", "title:bool")).unwrap_err();
        assert_eq!(error.code, "E100");
        assert!(error.message.contains("duplicate component prop `title`"));
    }

    #[test]
    fn checks_named_component_slots() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog(title:str)
  col
    slot header
    text title
    slot body
    slot actions
on cancel
on delete
view
  Dialog title="Delete task?"
    header:
      text "Danger zone"
    body:
      col
        text "This cannot be undone."
    actions:
      row
        button "Cancel" -> cancel
        button "Delete" -> delete
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace(
            "    actions:\n      row\n        button \"Cancel\" -> cancel\n        button \"Delete\" -> delete\n",
            "",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("requires slot `actions`"));

        let error = analyze(&source.replace("    actions:", "    footer:")).unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("does not declare slot `footer`"));

        let error = analyze(&source.replace(
            "    body:\n      col\n        text \"This cannot be undone.\"",
            "    body:\n      text \"First\"\n      text \"Second\"",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E040");
        assert!(error.message.contains("slot `body` needs exactly one root"));

        let error = analyze(&source.replace("    slot actions", "    slot body")).unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(
            error
                .message
                .contains("declares slot `body` more than once")
        );
    }

    #[test]
    fn checks_compound_component_slots() {
        let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog()
  col
    slot Header
    slot Body
    slot Actions
component Dialog.Header(title:str)
  col
    text title
    slot
component Dialog.Body()
  container
    slot
component Dialog.Actions()
  row
    slot
on close
view
  Dialog
    Dialog.Header title="About"
      text "Compound title"
    Dialog.Body
      text "Structured body"
    Dialog.Actions
      button "Close" -> close
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("    slot Actions\n", "")).unwrap_err();
        assert_eq!(error.code, "E124");
        assert!(error.message.contains("does not declare slot `Actions`"));

        let error = analyze(&source.replace(
            "    Dialog.Actions\n      button \"Close\" -> close",
            "    text \"not compound\"",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E040");
        assert!(error.message.contains("cannot mix compound components"));

        let error = analyze(&source.replace("Dialog.Header", "Dialog..Header")).unwrap_err();
        assert_eq!(error.code, "E072");
        assert!(error.message.contains("invalid component name"));
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
font ui family=sans
extern crate::backend
  markdown-viewer docs_viewer(prefix:str) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  docs:markdown = "# Hello [world](https://example.com)"
  images:[str] = []
on open(url)
on reset
  docs = markdown("# Reset")
on extend
  markdown docs append "\n![Ice](asset://ice)"
  images = markdown_images(docs)
view
  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 spacing=12.0 viewer=docs_viewer("docs") -> open _
    style font=ui inline-code-background=linear(1.57, background@0.0, primary@1.0) inline-code-color=foreground inline-code-font=mono code-block-font=mono link=primary inline-code-padding=2.0 inline-code-padding-x=3.0 inline-code-padding-y=4.0 inline-code-padding-top=5.0 inline-code-padding-right=6.0 inline-code-padding-bottom=7.0 inline-code-padding-left=8.0 inline-code-border=primary inline-code-border-width=1.0 inline-code-radius=4.0 inline-code-radius-tl=1.0 inline-code-radius-tr=2.0 inline-code-radius-br=3.0 inline-code-radius-bl=4.0
"##;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty.display(), "markdown");
        assert_eq!(document.handlers[0].params[0].ty.display(), "str");

        let error = analyze(&source.replace("spacing=12.0", "spacing=-1.0")).unwrap_err();
        assert!(error.message.contains("outside its valid range"));

        let error = analyze(&source.replace("markdown docs", "markdown missing")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("unknown markdown state"));

        let error = analyze(&source.replace("markdown docs append", "markdown missing append"))
            .unwrap_err();
        assert_eq!(error.code, "E140");
        assert!(error.message.contains("unknown markdown state"));

        let error = analyze(&source.replace(
            "markdown docs append \"\\n![Ice](asset://ice)\"",
            "markdown docs append true",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace("viewer=docs_viewer", "viewer=missing")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("markdown viewer"));

        let error = analyze(&source.replace("link=primary", "link=missing")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("markdown link"));

        let error =
            analyze(&source.replace("markdown_images(docs)", "markdown_images(true)")).unwrap_err();
        assert_eq!(error.code, "E101");
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
    active background=background border=foreground border-width=1.0 radius=4.0 placeholder=danger value=foreground selection=primary
    hovered background=background border=primary placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[0].ty.display(), "editor");

        let error = analyze(&source.replace("min-height=80.0", "min-height=300.0")).unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(error.message.contains("cannot exceed"));

        let error = analyze(&source.replace("placeholder=danger", "icon=danger")).unwrap_err();
        assert_eq!(error.code, "E099");
        assert!(error.message.contains("unknown editor style property"));
    }

    #[test]
    fn checks_component_controlled_state_origins() {
        let source = r#"app Notes
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-style editor_surface(readonly:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = ""
  title = "Notes"
  locked = false
  language = "rs"
component EditorPanel(content:editor, heading:str, readonly:bool, syntax:str)
  col
    input "Title" <-> heading
    editor <-> content highlighter=editor_highlight(syntax) key-binding=editor_keys(readonly) style=editor_surface(readonly) -> command _
on command(value)
view
  EditorPanel(body, title, locked, language)
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty.display(), "EditorCommand");

        let error = analyze(&source.replace(
            "EditorPanel(body, title, locked, language)",
            "EditorPanel(editor(\"scratch\"), title, locked, language)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E139");
        assert!(
            error
                .message
                .contains("editor binding must resolve to an app state")
        );

        let error =
            analyze(&source.replace("editor_keys(readonly)", "missing(readonly)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("editor binding"));

        let error =
            analyze(&source.replace("editor_highlight(syntax)", "missing(syntax)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("editor highlighter"));

        let error =
            analyze(&source.replace("editor_surface(readonly)", "missing(readonly)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("editor style"));
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
        assert!(
            error
                .message
                .contains("declares slot `children` more than once")
        );
    }

    #[test]
    fn checks_combo_search_state_and_routes() {
        let source = r#"app Demo
extern crate::backend
  input-style dynamic_input(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
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
on add
  combo modes push "Timeline"
view
  combo modes selected "Search modes" line-height=1.2 shaping=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground shadow=danger shadow-y=2.0
    icon code="⌕" font=ui size=12.0 spacing=6.0 side=right
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.states[1].ty.display(), "combo[str]");
        assert_eq!(document.handlers[0].params[0].ty.display(), "str");
        assert_eq!(document.handlers[1].params[0].ty.display(), "str");
        assert_eq!(document.handlers[2].params[0].ty.display(), "str");

        let error = analyze(&source.replace("spacing=6.0", "spacing=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("icon spacing"));

        let error = analyze(&source.replace("combo modes push", "combo missing push")).unwrap_err();
        assert_eq!(error.code, "E140");
        assert!(error.message.contains("unknown combo state"));

        let error =
            analyze(&source.replace("combo modes push", "combo selected push")).unwrap_err();
        assert_eq!(error.code, "E140");
        assert!(error.message.contains("not combo state"));

        let error = analyze(&source.replace("push \"Timeline\"", "push 1")).unwrap_err();
        assert_eq!(error.code, "E101");
    }

    #[test]
    fn replaces_combo_search_options_with_a_typed_list() {
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
  modes = ["Timeline"]
on selected(next)
  selected = some(next)
view
  combo modes selected "Search modes" -> selected _
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("[\"Timeline\"]", "[1]")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `[str]`, got `[i64]`"));
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
    float scale=1.1 x=(viewport_x + viewport_width - original_x - original_width) y=(viewport_y + viewport_height - original_y - original_height) shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 radius=8.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
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

        let bad_float_translation = source.replace(
            "x=(viewport_x + viewport_width - original_x - original_width)",
            "x=true",
        );
        let error = analyze(&bad_float_translation).unwrap_err();
        assert!(error.message.contains("expected `f64`, got `bool`"));

        let bad_float_blur = source.replace("shadow-blur=4.0", "shadow-blur=-1.0");
        let error = analyze(&bad_float_blur).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("float style metric"));

        let bad_float_color = source.replace("shadow=black/50", "shadow=missing");
        let error = analyze(&bad_float_color).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("unknown float shadow color"));

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
    fn checks_complete_container_layout() {
        let source = r#"app Boxed
extern crate::backend
  container-style dynamic_container(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  highlight = false
view
  container #card style=dynamic_container(highlight) width=fill height=80.0 max-width=640.0 max-height=120.0 align-x=center align-y=end clip=true padding=8.0 padding-left=12.0 background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true @w-full bg-background border border-foreground rounded-lg
    text "Card"
"#;
        analyze(source).unwrap();

        let bad_metric = source.replace("max-height=120.0", "max-height=-1.0");
        let error = analyze(&bad_metric).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("container metric"));

        let bad_clip = source.replace("clip=true", "clip=1");
        let error = analyze(&bad_clip).unwrap_err();
        assert_eq!(error.code, "E101");

        let bad_style = source.replace("shadow-blur=6.0", "shadow-blur=-1.0");
        let error = analyze(&bad_style).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("surface style metric"));

        let error = analyze(&source.replace("style=dynamic_container(highlight)", "style=rounded"))
            .unwrap_err();
        assert_eq!(error.code, "E184");
        assert!(error.message.contains("container style must be"));

        let error = analyze(&source.replace(
            "dynamic_container(highlight)",
            "missing_container(highlight)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("container style"));

        let error =
            analyze(&source.replace("dynamic_container(highlight)", "dynamic_container(1.0)"))
                .unwrap_err();
        assert_eq!(error.code, "E101");

        let unknown = source.replace("clip=true", "opaque=true");
        let error = analyze(&unknown).unwrap_err();
        assert_eq!(error.code, "E184");
        assert!(error.message.contains("unknown container property"));
    }

    #[test]
    fn checks_structured_overlays() {
        let source = r#"app Dialog
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  shown = true
on close
  shown = false
view
  overlay when=shown dismiss=close backdrop=black/60 padding=24.0 align-x=center align-y=end
    content
      text "Page"
    layer
      container width=320.0 padding=16.0 @bg-background rounded-lg
        text "Dialog"
"#;
        analyze(source).unwrap();

        let wrong_condition = source.replace("when=shown", "when=1");
        let error = analyze(&wrong_condition).unwrap_err();
        assert_eq!(error.code, "E101");

        let bad_padding = source.replace("padding=24.0", "padding=-1.0");
        let error = analyze(&bad_padding).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("overlay padding"));

        let bad_color = source.replace("black/60", "missing/60");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E185");
        assert!(error.message.contains("backdrop color"));

        let unnamed_section = source.replace("    content\n", "    page\n");
        let error = analyze(&unnamed_section).unwrap_err();
        assert_eq!(error.code, "E185");
        assert!(error.message.contains("`content` then `layer`"));
    }

    #[test]
    fn checks_persistent_pane_grids() {
        let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on clicked(name)
view
  pane-grid #work split=vertical ratio=0.7 width=fill height=fill spacing=8.0 min-size=120.0 resize=6.0 drag click=clicked(_)
    pane files
      text "Files"
    pane editor
      text "Editor"
"#;
        analyze(source).unwrap();

        let bad_ratio = source.replace("ratio=0.7", "ratio=2.0");
        let error = analyze(&bad_ratio).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("ratio"));

        let bad_metric = source.replace("min-size=120.0", "min-size=-1.0");
        let error = analyze(&bad_metric).unwrap_err();
        assert_eq!(error.code, "E128");

        let bad_panes = source.replace("pane editor", "panel editor");
        let error = analyze(&bad_panes).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("pane configuration"));
    }

    #[test]
    fn checks_nested_pane_configurations_and_closed_templates() {
        let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on open_preview
  pane #work split editor preview horizontal ratio=0.4
on resize_editor_stack
  pane #work resize editor_stack 0.55
view
  pane-grid #work width=fill height=fill
    split workspace_root vertical ratio=0.7
      pane files
        text "Files"
      split editor_stack horizontal ratio=0.6
        pane editor
          text "Editor"
        pane terminal
          text "Terminal"
    pane preview closed
      text "Preview"
"#;
        let document = analyze(source).unwrap();
        let ViewNode::PaneGrid {
            configuration,
            panes,
            ..
        } = &document.view
        else {
            panic!("pane-grid view")
        };
        assert_eq!(panes.len(), 4);
        assert!(matches!(configuration, PaneConfiguration::Split { .. }));

        let error = analyze(&source.replace("ratio=0.6", "ratio=1.1")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("ratio"));

        let error = analyze(&source.replace("pane terminal", "pane editor")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("duplicate pane `editor`"));

        let error =
            analyze(&source.replace("pane preview closed", "pane preview hidden")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("pane name closed"));

        let error = analyze(&source.replace("resize editor_stack", "resize missing")).unwrap_err();
        assert_eq!(error.code, "E188");
        assert!(error.message.contains("has no split `missing`"));

        let error =
            analyze(&source.replace("editor_stack horizontal", "workspace_root horizontal"))
                .unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(
            error
                .message
                .contains("duplicate pane split `workspace_root`")
        );
    }

    #[test]
    fn checks_runtime_pane_templates_and_keys() {
        let source = r#"app Workspace
extern crate::backend
  Task(id:i64, title:str)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  tasks:[Task] = []
  selected = 7
on open_task
  pane #work split files task(selected) horizontal
on close_task
  pane #work close task(selected)
view
  pane-grid #work
    pane files maximized=files_maximized
      col
        if files_maximized
          text "Maximized files"
    pane task in tasks by=task.id maximized=task_maximized
      col
        if task_maximized
          text "Maximized task"
        text task.title
"#;
        let document = analyze(source).unwrap();
        let ViewNode::PaneGrid { templates, .. } = &document.view else {
            panic!("pane-grid view")
        };
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].item, "task");
        assert_eq!(templates[0].items, "tasks");

        let error = analyze(&source.replace("task(selected)", "task(\"wrong\")")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error =
            analyze(&source.replacen("task(selected)", "missing(selected)", 1)).unwrap_err();
        assert_eq!(error.code, "E188");
        assert!(error.message.contains("no dynamic pane template `missing`"));

        let error = analyze(&source.replace("by=task.id", "by=task")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("dynamic pane keys"));

        let error =
            analyze(&source.replace("maximized=task_maximized", "maximized=task")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("must differ from its template item"));

        let error = analyze(&source.replace("in tasks", "in selected")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("requires list state `selected`"));
    }

    #[test]
    fn checks_structured_pane_titles_and_controls() {
        let source = r#"app Workspace
extern crate::backend
  pane-grid-style dynamic_panes(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  filter = ""
  active = true
on close
view
  pane-grid #work split=vertical style=dynamic_panes(active)
    style
      hovered-region background=linear(0.785, primary/25@0.0, background@0.5, danger@1.0) border=foreground border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
      hovered-split color=primary width=3.0
      picked-split color=danger width=4.0
    pane files background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true @bg-background border border-primary rounded
      title padding=4.0 padding-x=8.0 padding-top=6.0 always-controls background=primary/50 text=foreground border=danger border-width=1.0 radius=3.0 shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=false @bg-primary text-white
        text "Files"
      controls
        button "Close" -> close
      compact-controls
        button "×" -> close
      content
        input "Filter" #filter <-> filter
    pane editor
      title
        text "Editor"
      controls
        button "Close" -> close
      content
        text "Editor body"
"#;
        analyze(source).unwrap();

        let error =
            analyze(&source.replace("style=dynamic_panes(active)", "style=missing_panes(active)"))
                .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("unknown extern pane-grid style"));

        let error = analyze(&source.replace("padding-top=6.0", "padding-top=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("pane title padding"));

        let error =
            analyze(&source.replace("      controls\n        button \"Close\" -> close\n", ""))
                .unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(
            error
                .message
                .contains("compact-controls require a `controls`")
        );

        let error = analyze(&source.replace("      content\n", "      body\n")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(
            error
                .message
                .contains("title, controls, compact-controls, or content")
        );

        let error = analyze(&source.replace("@bg-background", "@p-4 bg-background")).unwrap_err();
        assert_eq!(error.code, "E042");
        assert!(error.message.contains("has no effect on `pane`"));

        let error = analyze(&source.replace("primary/25@0.0", "missing@0.0")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("unknown pane-grid background color"));

        let error = analyze(&source.replace("danger@1.0", "danger@1.1")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("gradient stop"));

        let error = analyze(&source.replace("danger@1.0", "danger")).unwrap_err();
        assert_eq!(error.code, "E189");
        assert!(error.message.contains("color@offset"));

        let error = analyze(&source.replace(
            "linear(0.785, primary/25@0.0, background@0.5, danger@1.0)",
            "linear(0.785, primary@0.0, primary@0.1, primary@0.2, primary@0.3, primary@0.4, primary@0.5, primary@0.6, primary@0.7, primary@1.0)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E189");
        assert!(error.message.contains("at most 8 color stops"));

        let error = analyze(&source.replace("shadow-blur=6.0", "shadow-blur=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("surface style metric"));

        let error = analyze(&source.replace("pixel-snap=true", "pixel-snap=1.0")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace("width=3.0", "width=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("pane-grid style metric"));

        let error =
            analyze(&source.replace("hovered-split color", "active-split color")).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(
            error
                .message
                .contains("hovered-region, hovered-split, or picked-split")
        );
    }

    #[test]
    fn checks_pane_state_operations_and_queries() {
        let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on arrange
  pane #work maximize editor
  pane #work restore
  pane #work swap files editor
  pane #work move editor left
  pane #work resize 0.6
  pane #work drop editor files center
  pane #work split editor preview horizontal ratio=0.4
  pane #work close editor
on inspect
  pane #work maximized -> observed _
on inspect_neighbor
  pane #work adjacent files right -> observed _
on observed(name)
view
  pane-grid #work split=vertical
    pane files
      text "Files"
    pane editor
      text "Editor"
    pane preview closed
      text "Preview"
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[3].params[0].ty.display(), "str?");

        let error = analyze(&source.replace("#work maximize", "#missing maximize")).unwrap_err();
        assert_eq!(error.code, "E188");
        assert!(error.message.contains("unknown pane-grid"));

        let error = analyze(&source.replace("maximize editor", "maximize missing")).unwrap_err();
        assert_eq!(error.code, "E188");
        assert!(error.message.contains("has no pane `missing`"));

        let error = analyze(&source.replace("swap files editor", "swap files files")).unwrap_err();
        assert_eq!(error.code, "E188");
        assert!(error.message.contains("different panes"));

        let error = analyze(&source.replace("resize 0.6", "resize 1.1")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("pane split ratio"));

        let error =
            analyze(&source.replace("pane #work maximized -> observed _", "pane #work maximized"))
                .unwrap_err();
        assert_eq!(error.code, "E188");
        assert!(error.message.contains("query requires a route"));

        let duplicate = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Frame()
  row
    slot left
    slot right
view
  Frame
    left:
      pane-grid #work split=vertical
        pane a
          text "A"
        pane b
          text "B"
    right:
      pane-grid #work split=horizontal
        pane c
          text "C"
        pane d
          text "D"
"#;
        let error = analyze(duplicate).unwrap_err();
        assert_eq!(error.code, "E187");
        assert!(error.message.contains("duplicate persistent pane-grid"));
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
extern crate::backend
  SliderNumber()
  sync slider_number(value:f64) -> SliderNumber
  slider-style dynamic_slider(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  precise:SliderNumber = slider_number(50.0)
  active = true
on changed(next)
  amount = next
on precise_changed(next)
  precise = next
view
  col
    slider amount min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 width=fill(2) height=20.0 style=dynamic_slider(active) -> changed _
      active rail-start=linear(0.0, primary@0.0, danger@1.0) rail-end=linear(1.57, background@0.0, primary/25@1.0) rail-width=4.0 rail-border=transparent rail-border-width=1.0 rail-radius=2.0 rail-radius-tl=1.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, foreground@1.0) handle-border=foreground handle-border-width=1.0
      hovered rail-start=foreground rail-end=background rail-radius-tr=3.0 rail-radius-br=3.0 rail-radius-bl=2.0 handle=rect(12) handle-color=foreground handle-radius=3.0 handle-radius-tl=1.0 handle-radius-tr=2.0 handle-radius-br=3.0 handle-radius-bl=4.0
      dragged rail-start=danger handle=circle(8.0) handle-color=danger
    slider amount min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 vertical width=20.0 height=fill -> changed _
    slider precise min=slider_number(0.0) max=slider_number(100.0) step=slider_number(5.0) default=slider_number(50.0) shift-step=slider_number(1.0) -> precise_changed _
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "SliderNumber");

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

        let bad_color = source.replace("danger@1.0", "missing@1.0");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown slider rail start color"));

        let bad_metric = source.replace("rail-width=4.0", "rail-width=-1.0");
        let error = analyze(&bad_metric).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("slider rail width"));

        let bad_handle = source.replace("handle=rect(12)", "handle=circle(7.0)");
        let error = analyze(&bad_handle).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("requires `handle=rect"));

        let error = analyze(&source.replace("dynamic_slider(active)", "missing_slider(active)"))
            .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("slider style"));

        let error =
            analyze(&source.replace("dynamic_slider(active)", "dynamic_slider(1.0)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error =
            analyze(&source.replace("style=dynamic_slider(active)", "style=primary")).unwrap_err();
        assert_eq!(error.code, "E076");

        let error = analyze(&source.replace("step=slider_number(5.0)", "step=5.0")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace("amount = 50.0", "amount = 50")).unwrap_err();
        assert_eq!(error.code, "E125");
        assert!(error.message.contains("extern numeric type"));
    }

    #[test]
    fn checks_progress_options_and_rejects_invalid_style() {
        let source = r#"app Controls
extern crate::backend
  progress-style dynamic_progress(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  active = true
view
  col
    progress amount min=0.0 max=100.0 length=fill(2) girth=20.0 style=dynamic_progress(active) background=linear(1.57, background@0.0, primary/25@1.0) bar=linear(0.0, primary/75@0.0, danger@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=4.0 radius-bl=5.0
    progress amount vertical length=120.0 girth=fill style=warning
"#;
        analyze(source).unwrap();

        let bad_range = source.replace("min=0.0 max=100.0", "min=101.0 max=100.0");
        let error = analyze(&bad_range).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("progress min cannot exceed max"));

        let bad_color = source.replace("danger@1.0", "missing@1.0");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown progress bar color"));

        let bad_radius = source.replace("radius=4.0", "radius=-1.0");
        let error = analyze(&bad_radius).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("progress radius"));

        let unknown = source.replace("dynamic_progress(active)", "missing(active)");
        let error = analyze(&unknown).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("progress style"));

        let wrong_arg = source.replace("dynamic_progress(active)", "dynamic_progress(amount)");
        let error = analyze(&wrong_arg).unwrap_err();
        assert_eq!(error.code, "E101");

        let malformed = source.replace("dynamic_progress(active)", "unknown");
        let error = analyze(&malformed).unwrap_err();
        assert_eq!(error.code, "E077");
    }

    #[test]
    fn checks_tooltip_style_and_rejects_invalid_values() {
        let source = r#"app Hints
extern crate::backend
  container-style tooltip_surface(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  tooltip position=bottom style=tooltip_surface(active) background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary/75 border-width=1.0 radius=5.0 radius-tl=2.0 radius-tr=3.0 radius-br=4.0 radius-bl=5.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true
    text "Hover"
    text "Tip"
"#;
        analyze(source).unwrap();

        let bad_color = source.replace("shadow=black/50", "shadow=missing");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown tooltip color"));

        let bad_background = source.replace("primary/25@1.0", "missing@1.0");
        let error = analyze(&bad_background).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("unknown tooltip background color"));

        let bad_blur = source.replace("shadow-blur=8.0", "shadow-blur=-1.0");
        let error = analyze(&bad_blur).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("tooltip shadow blur"));

        analyze(&source.replace("style=tooltip_surface(active)", "style=rounded")).unwrap();

        let unknown = source.replace("tooltip_surface(active)", "missing(active)");
        let error = analyze(&unknown).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("container style"));

        let wrong_arg = source.replace("tooltip_surface(active)", "tooltip_surface(1)");
        let error = analyze(&wrong_arg).unwrap_err();
        assert_eq!(error.code, "E101");

        let bad_style = source.replace("style=tooltip_surface(active)", "style=unknown");
        let error = analyze(&bad_style).unwrap_err();
        assert_eq!(error.code, "E086");
        assert!(error.message.contains("declared container style call"));
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
extern crate::backend
  scroll-style dynamic_scroll(busy:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  absolute_x = 0.0
  absolute_y = 0.0
  relative_x = 0.0
  relative_y = 0.0
on scrolled(ax, ay, rx, ry)
  absolute_x = ax
  absolute_y = ay
  relative_x = rx
  relative_y = ry
on viewport(ax, ay, reversed_x, reversed_y, rx, ry, bx, by, bw, bh, cx, cy, cw, ch)
view
  col
    scroll #feed direction=both width=fill height=200.0 bar=hidden bar-width=8.0 bar-margin=2.0 scroller-width=6.0 bar-spacing=4.0 anchor-x=end anchor-y=start auto=true scroll=scrolled style=dynamic_scroll(busy)
      text "Legacy offsets"
    scroll direction=both width=fill height=200.0 viewport=viewport style=dynamic_scroll(busy)
      col
        text "Complete viewport"
      active horizontal-disabled=false vertical-disabled=false
        container background=background text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
        horizontal-rail background=background border=primary border-width=1.0 radius=2.0
        horizontal-scroller background=primary border=foreground border-width=1.0 radius=2.0
        vertical-rail background=background border=primary border-width=1.0 radius=2.0
        vertical-scroller background=primary border=foreground border-width=1.0 radius=2.0
        gap background=background
        auto background=background border=primary border-width=1.0 radius=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 icon=foreground
      hovered horizontal-hovered=true vertical-hovered=false horizontal-disabled=false vertical-disabled=false
        horizontal-scroller background=foreground
      dragged horizontal-dragged=false vertical-dragged=true horizontal-disabled=false vertical-disabled=false
        vertical-scroller background=danger
"#;
        let document = analyze(source).unwrap();
        for param in &document.handlers[0].params {
            assert_eq!(param.ty.display(), "f64");
        }
        assert_eq!(document.handlers[1].params.len(), 14);
        for param in &document.handlers[1].params {
            assert_eq!(param.ty.display(), "f64");
        }

        let error = analyze(&source.replace("horizontal-hovered=true", "horizontal-hovered=maybe"))
            .unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("true or false"));

        let error = analyze(&source.replace(
            "auto=true scroll=scrolled",
            "auto=true scroll=scrolled viewport=viewport",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("either scroll= or viewport="));

        let error = analyze(&source.replace("dynamic_scroll(busy)", "missing(busy)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("scroll style"));

        let error = analyze(&source.replace("dynamic_scroll(busy)", "dynamic_scroll(absolute_x)"))
            .unwrap_err();
        assert_eq!(error.code, "E101");

        let error =
            analyze(&source.replace("style=dynamic_scroll(busy)", "style=primary")).unwrap_err();
        assert_eq!(error.code, "E074");
        assert!(error.message.contains("declared style call"));
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
extern crate::backend
  input-style dynamic_input(disabled:bool)
font ui family=sans
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
  input "Secret" #secret <-> value hint="Paste token" disabled=disabled secure=secure submit=submitted paste=pasted width=240.0 padding=8.0 text-size=14.0 line-height=1.2 align=center font=mono style=dynamic_input(disabled)
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    icon code="•" font=ui size=12.0 spacing=4.0 side=right
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "str");

        let error =
            analyze(&source.replace("dynamic_input(disabled)", "missing(disabled)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("input style"));

        let error = analyze(&source.replace("dynamic_input(disabled)", "dynamic_input(value)"))
            .unwrap_err();
        assert_eq!(error.code, "E101");

        let error =
            analyze(&source.replace("style=dynamic_input(disabled)", "style=primary")).unwrap_err();
        assert_eq!(error.code, "E065");
        assert!(error.message.contains("declared style call"));
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
    fn rejects_negative_input_icon_spacing() {
        let source = r#"app Form
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
view
  input "Value" <-> value
    icon code="+" spacing=-1.0
"#;
        let error = analyze(source).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("input icon spacing"));
    }

    #[test]
    fn checks_button_child_and_typed_properties() {
        let source = r#"app Actions
extern crate::backend
  button-style dynamic_button(disabled:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  disabled = false
on pressed
view
  button #action disabled=disabled width=fill height=48.0 padding=8.0 clip=true style=dynamic_button(disabled) -> pressed
    row
      text "Save"
      text "⌘S"
    active background=linear(1.57, primary@0.0, background@1.0) text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
    hovered background=foreground text=background
    pressed background=primary
    disabled background=background text=foreground
"#;
        analyze(source).unwrap();

        let bad_color = source.replace("border=primary", "border=missing");
        let error = analyze(&bad_color).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("missing"));

        let bad_preset = source.replace("style=dynamic_button(disabled)", "style=tertiary");
        let error = analyze(&bad_preset).unwrap_err();
        assert_eq!(error.code, "E066");
        assert!(error.message.contains("button style must be"));

        let unknown = source.replace("dynamic_button(disabled)", "missing(disabled)");
        let error = analyze(&unknown).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("button style"));

        let wrong_arg = source.replace("dynamic_button(disabled)", "dynamic_button(1.0)");
        let error = analyze(&wrong_arg).unwrap_err();
        assert_eq!(error.code, "E101");
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
    fn checks_complete_boolean_control_styles_and_typography() {
        let source = r#"app Preferences
extern crate::backend
  checkbox-style dynamic_checkbox(disabled:bool)
  toggler-style dynamic_toggler(disabled:bool)
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
    checkbox "Checkbox" checked=enabled style=dynamic_checkbox(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono icon="✓" icon-size=12.0 icon-line-height=1.0 icon-shaping=basic -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) icon=foreground text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0
      active unchecked background=background icon=primary text=foreground border=foreground
      hovered checked background=primary icon=foreground text=foreground border=primary
      hovered unchecked background=foreground icon=background text=primary border=primary
      disabled checked background=background icon=foreground text=foreground border=foreground
      disabled unchecked background=background icon=primary text=foreground border=primary
    toggler "Toggler" checked=enabled style=dynamic_toggler(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=auto wrapping=glyph font=default align=right -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) background-border=primary background-border-width=1.0 foreground=linear(0.0, foreground@0.0, primary@1.0) foreground-border=foreground foreground-border-width=2.0 text=foreground radius=7.0 radius-tl=6.0 radius-tr=7.0 radius-br=8.0 radius-bl=9.0 padding-ratio=0.125
      active unchecked background=background foreground=foreground text=primary
      hovered checked background=primary foreground=foreground text=foreground
      hovered unchecked background=foreground foreground=background text=primary
      disabled checked background=background foreground=foreground text=foreground
      disabled unchecked background=background foreground=primary text=foreground
"#;
        analyze(source).unwrap();

        let error =
            analyze(&source.replace("border=primary border-width", "border=missing border-width"))
                .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("checkbox border color `missing`"));

        let error = analyze(&source.replace("border-width=1.0", "border-width=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("checkbox style metric"));

        let error = analyze(&source.replace("style=dynamic_checkbox(enabled)", "style=warning"))
            .unwrap_err();
        assert_eq!(error.code, "E067");
        assert!(error.message.contains("checkbox style must be"));

        let error =
            analyze(&source.replace("dynamic_checkbox(enabled)", "missing_checkbox(enabled)"))
                .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("checkbox style"));

        let error = analyze(&source.replace("dynamic_checkbox(enabled)", "dynamic_checkbox(1.0)"))
            .unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace("style=dynamic_toggler(enabled)", "style=default"))
            .unwrap_err();
        assert_eq!(error.code, "E075");
        assert!(error.message.contains("toggler style must be"));

        let error =
            analyze(&source.replace("dynamic_toggler(enabled)", "missing_toggler(enabled)"))
                .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("toggler style"));

        let error = analyze(&source.replace("dynamic_toggler(enabled)", "dynamic_toggler(1.0)"))
            .unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace(
            "      active unchecked background=background",
            "      active checked background=background\n      active unchecked background=background",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E067");
        assert!(error.message.contains("duplicate checkbox active checked"));

        let error = analyze(&source.replace(
            "background-border=primary background-border-width",
            "background-border=missing background-border-width",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(
            error
                .message
                .contains("toggler background border color `missing`")
        );

        let error =
            analyze(&source.replace("padding-ratio=0.125", "padding-ratio=0.6")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("toggler padding ratio"));

        let error = analyze(&source.replace(
            "      active unchecked background=background foreground=foreground",
            "      active checked background=background\n      active unchecked background=background foreground=foreground",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E075");
        assert!(error.message.contains("duplicate toggler active checked"));
    }

    #[test]
    fn checks_complete_radio_api_and_generic_values() {
        let source = r#"app Choices
extern crate::backend
  Item(id:i64)
  radio-style dynamic_radio(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  choice = "list"
  items:[Item] = []
  highlight = false
on changed(next)
  choice = next
on float_changed(next)
on item_changed(next)
view
  col
    radio "List" value="list" selected=(choice == "list") style=dynamic_radio(highlight) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono -> changed _
      active selected background=linear(1.57, primary@0.0, background@1.0) dot=foreground border=primary border-width=2.0 text=foreground
      active unselected background=background dot=primary border=foreground text=foreground
      hovered selected background=primary dot=foreground border=foreground text=foreground
      hovered unselected background=foreground dot=background border=primary text=primary
    radio "Float" value=1.5 selected=false -> float_changed _
    for item in items
      radio "Item" value=item selected=false -> item_changed _
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty.display(), "str");
        assert_eq!(document.handlers[1].params[0].ty.display(), "f64");
        assert_eq!(document.handlers[2].params[0].ty.display(), "Item");

        let error =
            analyze(&source.replace("border=primary border-width", "border=missing border-width"))
                .unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("radio border color `missing`"));

        let error = analyze(&source.replace("border-width=2.0", "border-width=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("radio border width"));

        let error = analyze(&source.replace("value=\"list\"", "value=[\"list\"]")).unwrap_err();
        assert_eq!(error.code, "E125");
        assert!(error.message.contains("radio values must be"));

        let error = analyze(&source.replace("style=dynamic_radio(highlight)", "style=default"))
            .unwrap_err();
        assert_eq!(error.code, "E078");
        assert!(error.message.contains("radio style must be"));

        let error =
            analyze(&source.replace("dynamic_radio(highlight)", "missing_radio(highlight)"))
                .unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("radio style"));

        let error =
            analyze(&source.replace("dynamic_radio(highlight)", "dynamic_radio(1.0)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replace(
            "      active unselected background=background",
            "      active selected background=background\n      active unselected background=background",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E078");
        assert!(error.message.contains("duplicate radio active selected"));
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
    fn checks_native_text_style_callbacks() {
        let source = r#"app Typography
extern crate::backend
  text-style dynamic_text(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  col
    text "Styled" style=dynamic_text(active)
    rich-text style=dynamic_text(active)
      span "Rich"
"#;
        analyze(source).unwrap();

        let error =
            analyze(&source.replace("dynamic_text(active)", "missing_text(active)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("text style"));

        let error =
            analyze(&source.replace("dynamic_text(active)", "dynamic_text(1.0)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error = analyze(&source.replacen("style=dynamic_text(active)", "style=primary", 1))
            .unwrap_err();
        assert_eq!(error.code, "E063");

        let rich_only = source.replacen("style=dynamic_text(active)", "", 1);
        let error =
            analyze(&rich_only.replace("style=dynamic_text(active)", "style=primary")).unwrap_err();
        assert_eq!(error.code, "E186");
    }

    #[test]
    fn checks_structured_rich_text_spans() {
        let source = r#"app Typography
font ui family=sans weight=medium stretch=normal style=normal
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
on link(url)
view
  rich-text width=fill height=48.0 size=16.0 line-height=1.2 font=ui align-x=justified align-y=center wrapping=word color=foreground @font-bold -> link _
    span "Ice " size=18.0 line-height-px=22.0 font=ui color=primary background=linear(1.57, background@0.0, primary@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 padding=2.0 padding-left=4.0 underline strike=false
    span "language" link="https://example.com" @text-lg font-bold text-primary
"#;
        analyze(source).unwrap();

        let bad_text = source.replace("span \"Ice \"", "span [\"bad\"]");
        let error = analyze(&bad_text).unwrap_err();
        assert_eq!(error.code, "E186");
        assert!(error.message.contains("span text"));

        let bad_link = source.replace("link=\"https://example.com\"", "link=1");
        let error = analyze(&bad_link).unwrap_err();
        assert_eq!(error.code, "E101");

        let missing_route = source.replace(" @font-bold -> link _", " @font-bold");
        let error = analyze(&missing_route).unwrap_err();
        assert_eq!(error.code, "E186");
        assert!(error.message.contains("require `-> handler _`"));

        let bad_padding = source.replace("padding-left=4.0", "padding-left=-1.0");
        let error = analyze(&bad_padding).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("span padding"));

        let bad_background = source.replace("primary@1.0", "missing@1.0");
        let error = analyze(&bad_background).unwrap_err();
        assert_eq!(error.code, "E186");
        assert!(error.message.contains("missing"));
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
    fn checks_native_shader_programs() {
        let source = r#"app Demo
extern crate::backend
  shader native_shader(value:f64) -> bool
  shader passive_shader() -> unit
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 1.0
on shaded(active)
view
  col
    shader native_shader(amount) width=fill height=64.0 -> shaded _
    shader passive_shader()
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace(" -> shaded _", "")).unwrap_err();
        assert_eq!(error.code, "E191");
        assert!(error.message.contains("requires a route"));

        let error = analyze(&source.replace("height=64.0", "height=-1.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("shader size"));

        let error =
            analyze(&source.replace("native_shader(amount)", "native_shader(true)")).unwrap_err();
        assert!(error.message.contains("expected `f64`"));

        let error = analyze(&source.replace("height=64.0", "depth=64.0")).unwrap_err();
        assert_eq!(error.code, "E191");
        assert!(error.message.contains("unknown shader property"));

        let error = analyze(&source.replace(
            "shader native_shader(value:f64) -> bool",
            "shader native_shader(value:f64) -> bool ! bool",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E023");
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
  key:key = key.unidentified()
  physical:physical-key = key.native_unidentified()
  location:key-location = key.location("standard")
  modifiers:key-modifiers = key.modifiers(false, false, false, false)
  label = ""
  latin:str? = none
  matched = false
  typed:str? = none
  repeat = false
  command = false
on pressed(event)
  key = event.key
  physical = event.physical_key
  location = event.location
  modifiers = event.modifiers
  label = event.key.kind
  latin = key.latin(event.key, event.physical_key)
  matched = event.key == key.named("Enter")
  typed = event.text
  repeat = event.repeat
  command = event.modifiers.command
on released(event)
  physical = event.physical_key
  command = event.modifiers.jump
on modifiers_changed(modifiers)
  command = modifiers.macos_command
subscribe
  keyboard press -> pressed _
  keyboard release -> released _
  keyboard modifiers -> modifiers_changed _
view
  text label
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[0].params[0].ty.display(), "key-press");
        assert_eq!(document.handlers[1].params[0].ty.display(), "key-release");
        assert_eq!(document.handlers[2].params[0].ty.display(), "key-modifiers");

        let error = analyze(&source.replace(
            "on released(event)\n  physical = event.physical_key",
            "on released(event)\n  physical = event.repeat",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E151");
        assert!(error.message.contains("key-release"));

        for (from, to, message) in [
            (
                "key.named(\"Enter\")",
                "key.named(\"enter\")",
                "exact iced Rust variant",
            ),
            (
                "key.location(\"standard\")",
                "key.location(\"middle\")",
                "standard, left, right, or numpad",
            ),
            (
                "key.native_unidentified()",
                "key.native(\"windows\", 65536)",
                "0..=65535",
            ),
            (
                "key.latin(event.key, event.physical_key)",
                "key.latin(event.key, event.location)",
                "expected `physical-key`",
            ),
        ] {
            let error = analyze(&source.replace(from, to)).unwrap_err();
            assert!(error.message.contains(message), "{}", error.message);
        }
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
    fn checks_native_runtime_font_loading() {
        let source = r#"app Fonts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  font_bytes:bytes = bytes(00 01)
on load
  task font load font_bytes -> loaded _
on loaded(result)
view
  text "Fonts"
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[1].params[0].ty.display(), "unit");

        let error = analyze(&source.replace("font load font_bytes", "font load true")).unwrap_err();
        assert_eq!(error.code, "E101");
        assert!(error.message.contains("expected `bytes`"));

        let error =
            analyze(&source.replace(" -> loaded _", " -> loaded _ | loaded _")).unwrap_err();
        assert_eq!(error.code, "E131");
        assert!(error.message.contains("infallible"));
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
    fn checks_all_dynamic_widget_operations() {
        let source = r#"app DynamicOperations
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  ids = [1, 2]
  selected = 1
  value = ""
  focused = false
on focus
  task widget focus #field(selected)
on check
  task widget focused #field(selected) -> checked _
on checked(value)
  focused = value
on front
  task widget cursor-front #field(selected)
on end
  task widget cursor-end #field(selected)
on cursor
  task widget cursor #field(selected) 2
on all
  task widget select-all #field(selected)
on range
  task widget select #field(selected) 1 3
on snap
  task widget snap #list(selected) 0.0 1.0
on snap_end
  task widget snap-end #list(selected)
on scroll_to
  task widget scroll-to #list(selected) 0.0 24.0
on scroll_by
  task widget scroll-by #list(selected) -4.0 8.0
view
  col
    for id in ids
      input "Value" #field(id) <-> value
      scroll #list(id)
        text id
"#;
        let document = analyze(source).unwrap();
        assert_eq!(document.handlers[2].params[0].ty, Type::Bool);

        let error =
            analyze(&source.replacen("focus #field(selected)", "focus #missing(selected)", 1))
                .unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(error.message.contains("#missing(key)"));

        let error = analyze(&source.replace("selected = 1", "selected = \"one\"")).unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(error.message.contains("expects key type `i64`, got `str`"));

        let error = analyze(&source.replacen("focus #field(selected)", "focus #field(true)", 1))
            .unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(error.message.contains("expects key type `i64`, got `bool`"));
    }

    #[test]
    fn checks_scoped_widget_operations() {
        let source = include_str!("../../../examples/iced-app/src/ui/scoped_widget_operations.ice");
        analyze(source).unwrap();

        let error = analyze(&source.replacen("/inner/field", "/inner/missing", 1)).unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(error.message.contains("#outer(key)/inner/missing"));

        let error = analyze(&source.replacen("#outer(selected)", "#outer(value)", 1)).unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(
            error
                .message
                .contains("segment `outer` expects key type `i64`, got `str`")
        );

        let error = analyze(&source.replacen(
            "#row(row_index)/column(column_index)/cell",
            "#column(column_index)/row(row_index)/cell",
            1,
        ))
        .unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(error.message.contains("unknown app widget target"));
    }

    #[test]
    fn checks_widget_selectors() {
        let source = include_str!("../../../examples/iced-app/src/ui/widget_selectors.ice");
        let document = analyze(source).unwrap();
        assert_eq!(
            document.handlers[6].params[0].ty,
            Type::Option(Box::new(Type::WidgetTarget))
        );
        assert_eq!(
            document.handlers[7].params[0].ty,
            Type::List(Box::new(Type::WidgetTarget))
        );
        assert_eq!(
            document.handlers[8].params[0].ty,
            Type::List(Box::new(Type::Str))
        );

        for (before, after, message) in [
            ("find text \"Search\"", "find text 1", "expected `str`"),
            (
                "find point 12.0 24.0",
                "find point true 24.0",
                "expected `f64`",
            ),
            (
                "find id #root/field",
                "find id #root/missing",
                "unknown app widget target",
            ),
            (
                "find-all by_kind(\"text\")",
                "find-all by_kind(1)",
                "expected `str`",
            ),
        ] {
            let error = analyze(&source.replacen(before, after, 1)).unwrap_err();
            assert!(error.message.contains(message), "{}", error.message);
        }

        let error = analyze(&source.replacen(" -> found_one _", "", 1)).unwrap_err();
        assert_eq!(error.code, "E172");
        assert!(error.message.contains("selector requires"));

        let error = analyze(&source.replacen("value.kind", "value.missing", 1)).unwrap_err();
        assert_eq!(error.code, "E151");
        assert!(error.message.contains("has no field `missing`"));
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
    fn rejects_invalid_media_options() {
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

        let valid = source.replace(
            "image \"photo.ppm\" opacity=1.5",
            "image rgba(1, 1, bytes(ff 00 00 ff)) crop=(0, 0, 1, 1)",
        );
        analyze(&valid).unwrap();

        let error = analyze(&valid.replace("bytes(ff 00 00 ff)", "bytes(ff 00 00)")).unwrap_err();
        assert_eq!(error.code, "E152");
        assert!(error.message.contains("width × height × 4"));

        let error = analyze(&valid.replace("crop=(0, 0, 1, 1)", "crop=(-1, 0, 1, 1)")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("crop"));

        let viewer = source.replace(
            "image \"photo.ppm\" opacity=1.5",
            "viewer \"photo.ppm\" padding=8.0 min-scale=0.5 max-scale=4.0 scale-step=0.25",
        );
        analyze(&viewer).unwrap();
        let error = analyze(&viewer.replace("min-scale=0.5", "min-scale=5.0")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("minimum scale"));

        let source = source.replace(
            "image \"photo.ppm\" opacity=1.5",
            "svg \"icon.svg\" color=missing",
        );
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E129");
        assert!(error.message.contains("missing"));

        let source = source.replace(
            "svg \"icon.svg\" color=missing",
            "image \"photo.ppm\" memory",
        );
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E085");
        assert!(error.message.contains("only available on svg"));
    }

    #[test]
    fn checks_svg_style_calls() {
        let source = r#"app Demo
extern crate::backend
  svg-style dynamic_svg(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  svg "icon.svg" style=dynamic_svg(active)
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("dynamic_svg(active)", "missing(active)")).unwrap_err();
        assert_eq!(error.code, "E130");
        assert!(error.message.contains("svg style"));

        let error = analyze(&source.replace("dynamic_svg(active)", "dynamic_svg(1)")).unwrap_err();
        assert_eq!(error.code, "E101");

        let error =
            analyze(&source.replace("style=dynamic_svg(active)", "style=primary")).unwrap_err();
        assert_eq!(error.code, "E085");
        assert!(error.message.contains("declared style call"));

        let error = analyze(&source.replace(
            "svg \"icon.svg\" style=dynamic_svg(active)",
            "image \"icon.svg\" style=dynamic_svg(active)",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E085");
        assert!(error.message.contains("only available on svg"));
    }

    #[test]
    fn rejects_invalid_canvas_programs() {
        let source = r#"app Drawing
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  cached = true
  picture = rgba(1, 1, bytes(ff 00 ff ff))
on pressed(x, y)
on key(value)
view
  canvas width=fill height=120.0 cache=cached cache-group=drawings press=pressed
    event keyboard press -> key _
    redraw window frame after=16ms
    capture touch lost
    circle x=60.0 y=60.0 radius=24.0 fill=primary
    image picture x=4.0 y=4.0 width=16.0 height=16.0 opacity=0.8 snap=true
    svg "<svg/>" memory x=24.0 y=4.0 width=16.0 height=16.0 color=foreground opacity=0.9
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("fill=primary", "fill=missing")).unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("canvas fill"));

        let error = analyze(&source.replace("cache=cached", "cache=1.0")).unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("stable hashing"));

        let error = analyze(&source.replace("cache=cached ", "")).unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("cache-group requires"));

        let error = analyze(&source.replace("opacity=0.8", "opacity=1.1")).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("image opacity"));

        let error = analyze(&source.replace("color=foreground", "color=missing")).unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("svg color"));

        let error = analyze(&source.replace(" radius=24.0", "")).unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("requires `radius=`"));

        let error = analyze(&source.replace(
            "event keyboard press -> key _",
            "event keyboard press -> key _\n    event keyboard press -> key _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("duplicate canvas event"));

        let error =
            analyze(&source.replace("event keyboard press -> key _", "event every 1s -> key _"))
                .unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("canvas events accept"));

        let error = analyze(&source.replace(
            "event keyboard press -> key _",
            "event window focused with-id -> key _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E190");
        assert!(error.message.contains("`with-id` options"));

        let error = analyze(&source.replace("after=16ms", "after=0ms")).unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("positive"));
    }

    #[test]
    fn checks_canvas_local_state_and_event_blocks() {
        let source = r#"app Drawing
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on released(button)
view
  canvas width=fill height=120.0 cursor=(cursor_state) cursor-outside=outside
    state
      cursor_state = "grab"
      outside = false
      hits = 0
    event mouse pressed as button
      set cursor_state = "grabbing"
      set hits = hits + 1
      redraw
      capture
    event mouse released as button
      set cursor_state = "grab"
      emit released button
    text hits x=8.0 y=20.0 color=foreground size=14.0
"#;
        analyze(source).unwrap();

        let error = analyze(&source.replace("hits = 0", "cache = 0")).unwrap_err();
        assert!(error.message.contains("reserved"));

        let error = analyze(&source.replace("outside = false", "hits = 1")).unwrap_err();
        assert!(error.message.contains("duplicate canvas state"));

        let captured = source.replace(
            "  danger #ff0000\n",
            "  danger #ff0000\nstate\n  initial_cursor = \"grab\"\n",
        );
        let error = analyze(&captured.replacen(
            "cursor_state = \"grab\"",
            "cursor_state:str = initial_cursor",
            1,
        ))
        .unwrap_err();
        assert!(error.message.contains("initial_cursor"));

        let error =
            analyze(&source.replace("set hits = hits + 1", "set hits = \"many\"")).unwrap_err();
        assert!(error.message.contains("expected `i64`"));

        let error = analyze(&source.replace("set hits = hits + 1", "set missing = 1")).unwrap_err();
        assert!(error.message.contains("unknown canvas state `missing`"));

        let error = analyze(&source.replace(
            "event mouse released as button",
            "event mouse released as button, extra",
        ))
        .unwrap_err();
        assert!(error.message.contains("exposes 1 values"));

        let error = analyze(&source.replace(
            "      redraw\n      capture",
            "      redraw\n      emit released button\n      capture",
        ))
        .unwrap_err();
        assert!(error.message.contains("one `emit` or `redraw`"));

        let error =
            analyze(&source.replace("emit released button", "emit released _")).unwrap_err();
        assert!(error.message.contains("named bindings"));

        let error = analyze(&source.replace("cursor=(cursor_state)", "cursor=(hits)")).unwrap_err();
        assert!(error.message.contains("expected `str`"));

        let error = analyze(&source.replace("cursor=(cursor_state) ", "")).unwrap_err();
        assert!(error.message.contains("cursor-outside requires"));

        let error =
            analyze(&source.replace("cursor=(cursor_state)", "cursor=(\"bogus\")")).unwrap_err();
        assert!(error.message.contains("unknown canvas cursor"));
    }
}
