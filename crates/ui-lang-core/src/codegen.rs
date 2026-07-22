use crate::ast::*;
use crate::check::{controlled_state_bindings, expr_type};
use crate::{CheckedDocument, Error};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

pub fn generate(document: &CheckedDocument, source_path: &str) -> Result<String, Error> {
    let message = format!("__{}Message", document.app);
    let mut out = String::new();
    writeln!(
        out,
        "const _: &str = include_str!({});",
        rust_string(source_path)
    )
    .unwrap();
    writeln!(
        out,
        "type __IceRenderer = {}; type __IceElement<'a, Message, Theme = ::iced::Theme> = ::iced::Element<'a, Message, Theme, __IceRenderer>;",
        document
            .settings
            .renderer
            .as_deref()
            .unwrap_or("::iced::Renderer")
    )
    .unwrap();
    generate_keyboard_types(&mut out, document);
    generate_system_types(&mut out, document);
    generate_widget_selector_types(&mut out, document);
    generate_canvas_types(&mut out, document);
    generate_pane_types(&mut out, document)?;

    for component in document
        .components
        .iter()
        .filter(|component| !component.states.is_empty() || !component.handlers.is_empty())
    {
        let ty = component_state_type(&component.name);
        writeln!(out, "#[derive(Debug)]\nstruct {ty} {{").unwrap();
        for state in &component.states {
            writeln!(out, "{}: {},", state.name, state.ty.rust(&document.structs)).unwrap();
        }
        writeln!(
            out,
            "}}\nimpl ::std::default::Default for {ty} {{\nfn default() -> Self {{ Self {{"
        )
        .unwrap();
        for state in &component.states {
            writeln!(out, "{}: {},", state.name, initial_code(state, document)).unwrap();
        }
        writeln!(out, "}} }}\n}}").unwrap();
    }

    writeln!(out, "#[derive(Debug)]\npub struct {} {{", document.app).unwrap();
    writeln!(
        out,
        "pub(crate) __ice_accessibility: ::ui_lang_runtime::Bridge<{message}>,"
    )
    .unwrap();
    if !document.daemon {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\npub(crate) __ice_accessibility_initial: ::std::option::Option<usize>,\n#[cfg(all(target_os = \"windows\", not(test)))]\npub(crate) __ice_accessibility_pending: ::std::vec::Vec<{message}>,"
        )
        .unwrap();
    }
    for qr in &document.qr_codes {
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::qr_code::Data,",
            qr.name
        )
        .unwrap();
    }
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            templates,
            ..
        } = node
        else {
            unreachable!()
        };
        let pane_state = if templates.is_empty() {
            "&'static str".into()
        } else {
            pane_type(name)
        };
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::pane_grid::State<{pane_state}>,",
            pane_field(name)
        )
        .unwrap();
        if pane_split_slots(configuration).iter().any(Option::is_some) {
            writeln!(
                out,
                "pub(crate) {}: ::std::collections::BTreeMap<&'static str, ::iced::widget::pane_grid::Split>,",
                pane_splits_field(name)
            )
            .unwrap();
        }
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
    // ponytail: scoped entries persist for the app lifetime; add active-tree pruning if
    // unbounded dynamic component IDs become a measured source of map growth.
    for component in document
        .components
        .iter()
        .filter(|component| !component.states.is_empty() || !component.handlers.is_empty())
    {
        writeln!(
            out,
            "pub(crate) {}: ::std::collections::HashMap<::std::string::String, {}>,",
            component_state_field(&component.name),
            component_state_type(&component.name)
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();

    writeln!(out, "#[derive(Debug, Clone)]\nenum {message} {{").unwrap();
    writeln!(
        out,
        "__AccessibilitySnapshot(::std::boxed::Box<::ui_lang_runtime::Snapshot<{message}>>),\n__AccessibilityAction(::ui_lang_runtime::ActionRequest),\n__AccessibilityWindow(::iced::window::Id, ::iced::window::Event),\n#[cfg(all(target_os = \"windows\", not(test)))]\n__AccessibilityNativeWindow(::ui_lang_runtime::NativeWindow),\n__AccessibilityFocusNext,\n__AccessibilityFocusPrevious,"
    )
    .unwrap();
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
    for component in &document.components {
        for handler in &component.handlers {
            let variant = component_handler_variant(&component.name, &handler.name);
            let fields = ::std::iter::once("::std::string::String".to_owned())
                .chain(
                    handler
                        .params
                        .iter()
                        .map(|param| param.ty.rust(&document.structs)),
                )
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "{variant}({fields}),").unwrap();
        }
        for state in component
            .states
            .iter()
            .filter(|state| state.ty == Type::Str)
        {
            writeln!(
                out,
                "{}(::std::string::String, ::std::string::String),",
                component_binding_variant(&component.name, &state.name)
            )
            .unwrap();
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
    if has_animations(document) {
        writeln!(out, "__AnimationFrame,").unwrap();
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
    let subscription = ".subscription(Self::__subscription)";
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
    let window = if document.daemon {
        String::new()
    } else {
        window_settings_code(document.settings.window.as_ref(), source_path)
    };
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
    let root = if document.daemon {
        "::iced::daemon(Self::__boot, Self::__update, Self::__view)"
    } else {
        "::iced::application(Self::__boot, Self::__update, Self::__view)"
    };
    writeln!(out, "{root}{title}{subscription}.theme(Self::__theme){style}{settings}{default_font}{fonts}{window}{scale_factor}{executor}{presets}.run()").unwrap();
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
mod canvas;
mod expr;
mod probes;
mod runtime;
mod settings;
mod statement;
mod style;
mod subscription;
mod view;

use application::*;
use canvas::*;
use expr::*;
use probes::*;
use runtime::*;
use settings::*;
use statement::*;
use style::*;
use subscription::*;
use view::*;

#[cfg(test)]
#[path = "codegen/tests.rs"]
mod tests;
