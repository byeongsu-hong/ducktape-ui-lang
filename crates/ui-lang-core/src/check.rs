use crate::ast::*;
use crate::{CheckedDocument, Error};
use std::collections::{HashMap, HashSet};

pub fn analyze(mut document: Document) -> Result<CheckedDocument, Error> {
    check(&mut document)?;
    Ok(CheckedDocument::new(document))
}

fn check(document: &mut Document) -> Result<(), Error> {
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
    for component in &document.components {
        for state in &component.states {
            let actual = expr_type(&state.initial, &HashMap::new(), document, &state.span)?;
            if actual != Type::Unknown && !compatible(&state.ty, &actual) {
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
    for component in &document.components {
        for handler in &component.handlers {
            signatures.insert(
                component_handler_key(&component.name, &handler.name),
                vec![None; handler.params.len()],
            );
        }
    }

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
        let mut env: HashMap<String, Type> = component.params.iter().cloned().collect();
        env.extend(
            component
                .states
                .iter()
                .map(|state| (state.name.clone(), state.ty.clone())),
        );
        env.insert(component_context_key(&component.name), Type::Unit);
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
    for component in &mut document.components {
        for handler in &mut component.handlers {
            let key = component_handler_key(&component.name, &handler.name);
            let inferred = signatures.get(&key).expect("component handler signature");
            for (param, inferred) in handler.params.iter_mut().zip(inferred) {
                param.ty = inferred.clone().ok_or_else(|| {
                    Error::new(
                        "E102",
                        &handler.span,
                        format!(
                            "cannot infer type of `{}` in component handler `{}.{}`",
                            param.name, component.name, handler.name
                        ),
                    )
                })?;
            }
        }
    }

    for handler in document.handlers.iter().chain(&preset_handlers) {
        check_handler(handler, &states, document, &operation_ids, &pane_grids)?;
    }
    for component in &document.components {
        let states = component
            .states
            .iter()
            .map(|state| (state.name.clone(), state.ty.clone()))
            .collect();
        for handler in &component.handlers {
            if handler.statements.iter().any(|statement| {
                !matches!(
                    statement,
                    Statement::Assign { .. } | Statement::ReturnIf { .. }
                )
            }) {
                return Err(Error::new(
                    "E140",
                    &handler.span,
                    "component handlers support synchronous state assignments only",
                ));
            }
            check_handler(handler, &states, document, &[], &HashMap::new())?;
        }
    }
    Ok(())
}

const COMPONENT_CONTEXT_PREFIX: &str = "\0component:";

fn component_context_key(component: &str) -> String {
    format!("{COMPONENT_CONTEXT_PREFIX}{component}")
}

fn component_context(env: &HashMap<String, Type>) -> Option<&str> {
    env.keys()
        .find_map(|name| name.strip_prefix(COMPONENT_CONTEXT_PREFIX))
}

fn component_handler_key(component: &str, handler: &str) -> String {
    format!("{component}.{handler}")
}

mod application;
mod canvas;
mod declarations;
mod expr;
mod handler;
mod options;
mod state;
mod style;
mod subscription;
mod view;
mod widgets;

use application::*;
use canvas::*;
use declarations::*;
use handler::*;
use options::*;
pub(crate) use state::controlled_state_bindings;
use state::{check_qr_data, check_theme, pane_grid_span, repeated_pane_grid_span};
use style::*;
use subscription::*;
use view::*;
use widgets::*;

use expr::check_length_value;
pub(crate) use expr::expr_type;
pub(crate) use handler::task_flow_type;

pub(in crate::check) type WidgetIdPath = Vec<(String, Option<Type>)>;

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
