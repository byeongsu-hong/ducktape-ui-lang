use crate::Error;
use crate::ast::*;
use std::collections::BTreeMap;

mod canvas;
mod controls;
mod statement;
mod view;

use canvas::*;
use controls::*;
use statement::*;
use view::*;

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
        if let Some(name) = line.text.strip_prefix("app ") {
            if app.replace(identifier(name.trim(), line)?).is_some() {
                return Err(error("E002", line, "an app may only be declared once"));
            }
            settings = parse_app_settings(line)?;
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
        app: app.ok_or_else(|| Error::new("E006", &span, "missing `app Name` declaration"))?,
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

fn parse_preset(name: &str, line: &Line) -> Result<Preset, Error> {
    let name = identifier(name, line)?;
    let mut statements = Vec::new();
    let mut state = false;
    let mut boot = false;
    for section in &line.children {
        match section.text.as_str() {
            "state" if !state && !boot => {
                state = true;
                for item in &section.children {
                    match parse_statement(item)? {
                        statement @ Statement::Assign { .. } => statements.push(statement),
                        _ => {
                            return Err(error(
                                "E016",
                                item,
                                "preset state only accepts `name = value` overrides",
                            ));
                        }
                    }
                }
            }
            "boot" if !boot => {
                boot = true;
                statements.extend(
                    section
                        .children
                        .iter()
                        .map(parse_statement)
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            "state" if boot => {
                return Err(error("E016", section, "preset state must precede boot"));
            }
            "state" | "boot" => {
                return Err(error(
                    "E016",
                    section,
                    format!("duplicate preset section `{}`", section.text),
                ));
            }
            _ => {
                return Err(error(
                    "E016",
                    section,
                    "preset accepts `state` and `boot` sections",
                ));
            }
        }
    }
    Ok(Preset {
        name,
        statements,
        span: Span::line(line.number),
    })
}

fn parse_app_settings(line: &Line) -> Result<AppSettings, Error> {
    let mut settings = AppSettings::default();
    for item in &line.children {
        if item.text == "window" {
            if settings.window.is_some() {
                return Err(error("E014", item, "duplicate app setting `window`"));
            }
            settings.window = Some(parse_window_settings(item)?);
            continue;
        }
        if let Some(name) = item.text.strip_prefix("window ") {
            let name = identifier(name.trim(), item)?;
            if settings.windows.iter().any(|window| window.name == name) {
                return Err(error(
                    "E014",
                    item,
                    format!("duplicate app window `{name}`"),
                ));
            }
            settings.windows.push(NamedWindow {
                name,
                settings: parse_window_settings(item)?,
            });
            continue;
        }
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error(
                "E014",
                item,
                format!("unknown app setting `{}`", item.text),
            ));
        };
        let value = value.trim();
        macro_rules! set {
            ($field:ident, $value:expr) => {
                set_setting(&mut settings.$field, $value, name, item)?
            };
        }
        match name {
            "title" => set!(title, app_expression(value, item)?),
            "theme" => set!(theme, app_expression(value, item)?),
            "background" => set!(background, app_expression(value, item)?),
            "text-color" => set!(text_color, app_expression(value, item)?),
            "id" => set!(id, string_literal(value, item)?),
            "executor" => set!(executor, rust_path(value, item)?),
            "font" => {
                let path = string_literal(value, item)?;
                if path.is_empty()
                    || path.contains('\\')
                    || std::path::Path::new(&path).is_absolute()
                {
                    return Err(error(
                        "E015",
                        item,
                        "font paths must be non-empty relative `/` paths",
                    ));
                }
                if settings.fonts.iter().any(|font| font.path == path) {
                    return Err(error("E014", item, format!("duplicate app font `{path}`")));
                }
                settings.fonts.push(FontAsset {
                    path,
                    span: Span::line(item.number),
                });
            }
            "default-text-size" => set!(default_text_size, config_positive_number(value, item)?),
            "antialiasing" => set!(antialiasing, config_bool(value, item)?),
            "vsync" => set!(vsync, config_bool(value, item)?),
            "scale-factor" => set!(scale_factor, app_number_expression(value, item)?),
            _ => {
                return Err(error("E014", item, format!("unknown app setting `{name}`")));
            }
        }
    }
    Ok(settings)
}

fn app_expression(source: &str, line: &Line) -> Result<AppExpression, Error> {
    Ok(AppExpression {
        value: parse_expr(source, line)?,
        span: Span::line(line.number),
    })
}

fn app_number_expression(source: &str, line: &Line) -> Result<AppExpression, Error> {
    let mut expression = app_expression(source, line)?;
    if let Expr::I64(value) = &expression.value {
        expression.value = Expr::F64(*value as f64);
    }
    Ok(expression)
}

fn parse_window_settings(line: &Line) -> Result<WindowSettings, Error> {
    let mut settings = WindowSettings::default();
    for item in &line.children {
        if let Some(platform) = item.text.strip_prefix("platform ") {
            match platform.trim() {
                "linux" => set_setting(
                    &mut settings.linux,
                    parse_linux_window_settings(item)?,
                    "platform linux",
                    item,
                )?,
                "windows" => set_setting(
                    &mut settings.windows,
                    parse_windows_window_settings(item)?,
                    "platform windows",
                    item,
                )?,
                "macos" => set_setting(
                    &mut settings.macos,
                    parse_macos_window_settings(item)?,
                    "platform macos",
                    item,
                )?,
                "wasm" => set_setting(
                    &mut settings.wasm,
                    parse_wasm_window_settings(item)?,
                    "platform wasm",
                    item,
                )?,
                _ => {
                    return Err(error(
                        "E015",
                        item,
                        "window platform must be linux, windows, macos, or wasm",
                    ));
                }
            }
            continue;
        }
        ensure_leaf(item)?;
        let (name, value) = item
            .text
            .split_once(char::is_whitespace)
            .map_or((item.text.as_str(), ""), |(name, value)| {
                (name, value.trim())
            });
        macro_rules! set {
            ($field:ident, $value:expr) => {
                set_setting(&mut settings.$field, $value, name, item)?
            };
        }
        match name {
            "size" => set!(size, config_size(value, item)?),
            "maximized" => set!(maximized, config_bool(value, item)?),
            "fullscreen" => set!(fullscreen, config_bool(value, item)?),
            "position" => set!(position, config_position(value, item)?),
            "min-size" => set!(min_size, config_size(value, item)?),
            "max-size" => set!(max_size, config_size(value, item)?),
            "visible" => set!(visible, config_bool(value, item)?),
            "resizable" => set!(resizable, config_bool(value, item)?),
            "closeable" => set!(closeable, config_bool(value, item)?),
            "minimizable" => set!(minimizable, config_bool(value, item)?),
            "decorations" => set!(decorations, config_bool(value, item)?),
            "transparent" => set!(transparent, config_bool(value, item)?),
            "blur" => set!(blur, config_bool(value, item)?),
            "level" => set!(
                level,
                match value {
                    "normal" => WindowLevel::Normal,
                    "always-on-bottom" => WindowLevel::AlwaysOnBottom,
                    "always-on-top" => WindowLevel::AlwaysOnTop,
                    _ => {
                        return Err(error(
                            "E015",
                            item,
                            "window level must be normal, always-on-bottom, or always-on-top",
                        ));
                    }
                }
            ),
            "icon-rgba" => set!(icon, config_window_icon(value, item)?),
            "exit-on-close-request" => {
                set!(exit_on_close_request, config_bool(value, item)?)
            }
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown window setting `{name}`"),
                ));
            }
        }
    }
    if let (Some((min_width, min_height)), Some((max_width, max_height))) =
        (settings.min_size, settings.max_size)
        && (min_width > max_width || min_height > max_height)
    {
        return Err(error(
            "E015",
            line,
            "window min-size cannot exceed max-size",
        ));
    }
    Ok(settings)
}

fn parse_linux_window_settings(line: &Line) -> Result<LinuxWindowSettings, Error> {
    let mut settings = LinuxWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error("E015", item, "Linux window setting requires a value"));
        };
        let value = value.trim();
        match name {
            "application-id" => set_setting(
                &mut settings.application_id,
                string_literal(value, item)?,
                name,
                item,
            )?,
            "override-redirect" => set_setting(
                &mut settings.override_redirect,
                config_bool(value, item)?,
                name,
                item,
            )?,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown Linux window setting `{name}`"),
                ));
            }
        }
    }
    Ok(settings)
}

fn parse_windows_window_settings(line: &Line) -> Result<WindowsWindowSettings, Error> {
    let mut settings = WindowsWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error(
                "E015",
                item,
                "Windows window setting requires a value",
            ));
        };
        let value = value.trim();
        match name {
            "drag-and-drop" => set_setting(
                &mut settings.drag_and_drop,
                config_bool(value, item)?,
                name,
                item,
            )?,
            "skip-taskbar" => set_setting(
                &mut settings.skip_taskbar,
                config_bool(value, item)?,
                name,
                item,
            )?,
            "undecorated-shadow" => set_setting(
                &mut settings.undecorated_shadow,
                config_bool(value, item)?,
                name,
                item,
            )?,
            "corner" => set_setting(
                &mut settings.corner,
                match value {
                    "default" => WindowCorner::Default,
                    "do-not-round" => WindowCorner::DoNotRound,
                    "round" => WindowCorner::Round,
                    "round-small" => WindowCorner::RoundSmall,
                    _ => {
                        return Err(error(
                            "E015",
                            item,
                            "Windows window corner must be default, do-not-round, round, or round-small",
                        ));
                    }
                },
                name,
                item,
            )?,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown Windows window setting `{name}`"),
                ));
            }
        }
    }
    Ok(settings)
}

fn parse_macos_window_settings(line: &Line) -> Result<MacosWindowSettings, Error> {
    let mut settings = MacosWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error("E015", item, "macOS window setting requires a value"));
        };
        let value = value.trim();
        let slot = match name {
            "title-hidden" => &mut settings.title_hidden,
            "titlebar-transparent" => &mut settings.titlebar_transparent,
            "fullsize-content-view" => &mut settings.fullsize_content_view,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown macOS window setting `{name}`"),
                ));
            }
        };
        set_setting(slot, config_bool(value, item)?, name, item)?;
    }
    Ok(settings)
}

fn parse_wasm_window_settings(line: &Line) -> Result<WasmWindowSettings, Error> {
    let mut settings = WasmWindowSettings::default();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((name, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error("E015", item, "Wasm window setting requires a value"));
        };
        let value = value.trim();
        match name {
            "target" => set_setting(
                &mut settings.target,
                if value == "none" {
                    None
                } else {
                    Some(string_literal(value, item)?)
                },
                name,
                item,
            )?,
            _ => {
                return Err(error(
                    "E015",
                    item,
                    format!("unknown Wasm window setting `{name}`"),
                ));
            }
        }
    }
    Ok(settings)
}

fn config_window_icon(source: &str, line: &Line) -> Result<WindowIcon, Error> {
    let parts = split_words(source);
    if parts.len() != 3 {
        return Err(error(
            "E015",
            line,
            "window icon-rgba expects `\"relative/path.rgba\" width height`",
        ));
    }
    let path = string_literal(&parts[0], line)?;
    if path.is_empty() || path.contains('\\') || std::path::Path::new(&path).is_absolute() {
        return Err(error(
            "E015",
            line,
            "window icon paths must be non-empty relative `/` paths",
        ));
    }
    let dimension = |value: &str| {
        value
            .parse::<u32>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                error(
                    "E015",
                    line,
                    "window icon dimensions must be positive integers",
                )
            })
    };
    let width = dimension(&parts[1])?;
    let height = dimension(&parts[2])?;
    let byte_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| error("E015", line, "window icon dimensions are too large"))?;
    Ok(WindowIcon {
        path,
        width,
        height,
        byte_len,
        span: Span::line(line.number),
    })
}

fn set_setting<T>(slot: &mut Option<T>, value: T, name: &str, line: &Line) -> Result<(), Error> {
    if slot.replace(value).is_some() {
        Err(error("E014", line, format!("duplicate setting `{name}`")))
    } else {
        Ok(())
    }
}

fn config_bool(source: &str, line: &Line) -> Result<bool, Error> {
    match source {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error("E015", line, "setting expects true or false")),
    }
}

fn config_number(source: &str, line: &Line) -> Result<f64, Error> {
    source
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| error("E015", line, "setting expects a finite number"))
}

fn config_positive_number(source: &str, line: &Line) -> Result<f64, Error> {
    let value = config_number(source, line)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(error("E015", line, "setting must be greater than zero"))
    }
}

fn config_pair(source: &str, line: &Line) -> Result<(f64, f64), Error> {
    let parts = split_words(source);
    if parts.len() != 2 {
        return Err(error("E015", line, "window size expects `width height`"));
    }
    Ok((
        config_number(&parts[0], line)?,
        config_number(&parts[1], line)?,
    ))
}

fn config_size(source: &str, line: &Line) -> Result<(f64, f64), Error> {
    let (width, height) = config_pair(source, line)?;
    if width > 0.0 && height > 0.0 {
        Ok((width, height))
    } else {
        Err(error(
            "E015",
            line,
            "window dimensions must be greater than zero",
        ))
    }
}

fn config_position(source: &str, line: &Line) -> Result<WindowPosition, Error> {
    match source {
        "default" => Ok(WindowPosition::Default),
        "centered" => Ok(WindowPosition::Centered),
        _ => {
            let (x, y) = config_pair(source, line).map_err(|_| {
                error(
                    "E015",
                    line,
                    "window position expects default, centered, or `x y`",
                )
            })?;
            Ok(WindowPosition::Specific(x, y))
        }
    }
}

fn parse_font(source: &str, line: &Line) -> Result<FontDecl, Error> {
    ensure_leaf(line)?;
    let parts = split_words(source);
    let name = identifier(
        parts
            .first()
            .ok_or_else(|| error("E013", line, "font requires a name"))?,
        line,
    )?;
    let mut family = FontFamily::Named(name.clone());
    let mut weight = FontWeight::Normal;
    let mut stretch = FontStretch::Normal;
    let mut style = FontStyle::Normal;
    let mut default = false;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("family=") {
            family = match value {
                "serif" => FontFamily::Serif,
                "sans" => FontFamily::SansSerif,
                "cursive" => FontFamily::Cursive,
                "fantasy" => FontFamily::Fantasy,
                "mono" => FontFamily::Monospace,
                value => FontFamily::Named(string_literal(value, line)?),
            };
        } else if let Some(value) = part.strip_prefix("weight=") {
            weight = match value {
                "thin" => FontWeight::Thin,
                "extra-light" => FontWeight::ExtraLight,
                "light" => FontWeight::Light,
                "normal" => FontWeight::Normal,
                "medium" => FontWeight::Medium,
                "semibold" => FontWeight::Semibold,
                "bold" => FontWeight::Bold,
                "extra-bold" => FontWeight::ExtraBold,
                "black" => FontWeight::Black,
                _ => return Err(error("E013", line, "unknown font weight")),
            };
        } else if let Some(value) = part.strip_prefix("stretch=") {
            stretch = match value {
                "ultra-condensed" => FontStretch::UltraCondensed,
                "extra-condensed" => FontStretch::ExtraCondensed,
                "condensed" => FontStretch::Condensed,
                "semi-condensed" => FontStretch::SemiCondensed,
                "normal" => FontStretch::Normal,
                "semi-expanded" => FontStretch::SemiExpanded,
                "expanded" => FontStretch::Expanded,
                "extra-expanded" => FontStretch::ExtraExpanded,
                "ultra-expanded" => FontStretch::UltraExpanded,
                _ => return Err(error("E013", line, "unknown font stretch")),
            };
        } else if let Some(value) = part.strip_prefix("style=") {
            style = match value {
                "normal" => FontStyle::Normal,
                "italic" => FontStyle::Italic,
                "oblique" => FontStyle::Oblique,
                _ => return Err(error("E013", line, "unknown font style")),
            };
        } else if let Some(value) = part.strip_prefix("default=") {
            default = match value {
                "true" => true,
                "false" => false,
                _ => return Err(error("E013", line, "font default must be true or false")),
            };
        } else {
            return Err(error(
                "E013",
                line,
                format!("unknown font property `{part}`"),
            ));
        }
    }
    Ok(FontDecl {
        name,
        family,
        weight,
        stretch,
        style,
        default,
        span: Span::line(line.number),
    })
}

fn parse_qr_data(source: &str, line: &Line) -> Result<QrData, Error> {
    ensure_leaf(line)?;
    let parts = split_words(source);
    let name = parts
        .first()
        .ok_or_else(|| error("E093", line, "qr declaration needs a name"))?;
    let data = parts
        .get(1)
        .ok_or_else(|| error("E093", line, "qr declaration needs a string"))?;
    let data = if data.starts_with('"') {
        let Expr::Str(data) = parse_expr(data, line)? else {
            return Err(error(
                "E093",
                line,
                "qr data must be a string or bytes(...)",
            ));
        };
        QrPayload::Text(data)
    } else if let Some(data) = data
        .strip_prefix("bytes(")
        .and_then(|data| data.strip_suffix(')'))
    {
        QrPayload::Bytes(parse_hex_bytes(data, line, "E093")?)
    } else {
        return Err(error(
            "E093",
            line,
            "qr data must be a string or bytes(00 ff ...)",
        ));
    };
    let mut correction = None;
    let mut version = None;
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("correction=") {
            correction = Some(match value {
                "low" => QrCorrection::Low,
                "medium" => QrCorrection::Medium,
                "quartile" => QrCorrection::Quartile,
                "high" => QrCorrection::High,
                _ => {
                    return Err(error(
                        "E093",
                        line,
                        "qr correction must be low, medium, quartile, or high",
                    ));
                }
            });
        } else if let Some(value) = part.strip_prefix("version=") {
            let (kind, number) = value
                .split_once('(')
                .and_then(|(kind, number)| number.strip_suffix(')').map(|number| (kind, number)))
                .ok_or_else(|| {
                    error("E093", line, "qr version uses normal(1..40) or micro(1..4)")
                })?;
            let number = number
                .parse::<u8>()
                .map_err(|_| error("E093", line, "qr version uses normal(1..40) or micro(1..4)"))?;
            version = Some(match kind {
                "normal" => QrVersion::Normal(number),
                "micro" => QrVersion::Micro(number),
                _ => {
                    return Err(error(
                        "E093",
                        line,
                        "qr version uses normal(1..40) or micro(1..4)",
                    ));
                }
            });
        } else {
            return Err(error("E093", line, format!("unknown qr property `{part}`")));
        }
    }
    Ok(QrData {
        name: identifier(name, line)?,
        data,
        correction,
        version,
        span: Span::line(line.number),
    })
}

fn line_tree(source: &str) -> Result<Vec<Line>, Error> {
    let mut flat = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        if raw.contains('\t') {
            return Err(Error::new(
                "E009",
                &Span::line(index + 1),
                "tabs are not allowed; use spaces",
            ));
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let indent = raw.len() - raw.trim_start().len();
        flat.push(Line {
            number: index + 1,
            indent,
            text: trimmed.into(),
            children: Vec::new(),
        });
    }
    if flat.is_empty() {
        return Err(Error::new("E000", &Span::line(1), "empty .ice file"));
    }
    if flat[0].indent != 0 {
        return Err(error(
            "E009",
            &flat[0],
            "the first declaration must not be indented",
        ));
    }
    let mut index = 0;
    parse_block(&flat, &mut index, 0)
}

fn parse_block(flat: &[Line], index: &mut usize, indent: usize) -> Result<Vec<Line>, Error> {
    let mut output = Vec::new();
    while *index < flat.len() {
        if flat[*index].indent < indent {
            break;
        }
        if flat[*index].indent > indent {
            return Err(error("E009", &flat[*index], "unexpected indentation"));
        }
        let mut line = flat[*index].clone();
        *index += 1;
        if *index < flat.len() && flat[*index].indent > indent {
            let child_indent = flat[*index].indent;
            line.children = parse_block(flat, index, child_indent)?;
        }
        output.push(line);
    }
    Ok(output)
}

fn parse_extern_struct(line: &Line, namespace: &str) -> Result<ExternStruct, Error> {
    ensure_leaf(line)?;
    let (name, fields) = parse_signature(&line.text, line)?;
    let mut parsed_fields = Vec::new();
    if !fields.trim().is_empty() {
        for field in split_top(&fields, ',') {
            let Some((name, ty)) = field.split_once(':') else {
                return Err(error("E020", line, "struct fields use `name:type`"));
            };
            parsed_fields.push((identifier(name.trim(), line)?, parse_type(ty.trim(), line)?));
        }
    }
    Ok(ExternStruct {
        rust_path: format!("{namespace}::{name}"),
        name,
        fields: parsed_fields,
        span: Span::line(line.number),
    })
}

fn parse_extern_fn(
    source: &str,
    line: &Line,
    namespace: &str,
    kind: ExternKind,
) -> Result<ExternFn, Error> {
    ensure_leaf(line)?;
    let close = matching_paren(source, line)?;
    let name = identifier(source[..source.find('(').unwrap_or(0)].trim(), line)?;
    let params_source = &source[source.find('(').unwrap_or(0) + 1..close];
    let mut params = Vec::new();
    if !params_source.trim().is_empty() {
        for param in split_top(params_source, ',') {
            let Some((name, ty)) = param.split_once(':') else {
                return Err(error("E021", line, "function parameters use `name:type`"));
            };
            params.push((identifier(name.trim(), line)?, parse_type(ty.trim(), line)?));
        }
    }
    let rest = source[close + 1..].trim();
    let (progress, rest) = if kind == ExternKind::Sip {
        let Some(rest) = rest.strip_prefix("progress=") else {
            return Err(error(
                "E022",
                line,
                "extern sips require `progress=ProgressType -> ReturnType`",
            ));
        };
        let Some((progress, rest)) = split_top_marker(rest, "->") else {
            return Err(error(
                "E022",
                line,
                "extern sips require `progress=ProgressType -> ReturnType`",
            ));
        };
        (Some(parse_type(progress.trim(), line)?), rest)
    } else {
        let Some(rest) = rest.strip_prefix("->") else {
            return Err(error(
                "E022",
                line,
                "extern functions require `-> ReturnType`",
            ));
        };
        (None, rest)
    };
    let (output, error_ty) = match split_top_once(rest.trim(), '!') {
        Some((output, error_ty)) => (
            parse_type(output.trim(), line)?,
            Some(parse_type(error_ty.trim(), line)?),
        ),
        None => (parse_type(rest.trim(), line)?, None),
    };
    if error_ty.is_some()
        && matches!(
            kind,
            ExternKind::Component
                | ExternKind::Shader
                | ExternKind::Recipe
                | ExternKind::Selector
                | ExternKind::EventFilter
                | ExternKind::Sync
                | ExternKind::Subscription
                | ExternKind::Window
                | ExternKind::MarkdownViewer
                | ExternKind::EditorBinding
                | ExternKind::EditorHighlighter
                | ExternKind::EditorStyle
                | ExternKind::TextStyle
                | ExternKind::SliderStyle
                | ExternKind::ProgressStyle
                | ExternKind::ButtonStyle
                | ExternKind::CheckboxStyle
                | ExternKind::TogglerStyle
                | ExternKind::RadioStyle
                | ExternKind::ContainerStyle
                | ExternKind::SvgStyle
                | ExternKind::InputStyle
                | ExternKind::ScrollStyle
                | ExternKind::PickListStyle
                | ExternKind::MenuStyle
        )
    {
        return Err(error(
            "E023",
            line,
            "extern components, shaders, recipes, event filters, sync functions, subscriptions, window callbacks, markdown viewers, editor bindings/highlighters, and widget styles cannot declare an error type",
        ));
    }
    Ok(ExternFn {
        kind,
        rust_path: format!("{namespace}::{name}"),
        name,
        params,
        progress,
        output,
        error: error_ty,
        span: Span::line(line.number),
    })
}

fn parse_subscription(line: &Line) -> Result<Subscription, Error> {
    ensure_leaf(line)?;
    let Some((call, route)) = split_top_marker(&line.text, "->") else {
        return Err(error(
            "E084",
            line,
            "subscription uses `name(args)`, `every duration`, `repeat name() every duration`, `run name(args)`, `recipe name(args)`, `events id using=filter`, `event [raw] [with-id]`, `input-method event`, `keyboard event`, `mouse event`, `touch event`, `window event`, or `system theme` before `-> handler _`",
        ));
    };
    let call = call.trim();
    let (call, condition) = split_top_marker(call, " when ")
        .map_or((call, None), |(call, condition)| {
            (call.trim(), Some(condition.trim()))
        });
    let (call, status) =
        split_top_marker(call, " status=").map_or(Ok((call, None)), |(call, status)| {
            let status = match status.trim() {
                "any" => EventStatus::Any,
                "captured" => EventStatus::Captured,
                "ignored" => EventStatus::Ignored,
                _ => {
                    return Err(error(
                        "E084",
                        line,
                        "subscription status must be any, captured, or ignored",
                    ));
                }
            };
            Ok((call.trim(), Some(status)))
        })?;
    let (call, filter) = split_top_marker(call, " filter=")
        .map_or(Ok((call, None)), |(call, filter)| {
            Ok((call.trim(), Some(identifier(filter.trim(), line)?)))
        })?;
    let (call, context) = split_top_marker(call, " with=")
        .map_or((call, None), |(call, context)| {
            (call.trim(), Some(context.trim()))
        });
    let mut window_id = false;
    let source = if call == "system theme" {
        SubscriptionSource::SystemTheme
    } else if let Some(source) = call.strip_prefix("repeat ") {
        let Some((call, duration)) = split_top_marker(source, " every ") else {
            return Err(error(
                "E084",
                line,
                "repeat uses `repeat name() every duration`",
            ));
        };
        let (function, args) = parse_signature(call.trim(), line)?;
        if !args.trim().is_empty() {
            return Err(error(
                "E084",
                line,
                "repeated async functions cannot take arguments",
            ));
        }
        SubscriptionSource::Repeat {
            function,
            milliseconds: parse_duration(duration.trim(), line)?,
        }
    } else if let Some(duration) = call.strip_prefix("every ") {
        SubscriptionSource::Every {
            milliseconds: parse_duration(duration.trim(), line)?,
        }
    } else if let Some(call) = call.strip_prefix("run ") {
        let (function, args) = parse_signature(call.trim(), line)?;
        SubscriptionSource::Run {
            function,
            args: parse_expr_list(&args, line)?,
        }
    } else if let Some(call) = call.strip_prefix("recipe ") {
        let (function, args) = parse_signature(call.trim(), line)?;
        SubscriptionSource::Recipe {
            function,
            args: parse_expr_list(&args, line)?,
        }
    } else if let Some(source) = call.strip_prefix("events ") {
        let Some((id, filter)) = split_top_marker(source, " using=") else {
            return Err(error(
                "E084",
                line,
                "raw events use `events identity using=event_filter`",
            ));
        };
        SubscriptionSource::Events {
            id: parse_expr(id.trim(), line)?,
            filter: identifier(filter.trim(), line)?,
        }
    } else if matches!(
        call,
        "event" | "event with-id" | "event raw" | "event raw with-id"
    ) {
        window_id = call.ends_with("with-id");
        SubscriptionSource::Event {
            raw: call.starts_with("event raw"),
        }
    } else if call.starts_with("event ") {
        return Err(error(
            "E084",
            line,
            "generic event source uses `event [raw] [with-id]`",
        ));
    } else if let Some(event) = call.strip_prefix("input-method ") {
        SubscriptionSource::InputMethod(match event.trim() {
            "opened" => InputMethodEvent::Opened,
            "preedit" => InputMethodEvent::Preedit,
            "commit" => InputMethodEvent::Commit,
            "closed" => InputMethodEvent::Closed,
            _ => {
                return Err(error(
                    "E084",
                    line,
                    "input-method event must be opened, preedit, commit, or closed",
                ));
            }
        })
    } else if let Some(event) = call.strip_prefix("window ") {
        let event = event.trim();
        let event = event.strip_suffix(" with-id").map_or(event, |event| {
            window_id = true;
            event.trim()
        });
        SubscriptionSource::Window(match event {
            "frame" => WindowEvent::Frame,
            "opened" => WindowEvent::Opened,
            "closed" => WindowEvent::Closed,
            "moved" => WindowEvent::Moved,
            "resized" => WindowEvent::Resized,
            "rescaled" => WindowEvent::Rescaled,
            "close-request" => WindowEvent::CloseRequested,
            "focused" => WindowEvent::Focused,
            "unfocused" => WindowEvent::Unfocused,
            "file-hovered" => WindowEvent::FileHovered,
            "file-dropped" => WindowEvent::FileDropped,
            "files-hovered-left" => WindowEvent::FilesHoveredLeft,
            _ => return Err(error("E084", line, "unknown window event")),
        })
    } else if let Some(event) = call.strip_prefix("keyboard ") {
        SubscriptionSource::Keyboard(match event.trim() {
            "press" => KeyboardEvent::Press,
            "release" => KeyboardEvent::Release,
            "modifiers" => KeyboardEvent::Modifiers,
            _ => {
                return Err(error(
                    "E084",
                    line,
                    "keyboard event must be press, release, or modifiers",
                ));
            }
        })
    } else if let Some(event) = call.strip_prefix("mouse ") {
        SubscriptionSource::Mouse(match event.trim() {
            "entered" => MouseEvent::Entered,
            "left" => MouseEvent::Left,
            "moved" => MouseEvent::Moved,
            "pressed" => MouseEvent::Pressed,
            "released" => MouseEvent::Released,
            "wheel" => MouseEvent::Wheel,
            _ => {
                return Err(error(
                    "E084",
                    line,
                    "mouse event must be entered, left, moved, pressed, released, or wheel",
                ));
            }
        })
    } else if let Some(event) = call.strip_prefix("touch ") {
        SubscriptionSource::Touch(match event.trim() {
            "pressed" => TouchEvent::Pressed,
            "moved" => TouchEvent::Moved,
            "lifted" => TouchEvent::Lifted,
            "lost" => TouchEvent::Lost,
            _ => {
                return Err(error(
                    "E084",
                    line,
                    "touch event must be pressed, moved, lifted, or lost",
                ));
            }
        })
    } else {
        let (function, args) = parse_signature(call, line)?;
        SubscriptionSource::Extern {
            function,
            args: parse_expr_list(&args, line)?,
        }
    };
    if window_id && matches!(&source, SubscriptionSource::Window(WindowEvent::Frame)) {
        return Err(error(
            "E084",
            line,
            "window frame does not expose a window ID",
        ));
    }
    if status.is_some()
        && !matches!(
            &source,
            SubscriptionSource::Event { .. }
                | SubscriptionSource::InputMethod(_)
                | SubscriptionSource::Keyboard(_)
                | SubscriptionSource::Mouse(_)
                | SubscriptionSource::Touch(_)
                | SubscriptionSource::Window(
                    WindowEvent::Opened
                        | WindowEvent::Closed
                        | WindowEvent::Moved
                        | WindowEvent::Resized
                        | WindowEvent::Rescaled
                        | WindowEvent::CloseRequested
                        | WindowEvent::Focused
                        | WindowEvent::Unfocused
                        | WindowEvent::FileHovered
                        | WindowEvent::FileDropped
                        | WindowEvent::FilesHoveredLeft
                )
        )
    {
        return Err(error(
            "E084",
            line,
            "status filtering is only available on non-frame runtime events",
        ));
    }
    Ok(Subscription {
        source,
        window_id,
        context: context
            .map(|context| parse_expr(context, line))
            .transpose()?,
        filter,
        condition: condition
            .map(|condition| parse_expr(condition, line))
            .transpose()?,
        status,
        route: parse_route(route.trim(), line)?,
        span: Span::line(line.number),
    })
}

fn parse_duration(source: &str, line: &Line) -> Result<u64, Error> {
    let (number, multiplier) = source
        .strip_suffix("ms")
        .map(|number| (number, 1))
        .or_else(|| source.strip_suffix('s').map(|number| (number, 1_000)))
        .ok_or_else(|| {
            error(
                "E084",
                line,
                "duration must use `ms` or `s`, like `500ms` or `2s`",
            )
        })?;
    let value = number
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(multiplier))
        .filter(|value| *value > 0)
        .ok_or_else(|| error("E084", line, "duration must be a positive whole number"))?;
    Ok(value)
}

fn parse_state(line: &Line) -> Result<State, Error> {
    ensure_leaf(line)?;
    let Some((left, right)) = split_top_once(&line.text, '=') else {
        return Err(error(
            "E030",
            line,
            "state entries use `name[:type] = value`",
        ));
    };
    let (name, declared) = match left.split_once(':') {
        Some((name, ty)) => (
            identifier(name.trim(), line)?,
            Some(parse_type(ty.trim(), line)?),
        ),
        None => (identifier(left.trim(), line)?, None),
    };
    let initial = parse_expr(right.trim(), line)?;
    let inferred = literal_type(&initial);
    let ty = declared.or(inferred).ok_or_else(|| {
        error("E031", line, "state type cannot be inferred")
            .hint("write an explicit type, for example `items:[Item] = []`")
    })?;
    Ok(State {
        name,
        ty,
        initial,
        span: Span::line(line.number),
    })
}

fn parse_component(header: &str, line: &Line) -> Result<Component, Error> {
    if line.children.len() != 1 {
        return Err(error(
            "E040",
            line,
            "component must have exactly one root node",
        ));
    }
    let (name, params_source) = parse_component_signature(header, line)?;
    let mut params = Vec::new();
    if !params_source.trim().is_empty() {
        for param in split_top(&params_source, ',') {
            let Some((name, ty)) = param.split_once(':') else {
                return Err(error(
                    "E043",
                    line,
                    "component parameters require `name:type`",
                ));
            };
            params.push((identifier(name.trim(), line)?, parse_type(ty.trim(), line)?));
        }
    }
    Ok(Component {
        name,
        params,
        root: parse_view(&line.children[0])?,
        span: Span::line(line.number),
    })
}

fn parse_handler(header: &str, line: &Line) -> Result<Handler, Error> {
    let header = header.trim();
    let (name, params) = if header.contains('(') {
        let (name, params) = parse_signature(header, line)?;
        let params = split_top(&params, ',')
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                Ok(HandlerParam {
                    name: identifier(value.trim(), line)?,
                    ty: Type::Unknown,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        (name, params)
    } else {
        (identifier(header, line)?, Vec::new())
    };
    let statements = line
        .children
        .iter()
        .map(parse_statement)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Handler {
        name,
        params,
        statements,
        span: Span::line(line.number),
    })
}

fn parse_route(source: &str, line: &Line) -> Result<Route, Error> {
    let source = source.trim();
    if let Some(open) = source.find('(') {
        let close = matching_paren(source, line)?;
        let handler = identifier(source[..open].trim(), line)?;
        let args = split_top(&source[open + 1..close], ',')
            .into_iter()
            .filter(|part| !part.trim().is_empty())
            .map(|part| {
                if part.trim() == "_" {
                    Ok(RouteArg::Payload)
                } else {
                    Ok(RouteArg::Expr(parse_expr(part.trim(), line)?))
                }
            })
            .collect::<Result<_, Error>>()?;
        return Ok(Route {
            handler,
            args,
            span: Span::line(line.number),
        });
    }
    let mut words = source.split_whitespace();
    let handler = identifier(
        words
            .next()
            .ok_or_else(|| error("E052", line, "empty route"))?,
        line,
    )?;
    let args = words
        .map(|word| {
            if word == "_" {
                Ok(RouteArg::Payload)
            } else {
                Ok(RouteArg::Expr(parse_expr(word, line)?))
            }
        })
        .collect::<Result<_, Error>>()?;
    Ok(Route {
        handler,
        args,
        span: Span::line(line.number),
    })
}

fn parse_id(source: &str, line: &Line) -> Result<Id, Error> {
    let source = source.strip_prefix('#').unwrap_or(source);
    if let Some(open) = source.find('(') {
        let close = matching_paren(source, line)?;
        if close + 1 != source.len() {
            return Err(error("E068", line, "unexpected text after dynamic id"));
        }
        Ok(Id {
            name: kebab_identifier(&source[..open], line)?,
            key: Some(parse_expr(&source[open + 1..close], line)?),
        })
    } else {
        Ok(Id {
            name: kebab_identifier(source, line)?,
            key: None,
        })
    }
}

fn parse_type(source: &str, line: &Line) -> Result<Type, Error> {
    let source = source.trim();
    if let Some(inner) = source.strip_suffix('?') {
        return Ok(Type::Option(Box::new(parse_type(inner, line)?)));
    }
    if let Some(inner) = source
        .strip_prefix("result[")
        .and_then(|source| source.strip_suffix(']'))
    {
        let parts = split_top(inner, ',');
        if parts.len() != 2 {
            return Err(error(
                "E023",
                line,
                "result type uses `result[Output,Error]`",
            ));
        }
        return Ok(Type::Result(
            Box::new(parse_type(parts[0].trim(), line)?),
            Box::new(parse_type(parts[1].trim(), line)?),
        ));
    }
    if let Some(inner) = source
        .strip_prefix("combo[")
        .and_then(|source| source.strip_suffix(']'))
    {
        return Ok(Type::Combo(Box::new(parse_type(inner, line)?)));
    }
    if source.starts_with('[') && source.ends_with(']') {
        return Ok(Type::List(Box::new(parse_type(
            &source[1..source.len() - 1],
            line,
        )?)));
    }
    Ok(match source {
        "bool" => Type::Bool,
        "i64" => Type::I64,
        "f64" => Type::F64,
        "str" => Type::Str,
        "bytes" => Type::Bytes,
        "image" => Type::Image,
        "markdown" => Type::Markdown,
        "editor" => Type::Editor,
        "event" => Type::Event,
        "key" => Type::Key,
        "physical-key" => Type::PhysicalKey,
        "key-location" => Type::KeyLocation,
        "key-modifiers" => Type::KeyModifiers,
        "pixels" => Type::Pixels,
        "padding" => Type::Padding,
        "degrees" => Type::Degrees,
        "radians" => Type::Radians,
        "point" => Type::Point,
        "point-u32" => Type::PointU32,
        "vector" => Type::Vector,
        "size" => Type::Size,
        "rectangle" => Type::Rectangle,
        "rectangle-u32" => Type::RectangleU32,
        "transformation" => Type::Transformation,
        "mouse-button" => Type::MouseButton,
        "mouse-cursor" => Type::MouseCursor,
        "mouse-click" => Type::MouseClick,
        "touch-finger" => Type::TouchFinger,
        "instant" => Type::Instant,
        "window-id" => Type::WindowId,
        "widget-id" => Type::WidgetId,
        "widget-target" => Type::WidgetTarget,
        "task-handle" => Type::TaskHandle,
        "unit" => Type::Unit,
        value if value.chars().next().is_some_and(char::is_uppercase) => {
            Type::Named(identifier(value, line)?)
        }
        _ => return Err(error("E023", line, format!("unknown type `{source}`"))),
    })
}

fn parse_expr(source: &str, line: &Line) -> Result<Expr, Error> {
    ExprParser::new(source, line)?.parse()
}

fn parse_hex_bytes(source: &str, line: &Line, code: &'static str) -> Result<Vec<u8>, Error> {
    source
        .split_whitespace()
        .map(|byte| {
            (byte.len() == 2)
                .then(|| u8::from_str_radix(byte, 16).ok())
                .flatten()
                .ok_or_else(|| error(code, line, "bytes use two hex digits per byte"))
        })
        .collect()
}

fn parse_expr_list(source: &str, line: &Line) -> Result<Vec<Expr>, Error> {
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }
    split_top(source, ',')
        .into_iter()
        .map(|part| parse_expr(part.trim(), line))
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    I64(i64),
    F64(f64),
    Bytes(Vec<u8>),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Not,
    Neg,
    Plus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

struct ExprParser<'a> {
    tokens: Vec<Token>,
    index: usize,
    line: &'a Line,
}

impl<'a> ExprParser<'a> {
    fn new(source: &str, line: &'a Line) -> Result<Self, Error> {
        Ok(Self {
            tokens: lex_expr(source, line)?,
            index: 0,
            line,
        })
    }

    fn parse(mut self) -> Result<Expr, Error> {
        let expr = self.binary(0)?;
        if self.index != self.tokens.len() {
            return Err(error("E070", self.line, "unexpected token in expression"));
        }
        Ok(expr)
    }

    fn binary(&mut self, min_precedence: u8) -> Result<Expr, Error> {
        let mut left = self.unary()?;
        while let Some((op, precedence)) = self.binary_op() {
            if precedence < min_precedence {
                break;
            }
            self.index += 1;
            let right = self.binary(precedence + 1)?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        if self.peek() == Some(&Token::Not) {
            self.index += 1;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                value: Box::new(self.unary()?),
            });
        }
        if self.peek() == Some(&Token::Neg) {
            self.index += 1;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                value: Box::new(self.unary()?),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let token = self
            .next()
            .ok_or_else(|| error("E070", self.line, "expected expression"))?;
        match token {
            Token::Str(value) => Ok(Expr::Str(value)),
            Token::I64(value) => Ok(Expr::I64(value)),
            Token::F64(value) => Ok(Expr::F64(value)),
            Token::Bytes(value) => Ok(Expr::Bytes(value)),
            Token::LBracket => {
                if self.peek() == Some(&Token::RBracket) {
                    self.index += 1;
                    return Ok(Expr::EmptyList);
                }
                let mut values = Vec::new();
                loop {
                    values.push(self.binary(0)?);
                    if self.peek() == Some(&Token::Comma) {
                        self.index += 1;
                    } else {
                        break;
                    }
                }
                if self.next() != Some(Token::RBracket) {
                    return Err(error("E070", self.line, "missing closing `]`"));
                }
                Ok(Expr::List(values))
            }
            Token::LParen => {
                let value = self.binary(0)?;
                if self.next() != Some(Token::RParen) {
                    return Err(error("E070", self.line, "missing closing `)`"));
                }
                Ok(value)
            }
            Token::Ident(name) if name == "true" => Ok(Expr::Bool(true)),
            Token::Ident(name) if name == "false" => Ok(Expr::Bool(false)),
            Token::Ident(name) if name == "none" => Ok(Expr::None),
            Token::Ident(name) => {
                let mut path = vec![name];
                while self.peek() == Some(&Token::Dot) {
                    self.index += 1;
                    match self.next() {
                        Some(Token::Ident(field)) => path.push(field),
                        _ => return Err(error("E070", self.line, "expected name after `.`")),
                    }
                }
                if self.peek() == Some(&Token::LParen) {
                    self.index += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Token::RParen) {
                        loop {
                            args.push(self.binary(0)?);
                            if self.peek() == Some(&Token::Comma) {
                                self.index += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if self.next() != Some(Token::RParen) {
                        return Err(error("E070", self.line, "missing closing `)`"));
                    }
                    return Ok(Expr::Call {
                        name: path.join("."),
                        args,
                    });
                }
                Ok(Expr::Path(path))
            }
            _ => Err(error("E070", self.line, "invalid expression")),
        }
    }

    fn binary_op(&self) -> Option<(BinaryOp, u8)> {
        Some(match self.peek()? {
            Token::Or => (BinaryOp::Or, 1),
            Token::And => (BinaryOp::And, 2),
            Token::EqEq => (BinaryOp::Eq, 3),
            Token::NotEq => (BinaryOp::NotEq, 3),
            Token::Lt => (BinaryOp::Lt, 4),
            Token::LtEq => (BinaryOp::LtEq, 4),
            Token::Gt => (BinaryOp::Gt, 4),
            Token::GtEq => (BinaryOp::GtEq, 4),
            Token::Plus => (BinaryOp::Add, 5),
            Token::Neg => (BinaryOp::Sub, 5),
            Token::Star => (BinaryOp::Mul, 6),
            Token::Slash => (BinaryOp::Div, 6),
            Token::Percent => (BinaryOp::Rem, 6),
            _ => return None,
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<Token> {
        let value = self.tokens.get(self.index).cloned();
        self.index += usize::from(value.is_some());
        value
    }
}

fn lex_expr(source: &str, line: &Line) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '"' {
            index += 1;
            let mut value = String::new();
            while index < chars.len() && chars[index] != '"' {
                if chars[index] == '\\' {
                    index += 1;
                    let escaped = *chars
                        .get(index)
                        .ok_or_else(|| error("E070", line, "unfinished string escape"))?;
                    value.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        _ => {
                            return Err(error(
                                "E070",
                                line,
                                format!("unsupported string escape `\\{escaped}`"),
                            ));
                        }
                    });
                } else {
                    value.push(chars[index]);
                }
                index += 1;
            }
            if chars.get(index) != Some(&'"') {
                return Err(error("E070", line, "unterminated string"));
            }
            index += 1;
            tokens.push(Token::Str(value));
            continue;
        }
        if chars[index..].starts_with(&['b', 'y', 't', 'e', 's', '(']) {
            let start = index + 6;
            let end = chars[start..]
                .iter()
                .position(|ch| *ch == ')')
                .map(|offset| start + offset)
                .ok_or_else(|| error("E070", line, "missing closing `)` after bytes"))?;
            let source = chars[start..end].iter().collect::<String>();
            tokens.push(Token::Bytes(parse_hex_bytes(&source, line, "E070")?));
            index = end + 1;
            continue;
        }
        if ch.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            let value: String = chars[start..index].iter().collect();
            if value.contains('.') {
                tokens.push(Token::F64(
                    value
                        .parse()
                        .map_err(|_| error("E070", line, "invalid float"))?,
                ));
            } else {
                tokens.push(Token::I64(
                    value
                        .parse()
                        .map_err(|_| error("E070", line, "invalid integer"))?,
                ));
            }
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            tokens.push(Token::Ident(chars[start..index].iter().collect()));
            continue;
        }
        let next = chars.get(index + 1).copied();
        let (token, width) = match (ch, next) {
            ('=', Some('=')) => (Token::EqEq, 2),
            ('!', Some('=')) => (Token::NotEq, 2),
            ('<', Some('=')) => (Token::LtEq, 2),
            ('>', Some('=')) => (Token::GtEq, 2),
            ('&', Some('&')) => (Token::And, 2),
            ('|', Some('|')) => (Token::Or, 2),
            ('(', _) => (Token::LParen, 1),
            (')', _) => (Token::RParen, 1),
            ('[', _) => (Token::LBracket, 1),
            (']', _) => (Token::RBracket, 1),
            (',', _) => (Token::Comma, 1),
            ('.', _) => (Token::Dot, 1),
            ('!', _) => (Token::Not, 1),
            ('-', _) => (Token::Neg, 1),
            ('+', _) => (Token::Plus, 1),
            ('*', _) => (Token::Star, 1),
            ('/', _) => (Token::Slash, 1),
            ('%', _) => (Token::Percent, 1),
            ('<', _) => (Token::Lt, 1),
            ('>', _) => (Token::Gt, 1),
            _ => return Err(error("E070", line, format!("unexpected character `{ch}`"))),
        };
        tokens.push(token);
        index += width;
    }
    Ok(tokens)
}

#[cfg(test)]
#[path = "parser/tests.rs"]
mod tests;

fn parse_signature(source: &str, line: &Line) -> Result<(String, String), Error> {
    let (name, args) = signature_parts(source, line)?;
    Ok((identifier(name, line)?, args))
}

fn parse_component_signature(source: &str, line: &Line) -> Result<(String, String), Error> {
    let (name, args) = signature_parts(source, line)?;
    Ok((component_identifier(name, line)?, args))
}

fn signature_parts<'a>(source: &'a str, line: &Line) -> Result<(&'a str, String), Error> {
    let open = source
        .find('(')
        .ok_or_else(|| error("E024", line, "expected `(`"))?;
    let close = matching_paren(source, line)?;
    if !source[close + 1..].trim().is_empty() {
        return Err(error("E024", line, "unexpected text after `)`"));
    }
    Ok((source[..open].trim(), source[open + 1..close].into()))
}

fn matching_paren(source: &str, line: &Line) -> Result<usize, Error> {
    let open = source
        .find('(')
        .ok_or_else(|| error("E024", line, "expected `(`"))?;
    let mut depth = 0;
    let mut string = false;
    for (index, ch) in source.char_indices().skip_while(|(index, _)| *index < open) {
        if ch == '"' {
            string = !string;
        } else if !string {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
        }
    }
    Err(error("E024", line, "missing closing `)`"))
}

fn split_words(source: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut string = false;
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    for (byte, ch) in &chars {
        match *ch {
            '"' => string = !string,
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            ch if ch.is_whitespace() && !string && depth == 0 => {
                if start < *byte {
                    output.push(source[start..*byte].into());
                }
                start = *byte + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start < source.len() {
        output.push(source[start..].into());
    }
    output
}

fn split_top(source: &str, delimiter: char) -> Vec<&str> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut string = false;
    for (index, ch) in source.char_indices() {
        match ch {
            '"' => string = !string,
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            ch if ch == delimiter && !string && depth == 0 => {
                output.push(source[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    output.push(source[start..].trim());
    output
}

fn split_top_once(source: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut depth = 0;
    let mut string = false;
    for (index, ch) in source.char_indices() {
        match ch {
            '"' => string = !string,
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            ch if ch == delimiter && !string && depth == 0 => {
                return Some((&source[..index], &source[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

fn split_top_marker<'a>(source: &'a str, marker: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0;
    let mut string = false;
    let bytes = source.as_bytes();
    let mut index = 0;
    while index + marker.len() <= bytes.len() {
        let ch = source[index..].chars().next()?;
        match ch {
            '"' => string = !string,
            '(' | '[' if !string => depth += 1,
            ')' | ']' if !string => depth -= 1,
            _ => {}
        }
        let part_of_binding = marker == "->" && index > 0 && bytes[index - 1] == b'<';
        if !string && depth == 0 && !part_of_binding && source[index..].starts_with(marker) {
            return Some((&source[..index], &source[index + marker.len()..]));
        }
        index += ch.len_utf8();
    }
    None
}

fn strip_wrapping_parens(source: &str) -> &str {
    let source = source.trim();
    if source.starts_with('(') && source.ends_with(')') {
        &source[1..source.len() - 1]
    } else {
        source
    }
}

fn string_literal(source: &str, line: &Line) -> Result<String, Error> {
    match parse_expr(source, line)? {
        Expr::Str(value) => Ok(value),
        _ => Err(error("E071", line, "expected string literal")),
    }
}

fn literal_type(expr: &Expr) -> Option<Type> {
    Some(match expr {
        Expr::Bool(_) => Type::Bool,
        Expr::I64(_) => Type::I64,
        Expr::F64(_) => Type::F64,
        Expr::Str(_) => Type::Str,
        Expr::Bytes(_) => Type::Bytes,
        Expr::Call { name, .. } if matches!(name.as_str(), "encoded" | "rgba") => Type::Image,
        Expr::EmptyList => return None,
        Expr::List(values) => {
            let first = values.first().and_then(literal_type)?;
            if values
                .iter()
                .skip(1)
                .all(|value| literal_type(value).as_ref() == Some(&first))
            {
                Type::List(Box::new(first))
            } else {
                return None;
            }
        }
        Expr::None => return None,
        _ => return None,
    })
}

fn valid_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

fn identifier(source: &str, line: &Line) -> Result<String, Error> {
    if !source.is_empty()
        && source.chars().enumerate().all(|(index, ch)| {
            ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
        })
    {
        Ok(source.into())
    } else {
        Err(error(
            "E072",
            line,
            format!("invalid identifier `{source}`"),
        ))
    }
}

fn component_identifier(source: &str, line: &Line) -> Result<String, Error> {
    if source.split('.').all(|part| identifier(part, line).is_ok()) {
        Ok(source.into())
    } else {
        Err(error(
            "E072",
            line,
            format!("invalid component name `{source}`"),
        ))
    }
}

fn kebab_identifier(source: &str, line: &Line) -> Result<String, Error> {
    if !source.is_empty()
        && source
            .chars()
            .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
    {
        Ok(source.into())
    } else {
        Err(error("E072", line, format!("invalid id `{source}`")))
    }
}

fn rust_path(source: &str, line: &Line) -> Result<String, Error> {
    if source
        .split("::")
        .all(|part| part == "crate" || identifier(part, line).is_ok())
    {
        Ok(source.into())
    } else {
        Err(error("E073", line, format!("invalid Rust path `{source}`")))
    }
}

fn ensure_leaf(line: &Line) -> Result<(), Error> {
    if line.children.is_empty() {
        Ok(())
    } else {
        Err(error(
            "E009",
            line,
            "this line cannot have an indented block",
        ))
    }
}

fn error(code: &'static str, line: &Line, message: impl Into<String>) -> Error {
    Error::new(code, &Span::line(line.number), message)
}
