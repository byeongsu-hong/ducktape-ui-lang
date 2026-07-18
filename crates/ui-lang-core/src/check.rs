use crate::Error;
use crate::ast::*;
use std::collections::{HashMap, HashSet};

mod canvas;
mod expr;
mod handler;
mod options;
mod style;
mod subscription;
mod view;

use canvas::*;
use handler::*;
use options::*;
use style::*;
use subscription::*;
use view::*;

pub(crate) use expr::expr_type;
pub(crate) use handler::task_flow_type;

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
    infer_view(&document.view, &states, document, &mut signatures, &mut ids)?;
    let pane_grids = static_pane_grids(&document.view)?;
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
    let operation_ids = widget_operation_ids(&document.view, &states, document)?;
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
    for setting in [
        &document.settings.title,
        &document.settings.background,
        &document.settings.text_color,
    ]
    .into_iter()
    .flatten()
    {
        require_type(
            &expr_type(&setting.value, states, document, &setting.span)?,
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
            check_call_args(factory, args, states, document, &setting.span)?;
        } else {
            require_type(
                &expr_type(&setting.value, states, document, &setting.span)?,
                &Type::Str,
                &setting.span,
            )?;
        }
    }
    if let Some(setting) = &document.settings.scale_factor {
        require_type(
            &expr_type(&setting.value, states, document, &setting.span)?,
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

fn static_pane_grids(root: &ViewNode) -> Result<HashMap<String, HashSet<String>>, Error> {
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
        if let Some(progress) = &item.progress {
            check(progress, &item.span)?;
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
        Type::Result(output, error) => {
            check_declared_type(output, span, known)?;
            check_declared_type(error, span, known)
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
            ViewNode::PaneGrid { panes, .. } => {
                for child in panes.iter().flat_map(PaneView::nodes) {
                    collect(child, document, editors, env, components, output)?;
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
        ViewNode::PaneGrid { panes, .. } => panes
            .iter()
            .flat_map(PaneView::nodes)
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

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
