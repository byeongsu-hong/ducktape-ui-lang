use super::*;

#[derive(Clone)]
struct WidgetIdSlot {
    entries: Vec<(String, ViewNode, HashMap<String, Type>)>,
    parent: Option<Box<Self>>,
}

pub(super) fn widget_operation_ids(
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
            ViewNode::PaneGrid { panes, .. } => {
                for pane in panes {
                    let mut pane_scope = scope.clone();
                    pane_scope.push((pane.name.clone(), None));
                    for node in pane.nodes() {
                        collect(node, env, document, &pane_scope, slot, components, output)?;
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

pub(super) fn check_widget_target(
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

pub(super) fn widget_selector_output(
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

pub(super) fn check_widget_selector(
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

pub(super) fn static_pane_grids(
    root: &ViewNode,
) -> Result<HashMap<String, HashSet<String>>, Error> {
    fn collect(
        node: &ViewNode,
        output: &mut HashMap<String, HashSet<String>>,
    ) -> Result<(), Error> {
        match node {
            ViewNode::PaneGrid {
                name, panes, span, ..
            } => {
                if output
                    .insert(
                        name.clone(),
                        panes.iter().map(|pane| pane.name.clone()).collect(),
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
                        collect(node, output)?;
                    }
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output)?;
                }
            }
            ViewNode::Tooltip { content, tip, .. }
            | ViewNode::Overlay {
                content,
                layer: tip,
                ..
            } => {
                collect(content, output)?;
                collect(tip, output)?;
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output)?;
                    collect(&column.cell, output)?;
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
            } => collect(content, output)?,
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output)?;
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output)?;
                    collect(wide, output)?;
                }
                ResponsiveContent::Size { content, .. } => collect(content, output)?,
            },
            _ => {}
        }
        Ok(())
    }
    let mut output = HashMap::new();
    collect(root, &mut output)?;
    Ok(output)
}
