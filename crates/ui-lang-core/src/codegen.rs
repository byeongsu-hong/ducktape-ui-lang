use crate::Error;
use crate::ast::*;
use crate::check::{controlled_state_bindings, expr_type};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

mod canvas;
mod expr;
mod statement;
mod style;
mod view;

use canvas::*;
use expr::*;
use statement::*;
use style::*;
use view::*;

pub fn generate(document: &Document, source_path: &str) -> Result<String, Error> {
    let message = format!("__{}Message", document.app);
    let mut out = String::new();
    writeln!(
        out,
        "const _: &str = include_str!({});",
        rust_string(source_path)
    )
    .unwrap();
    generate_keyboard_types(&mut out, document);
    generate_system_types(&mut out, document);
    generate_widget_selector_types(&mut out, document);
    generate_canvas_types(&mut out, document);

    writeln!(out, "#[derive(Debug)]\npub struct {} {{", document.app).unwrap();
    for qr in &document.qr_codes {
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::qr_code::Data,",
            qr.name
        )
        .unwrap();
    }
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid { name, .. } = node else {
            unreachable!()
        };
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::pane_grid::State<&'static str>,",
            pane_field(name)
        )
        .unwrap();
    }
    for state in &document.states {
        writeln!(
            out,
            "pub(crate) {}: {},",
            state.name,
            state.ty.rust(&document.structs)
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();

    writeln!(out, "#[derive(Debug, Clone)]\nenum {message} {{").unwrap();
    for handler in &document.handlers {
        if handler.name == "mount" {
            continue;
        }
        let variant = pascal(&handler.name);
        if handler.params.is_empty() {
            writeln!(out, "{variant},").unwrap();
        } else {
            let fields = handler
                .params
                .iter()
                .map(|param| param.ty.rust(&document.structs))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "{variant}({fields}),").unwrap();
        }
    }
    for binding in controlled_state_bindings(document, false)
        .expect("checker validates controlled input bindings")
    {
        writeln!(out, "{}(::std::string::String),", binding_variant(&binding)).unwrap();
    }
    for binding in controlled_state_bindings(document, true)
        .expect("checker validates controlled editor bindings")
    {
        writeln!(
            out,
            "{}(::iced::widget::text_editor::Action),",
            editor_variant(&binding)
        )
        .unwrap();
    }
    if needs_extern_noop(document) {
        writeln!(out, "__ExternNoop,").unwrap();
    }
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid { name, options, .. } = node else {
            unreachable!()
        };
        if options.resize_leeway.is_some() {
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::ResizeEvent),",
                pane_resize_variant(name)
            )
            .unwrap();
        }
        if options.draggable {
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::DragEvent),",
                pane_drag_variant(name)
            )
            .unwrap();
        }
    }
    writeln!(out, "}}").unwrap();

    generate_extern_probes(&mut out, document);
    generate_editor_binding_mapper(&mut out, document);
    writeln!(out, "impl {} {{", document.app).unwrap();
    generate_named_windows(&mut out, document, source_path);
    writeln!(out, "pub fn run() -> ::iced::Result {{").unwrap();
    let subscription = if document.subscriptions.is_empty() {
        ""
    } else {
        ".subscription(Self::__subscription)"
    };
    let default_font = document
        .fonts
        .iter()
        .find(|font| font.default)
        .map_or_else(String::new, |font| {
            format!(".default_font({})", font_decl_code(font))
        });
    let title = document
        .settings
        .title
        .as_ref()
        .map_or("", |_| ".title(Self::__title)");
    let settings = app_settings_code(&document.settings);
    let fonts = font_assets_code(&document.settings, source_path);
    let window = window_settings_code(document.settings.window.as_ref(), source_path);
    let executor = document
        .settings
        .executor
        .as_ref()
        .map_or_else(String::new, |executor| format!(".executor::<{executor}>()"));
    let presets = if document.presets.is_empty() {
        String::new()
    } else {
        format!(
            ".presets([{}])",
            document
                .presets
                .iter()
                .enumerate()
                .map(|(index, preset)| format!(
                    "::iced::Preset::new({}, Self::__preset_{index})",
                    rust_string(&preset.name)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let scale_factor = document
        .settings
        .scale_factor
        .as_ref()
        .map_or("", |_| ".scale_factor(Self::__scale_factor)");
    let style = if document.settings.background.is_some() || document.settings.text_color.is_some()
    {
        ".style(Self::__style)"
    } else {
        ""
    };
    writeln!(out, "::iced::application(Self::__boot, Self::__update, Self::__view){title}{subscription}.theme(Self::__theme){style}{settings}{default_font}{fonts}{window}{scale_factor}{executor}{presets}.run()").unwrap();
    writeln!(out, "}}").unwrap();

    generate_theme(&mut out, document)?;
    generate_boot(&mut out, document, &message)?;
    generate_presets(&mut out, document, &message)?;
    generate_update(&mut out, document, &message)?;
    generate_subscription(&mut out, document, &message)?;
    generate_view(&mut out, document, &message)?;
    writeln!(out, "}}").unwrap();
    Ok(out)
}

mod application;
mod probes;
mod runtime;
mod settings;
mod subscription;

use application::*;
use probes::*;
use runtime::*;
use settings::*;
use subscription::*;

#[cfg(test)]
#[path = "codegen/tests.rs"]
mod tests;
