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

type WidgetIdPath = Vec<(String, Option<Type>)>;

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

mod application;
mod declarations;
mod state;
mod widgets;

use application::*;
use declarations::*;
pub(crate) use state::controlled_state_bindings;
use state::{check_qr_data, check_theme, pane_grid_span, repeated_pane_grid_span};
use widgets::*;

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
