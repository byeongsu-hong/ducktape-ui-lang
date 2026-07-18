use crate::Error;
use crate::ast::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
struct Line {
    number: usize,
    indent: usize,
    text: String,
    children: Vec<Line>,
}

pub fn parse(source: &str) -> Result<Document, Error> {
    let lines = line_tree(source)?;
    let mut app = None;
    let mut daemon = false;
    let mut settings = AppSettings::default();
    let mut presets = Vec::new();
    let mut extern_path = None;
    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut subscriptions = Vec::new();
    let mut theme = BTreeMap::new();
    let mut fonts = Vec::new();
    let mut qr_codes = Vec::new();
    let mut states = Vec::new();
    let mut components = Vec::new();
    let mut handlers = Vec::new();
    let mut view = None;

    for line in &lines {
        let root = line
            .text
            .strip_prefix("app ")
            .map(|name| (name, false))
            .or_else(|| line.text.strip_prefix("daemon ").map(|name| (name, true)));
        if let Some((name, is_daemon)) = root {
            if app.replace(identifier(name.trim(), line)?).is_some() {
                return Err(error(
                    "E002",
                    line,
                    "an app or daemon may only be declared once",
                ));
            }
            settings = parse_app_settings(line)?;
            if is_daemon
                && let Some(window) = line.children.iter().find(|item| item.text == "window")
            {
                return Err(error(
                    "E014",
                    window,
                    "a daemon has no initial window",
                )
                .hint("declare a named `window name` and open it with `task window open name -> handler _`"));
            }
            daemon = is_daemon;
        } else if let Some(name) = line.text.strip_prefix("preset ") {
            presets.push(parse_preset(name.trim(), line)?);
        } else if let Some(path) = line.text.strip_prefix("extern ") {
            if extern_path.is_some() {
                return Err(error(
                    "E003",
                    line,
                    "only one extern namespace is supported",
                ));
            }
            let path = rust_path(path.trim(), line)?;
            extern_path = Some(path.clone());
            for item in &line.children {
                if let Some(source) = item.text.strip_prefix("component ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Component)?);
                } else if let Some(source) = item.text.strip_prefix("shader ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Shader)?);
                } else if let Some(source) = item.text.strip_prefix("task ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Task)?);
                } else if let Some(source) = item.text.strip_prefix("stream ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Stream)?);
                } else if let Some(source) = item.text.strip_prefix("sip ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Sip)?);
                } else if let Some(source) = item.text.strip_prefix("recipe ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Recipe)?);
                } else if let Some(source) = item.text.strip_prefix("selector ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Selector)?);
                } else if let Some(source) = item.text.strip_prefix("event-filter ") {
                    let function = parse_extern_fn(source, item, &path, ExternKind::EventFilter)?;
                    if !function.params.is_empty() {
                        return Err(error(
                            "E022",
                            item,
                            "event filters receive the iced runtime event implicitly and declare no parameters",
                        ));
                    }
                    functions.push(function);
                } else if let Some(source) = item.text.strip_prefix("sync ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Sync)?);
                } else if let Some(source) = item.text.strip_prefix("subscription ") {
                    functions.push(parse_extern_fn(
                        source,
                        item,
                        &path,
                        ExternKind::Subscription,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("theme ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::Theme,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("themer ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Themer)?);
                } else if let Some(source) = item.text.strip_prefix("window ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Window)?);
                } else if let Some(source) = item.text.strip_prefix("markdown-viewer ") {
                    functions.push(parse_extern_fn(
                        source,
                        item,
                        &path,
                        ExternKind::MarkdownViewer,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("editor-binding ") {
                    functions.push(parse_extern_fn(
                        source,
                        item,
                        &path,
                        ExternKind::EditorBinding,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("editor-highlighter ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::EditorHighlighter,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("editor-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::EditorStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("text-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::TextStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("slider-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::SliderStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("progress-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ProgressStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("button-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ButtonStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("checkbox-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::CheckboxStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("toggler-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::TogglerStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("radio-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::RadioStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("container-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ContainerStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("svg-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::SvgStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("input-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::InputStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("scroll-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ScrollStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("pick-list-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::PickListStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("menu-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::MenuStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("pane-grid-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::PaneGridStyle,
                    )?);
                } else if item.text.chars().next().is_some_and(char::is_uppercase) {
                    structs.push(parse_extern_struct(item, &path)?);
                } else {
                    functions.push(parse_extern_fn(
                        &item.text,
                        item,
                        &path,
                        ExternKind::Future,
                    )?);
                }
            }
        } else if line.text == "theme" {
            for item in &line.children {
                ensure_leaf(item)?;
                let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
                    return Err(error("E010", item, "expected `name #RRGGBB`"));
                };
                let name = identifier(name, item)?;
                let value = value.trim();
                if !valid_color(value) {
                    return Err(error("E011", item, "theme colors use #RRGGBB or #RRGGBBAA"));
                }
                if theme.insert(name.clone(), value.into()).is_some() {
                    return Err(error(
                        "E012",
                        item,
                        format!("duplicate theme token `{name}`"),
                    ));
                }
            }
        } else if line.text == "state" {
            states.extend(
                line.children
                    .iter()
                    .map(parse_state)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else if let Some(source) = line.text.strip_prefix("font ") {
            fonts.push(parse_font(source, line)?);
        } else if line.text == "qr" || line.text.starts_with("qr ") {
            qr_codes.push(parse_qr_data(line.text[2..].trim(), line)?);
        } else if let Some(header) = line.text.strip_prefix("component ") {
            components.push(parse_component(header, line)?);
        } else if let Some(header) = line.text.strip_prefix("on ") {
            handlers.push(parse_handler(header, line)?);
        } else if line.text == "subscribe" {
            subscriptions.extend(
                line.children
                    .iter()
                    .map(parse_subscription)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else if line.text == "view" {
            if view.is_some() {
                return Err(error("E004", line, "an app may only have one view"));
            }
            if line.children.len() != 1 {
                return Err(error(
                    "E005",
                    line,
                    "view must contain exactly one root node",
                ));
            }
            view = Some(parse_view(&line.children[0])?);
        } else {
            return Err(error(
                "E001",
                line,
                format!("unknown declaration `{}`", line.text),
            ));
        }
    }

    let span = Span::line(1);
    Ok(Document {
        app: app.ok_or_else(|| {
            Error::new(
                "E006",
                &span,
                "missing `app Name` or `daemon Name` declaration",
            )
        })?,
        daemon,
        settings,
        presets,
        extern_path,
        structs,
        functions,
        subscriptions,
        theme,
        fonts,
        qr_codes,
        states,
        components,
        handlers,
        view: view.ok_or_else(|| Error::new("E008", &span, "missing `view` block"))?,
    })
}

mod canvas;
mod controls;
mod declarations;
mod expression;
mod settings;
mod statement;
mod syntax;
mod view;

use canvas::*;
use controls::*;
use declarations::*;
use expression::*;
use settings::*;
use statement::*;
use syntax::*;
use view::*;

#[cfg(test)]
#[path = "parser/tests.rs"]
mod tests;
