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

fn parse_statement(line: &Line) -> Result<Statement, Error> {
    let group = match line.text.as_str() {
        "parallel" => Some(TaskGroupKind::Parallel),
        "sequential" => Some(TaskGroupKind::Sequential),
        _ => None,
    };
    if let Some(kind) = group {
        if line.children.is_empty() {
            return Err(error(
                "E050",
                line,
                "task groups require at least one indented task",
            ));
        }
        return Ok(Statement::TaskGroup {
            kind,
            statements: line
                .children
                .iter()
                .map(parse_statement)
                .collect::<Result<_, _>>()?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("abortable ") {
        let parts = split_words(source);
        if parts.len() > 2
            || parts.is_empty()
            || parts.get(1).is_some_and(|value| value != "abort-on-drop")
        {
            return Err(error(
                "E050",
                line,
                "abortable uses `abortable handle [abort-on-drop]`",
            ));
        }
        if line.children.len() != 1 {
            return Err(error(
                "E050",
                line,
                "abortable requires exactly one indented task",
            ));
        }
        return Ok(Statement::Abortable {
            handle: identifier(&parts[0], line)?,
            abort_on_drop: parts.len() == 2,
            task: Box::new(parse_statement(&line.children[0])?),
            span: Span::line(line.number),
        });
    }
    if line.text == "abortable" {
        return Err(error("E050", line, "abortable requires a handle state"));
    }
    if line.text == "flow" {
        return parse_task_flow(line);
    }
    if let Some(source) = line.text.strip_prefix("sip ") {
        return parse_sip_statement(source, line);
    }
    if line.text == "sip" {
        return Err(error("E050", line, "sip requires an extern call"));
    }
    ensure_leaf(line)?;
    if let Some(source) = line.text.strip_prefix("combo ") {
        let Some((target, value)) = split_top_marker(source, " push ") else {
            return Err(error(
                "E050",
                line,
                "combo mutation uses `combo state push value`",
            ));
        };
        return Ok(Statement::ComboPush {
            target: identifier(target.trim(), line)?,
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("markdown ") {
        let Some((target, value)) = split_top_marker(source, " append ") else {
            return Err(error(
                "E050",
                line,
                "markdown mutation uses `markdown state append text`",
            ));
        };
        return Ok(Statement::MarkdownAppend {
            target: identifier(target.trim(), line)?,
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(condition) = line.text.strip_prefix("return if ") {
        return Ok(Statement::ReturnIf {
            condition: parse_expr(condition, line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(handle) = line.text.strip_prefix("abort ") {
        return Ok(Statement::Abort {
            handle: identifier(handle.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("pane ") {
        return parse_pane_operation(source, line);
    }
    if let Some(source) = line.text.strip_prefix("task widget ") {
        return parse_widget_operation(source, line);
    }
    if let Some(source) = line.text.strip_prefix("task window ") {
        return parse_window_operation(source, line);
    }
    for (prefix, primary) in [
        ("task clipboard write-primary ", true),
        ("task clipboard write ", false),
    ] {
        if let Some(value) = line.text.strip_prefix(prefix) {
            return Ok(Statement::ClipboardWrite {
                primary,
                value: parse_expr(value, line)?,
                span: Span::line(line.number),
            });
        }
    }
    let effect = line
        .text
        .strip_prefix("run ")
        .map(|source| (EffectKind::Future, source))
        .or_else(|| {
            line.text
                .strip_prefix("task ")
                .map(|source| (EffectKind::Task, source))
        })
        .or_else(|| {
            line.text
                .strip_prefix("stream ")
                .map(|source| (EffectKind::Stream, source))
        });
    if let Some((kind, run)) = effect {
        let Some((call, routes)) = split_top_marker(run, "->") else {
            let keyword = match kind {
                EffectKind::Future => "run",
                EffectKind::Task => "task",
                EffectKind::Stream => "stream",
            };
            return Err(error(
                "E050",
                line,
                format!("{keyword} requires `-> success _ | error _`"),
            ));
        };
        let (function, args) = parse_effect_call(kind, call.trim(), line)?;
        let (success, error_route) = match split_top_once(routes.trim(), '|') {
            Some((success, failure)) => (
                parse_route(success.trim(), line)?,
                Some(parse_route(failure.trim(), line)?),
            ),
            None => (parse_route(routes.trim(), line)?, None),
        };
        return Ok(Statement::Run {
            kind,
            function,
            args,
            success,
            error: error_route,
            span: Span::line(line.number),
        });
    }
    if let Some((target, value)) = split_top_once(&line.text, '=') {
        return Ok(Statement::Assign {
            target: identifier(target.trim(), line)?,
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    Err(error(
        "E051",
        line,
        format!("unknown statement `{}`", line.text),
    ))
}

fn parse_task_flow(line: &Line) -> Result<Statement, Error> {
    let Some(first) = line.children.first() else {
        return Err(error(
            "E050",
            line,
            "flow requires an indented `from run|task|stream ...`, `from done value`, or `from none Type` source",
        ));
    };
    ensure_leaf(first)?;
    let source = first.text.strip_prefix("from ").ok_or_else(|| {
        error(
            "E050",
            first,
            "the first flow line must be `from run|task|stream ...`, `from done value`, or `from none Type`",
        )
    })?;
    let source = parse_task_source(source, first)?;
    let mut transforms = Vec::new();
    let mut success = None;
    let mut failure = None;
    let mut units = None;
    for item in &line.children[1..] {
        ensure_leaf(item)?;
        if item.text == "collect" {
            transforms.push(TaskTransform::Collect {
                span: Span::line(item.number),
            });
            continue;
        }
        if item.text == "discard" {
            transforms.push(TaskTransform::Discard {
                span: Span::line(item.number),
            });
            continue;
        }
        if let Some(source) = item.text.strip_prefix("map-error ") {
            let Some((binding, value)) = split_top_marker(source, "->") else {
                return Err(error(
                    "E050",
                    item,
                    "map-error uses `map-error error -> sync_call(error)`",
                ));
            };
            transforms.push(TaskTransform::MapError {
                binding: identifier(binding.trim(), item)?,
                value: parse_expr(value.trim(), item)?,
                span: Span::line(item.number),
            });
            continue;
        }
        let transform = item
            .text
            .strip_prefix("then ")
            .map(|source| (false, source))
            .or_else(|| {
                item.text
                    .strip_prefix("and-then ")
                    .map(|source| (true, source))
            });
        if let Some((and_then, source)) = transform {
            let Some((binding, source)) = split_top_marker(source, "->") else {
                return Err(error(
                    "E050",
                    item,
                    "flow transforms use `then value -> task call(...)` or `and-then value -> task call(...)`",
                ));
            };
            let binding = identifier(binding.trim(), item)?;
            let source = parse_task_source(source.trim(), item)?;
            transforms.push(if and_then {
                TaskTransform::AndThen {
                    binding,
                    source,
                    span: Span::line(item.number),
                }
            } else {
                TaskTransform::Then {
                    binding,
                    source,
                    span: Span::line(item.number),
                }
            });
            continue;
        }
        let Some((kind, route)) = split_top_marker(&item.text, "->") else {
            return Err(error(
                "E050",
                item,
                "flow lines must be then, and-then, map-error, collect, discard, done, error, or units",
            ));
        };
        let slot = match kind.trim() {
            "done" => &mut success,
            "error" => &mut failure,
            "units" => &mut units,
            _ => {
                return Err(error(
                    "E050",
                    item,
                    "flow route must be done, error, or units",
                ));
            }
        };
        if slot.is_some() {
            return Err(error(
                "E050",
                item,
                format!("duplicate flow {} route", kind.trim()),
            ));
        }
        *slot = Some(parse_route(route.trim(), item)?);
    }
    Ok(Statement::TaskFlow {
        source,
        transforms,
        success,
        error: failure,
        units,
        span: Span::line(line.number),
    })
}

fn parse_task_source(source: &str, line: &Line) -> Result<TaskSource, Error> {
    if let Some(value) = source.strip_prefix("done ") {
        return Ok(TaskSource::Done {
            value: parse_expr(value.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    if let Some(output) = source.strip_prefix("none ") {
        return Ok(TaskSource::None {
            output: parse_type(output.trim(), line)?,
            span: Span::line(line.number),
        });
    }
    let (kind, call) = source
        .strip_prefix("run ")
        .map(|call| (EffectKind::Future, call))
        .or_else(|| {
            source
                .strip_prefix("task ")
                .map(|call| (EffectKind::Task, call))
        })
        .or_else(|| {
            source
                .strip_prefix("stream ")
                .map(|call| (EffectKind::Stream, call))
        })
        .ok_or_else(|| {
            error(
                "E050",
                line,
                "task source must start with run, task, or stream",
            )
        })?;
    let (function, args) = parse_effect_call(kind, call.trim(), line)?;
    Ok(TaskSource::Effect {
        kind,
        function,
        args,
        span: Span::line(line.number),
    })
}

fn parse_effect_call(
    kind: EffectKind,
    call: &str,
    line: &Line,
) -> Result<(String, Vec<Expr>), Error> {
    if kind == EffectKind::Task && call == "system info" {
        Ok(("__ice_system_info".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "system theme" {
        Ok(("__ice_system_theme".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "time now" {
        Ok(("__ice_time_now".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "clipboard read" {
        Ok(("__ice_clipboard_read".into(), Vec::new()))
    } else if kind == EffectKind::Task && call == "clipboard read-primary" {
        Ok(("__ice_clipboard_read_primary".into(), Vec::new()))
    } else if kind == EffectKind::Task
        && let Some(value) = call.strip_prefix("font load ")
    {
        if value.trim().is_empty() {
            return Err(error("E050", line, "font load requires bytes"));
        }
        Ok((
            "__ice_font_load".into(),
            vec![parse_expr(value.trim(), line)?],
        ))
    } else if call.starts_with("system ") {
        Err(error(
            "E050",
            line,
            "system task must be `task system info` or `task system theme`",
        ))
    } else if call.starts_with("clipboard ") {
        Err(error(
            "E050",
            line,
            "clipboard task must read, read-primary, write, or write-primary",
        ))
    } else if call.starts_with("font ") {
        Err(error(
            "E050",
            line,
            "font task must be `task font load bytes -> loaded`",
        ))
    } else if call.starts_with("time ") {
        Err(error("E050", line, "time task must be `task time now`"))
    } else {
        let (function, args_source) = parse_signature(call, line)?;
        Ok((function, parse_expr_list(&args_source, line)?))
    }
}

fn parse_sip_statement(source: &str, line: &Line) -> Result<Statement, Error> {
    let (function, args) = parse_signature(source.trim(), line)?;
    let args = parse_expr_list(&args, line)?;
    let mut progress = None;
    let mut success = None;
    let mut failure = None;
    for route in &line.children {
        ensure_leaf(route)?;
        let Some((kind, target)) = split_top_marker(&route.text, "->") else {
            return Err(error(
                "E050",
                route,
                "sip routes use `progress -> handler _`, `done -> handler _`, or `error -> handler _`",
            ));
        };
        let slot = match kind.trim() {
            "progress" => &mut progress,
            "done" => &mut success,
            "error" => &mut failure,
            _ => {
                return Err(error(
                    "E050",
                    route,
                    "sip route must be progress, done, or error",
                ));
            }
        };
        if slot.is_some() {
            return Err(error(
                "E050",
                route,
                format!("duplicate sip {} route", kind.trim()),
            ));
        }
        *slot = Some(parse_route(target.trim(), route)?);
    }
    let progress = progress.ok_or_else(|| {
        error(
            "E050",
            line,
            "sip requires an indented `progress -> handler _` route",
        )
    })?;
    let success = success.ok_or_else(|| {
        error(
            "E050",
            line,
            "sip requires an indented `done -> handler _` route",
        )
    })?;
    Ok(Statement::Sip {
        function,
        args,
        progress,
        success,
        error: failure,
        span: Span::line(line.number),
    })
}

fn parse_pane_operation(source: &str, line: &Line) -> Result<Statement, Error> {
    let (source, route) = split_top_marker(source, "->")
        .map_or((source, None), |(source, route)| (source, Some(route)));
    let parts = split_words(source);
    let grid = parts
        .first()
        .and_then(|part| part.strip_prefix('#'))
        .ok_or_else(|| error("E188", line, "pane operation target must use `#grid`"))?;
    let grid = identifier(grid, line)?;
    let pane = |index: usize| {
        identifier(
            parts
                .get(index)
                .ok_or_else(|| error("E188", line, "pane operation is missing a pane name"))?,
            line,
        )
    };
    let edge = |index: usize| {
        Ok(match parts.get(index).map(String::as_str) {
            Some("top") => PaneEdge::Top,
            Some("left") => PaneEdge::Left,
            Some("right") => PaneEdge::Right,
            Some("bottom") => PaneEdge::Bottom,
            _ => {
                return Err(error(
                    "E188",
                    line,
                    "pane edge must be top, left, right, or bottom",
                ));
            }
        })
    };
    let axis = |index: usize| {
        Ok(match parts.get(index).map(String::as_str) {
            Some("horizontal") => PaneAxis::Horizontal,
            Some("vertical") => PaneAxis::Vertical,
            _ => {
                return Err(error(
                    "E188",
                    line,
                    "pane split axis must be horizontal or vertical",
                ));
            }
        })
    };
    let operation = match parts.get(1).map(String::as_str) {
        Some("maximize") if parts.len() == 3 => PaneOperation::Maximize { pane: pane(2)? },
        Some("restore") if parts.len() == 2 => PaneOperation::Restore,
        Some("maximized") if parts.len() == 2 => PaneOperation::Maximized,
        Some("adjacent") if parts.len() == 4 => PaneOperation::Adjacent {
            pane: pane(2)?,
            edge: edge(3)?,
        },
        Some("swap") if parts.len() == 4 => PaneOperation::Swap {
            first: pane(2)?,
            second: pane(3)?,
        },
        Some("close") if parts.len() == 3 => PaneOperation::Close { pane: pane(2)? },
        Some("move") if parts.len() == 4 => PaneOperation::Move {
            pane: pane(2)?,
            edge: edge(3)?,
        },
        Some("resize") if parts.len() == 3 => PaneOperation::Resize {
            ratio: parse_expr(strip_wrapping_parens(&parts[2]), line)?,
        },
        Some("drop") if parts.len() == 5 => PaneOperation::Drop {
            pane: pane(2)?,
            target: pane(3)?,
            edge: match parts[4].as_str() {
                "center" => None,
                _ => Some(edge(4)?),
            },
        },
        Some("split") if (5..=6).contains(&parts.len()) => PaneOperation::Split {
            target: pane(2)?,
            pane: pane(3)?,
            axis: axis(4)?,
            ratio: parts.get(5).map_or(Ok(Expr::F64(0.5)), |part| {
                let value = part
                    .strip_prefix("ratio=")
                    .ok_or_else(|| error("E188", line, "pane split ratio uses `ratio=value`"))?;
                parse_expr(strip_wrapping_parens(value), line)
            })?,
        },
        _ => {
            return Err(error(
                "E188",
                line,
                "unknown pane operation or wrong arguments",
            ));
        }
    };
    Ok(Statement::PaneOperation {
        grid,
        operation,
        route: route
            .map(|route| parse_route(route.trim(), line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

fn parse_widget_operation(source: &str, line: &Line) -> Result<Statement, Error> {
    let (source, route) = split_top_marker(source, "->")
        .map_or((source, None), |(source, route)| (source, Some(route)));
    let parts = split_words(source);
    let target = |index: usize| {
        let value = parts
            .get(index)
            .ok_or_else(|| error("E052", line, "widget operation is missing `#id`"))?;
        parse_widget_target(value, line)
    };
    let expr = |index: usize| {
        parse_expr(
            strip_wrapping_parens(
                parts
                    .get(index)
                    .ok_or_else(|| error("E052", line, "widget operation is missing a value"))?,
            ),
            line,
        )
    };
    let operation = if let Some(selector) = source.strip_prefix("find-all ") {
        WidgetOperation::Find {
            selector: parse_widget_selector(selector.trim(), line)?,
            all: true,
        }
    } else if let Some(selector) = source.strip_prefix("find ") {
        WidgetOperation::Find {
            selector: parse_widget_selector(selector.trim(), line)?,
            all: false,
        }
    } else {
        match parts.first().map(String::as_str) {
            Some("focus-previous") if parts.len() == 1 => WidgetOperation::FocusPrevious,
            Some("focus-next") if parts.len() == 1 => WidgetOperation::FocusNext,
            Some("focus") if parts.len() == 2 => WidgetOperation::Focus { target: target(1)? },
            Some("focused") if parts.len() == 2 => WidgetOperation::Focused { target: target(1)? },
            Some("cursor-front") if parts.len() == 2 => {
                WidgetOperation::CursorFront { target: target(1)? }
            }
            Some("cursor-end") if parts.len() == 2 => {
                WidgetOperation::CursorEnd { target: target(1)? }
            }
            Some("cursor") if parts.len() == 3 => WidgetOperation::Cursor {
                target: target(1)?,
                position: expr(2)?,
            },
            Some("select-all") if parts.len() == 2 => {
                WidgetOperation::SelectAll { target: target(1)? }
            }
            Some("select") if parts.len() == 4 => WidgetOperation::Select {
                target: target(1)?,
                start: expr(2)?,
                end: expr(3)?,
            },
            Some("snap") if parts.len() == 4 => WidgetOperation::Snap {
                target: target(1)?,
                x: expr(2)?,
                y: expr(3)?,
            },
            Some("snap-end") if parts.len() == 2 => WidgetOperation::SnapEnd { target: target(1)? },
            Some("scroll-to") if parts.len() == 4 => WidgetOperation::ScrollTo {
                target: target(1)?,
                x: expr(2)?,
                y: expr(3)?,
            },
            Some("scroll-by") if parts.len() == 4 => WidgetOperation::ScrollBy {
                target: target(1)?,
                x: expr(2)?,
                y: expr(3)?,
            },
            _ => {
                return Err(error(
                    "E052",
                    line,
                    "unknown widget operation or wrong arguments",
                ));
            }
        }
    };
    Ok(Statement::WidgetOperation {
        operation,
        route: route
            .map(|route| parse_route(route.trim(), line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

fn parse_widget_selector(source: &str, line: &Line) -> Result<WidgetSelector, Error> {
    if let Some(target) = source.strip_prefix("id ") {
        Ok(WidgetSelector::Id(parse_widget_target(
            target.trim(),
            line,
        )?))
    } else if let Some(value) = source.strip_prefix("text ") {
        Ok(WidgetSelector::Text(parse_expr(value.trim(), line)?))
    } else if let Some(values) = source.strip_prefix("point ") {
        let values = split_words(values);
        if values.len() != 2 {
            return Err(error(
                "E052",
                line,
                "point selector requires x and y expressions",
            ));
        }
        Ok(WidgetSelector::Point {
            x: parse_expr(&values[0], line)?,
            y: parse_expr(&values[1], line)?,
        })
    } else if source == "focused" {
        Ok(WidgetSelector::Focused)
    } else {
        let (function, args) = parse_signature(source, line)?;
        Ok(WidgetSelector::Extern {
            function,
            args: parse_expr_list(&args, line)?,
        })
    }
}

fn parse_widget_target(source: &str, line: &Line) -> Result<WidgetTarget, Error> {
    let source = source.strip_prefix('#').ok_or_else(|| {
        error(
            "E052",
            line,
            "widget operation target must use `#id`, `#id(key)`, or `#scope/id`",
        )
    })?;
    let segments = split_top(source, '/')
        .into_iter()
        .map(|segment| {
            let segment = segment.strip_prefix('#').unwrap_or(segment);
            if segment.is_empty() {
                return Err(error("E052", line, "widget target contains an empty scope"));
            }
            if segment.contains('(') {
                parse_id(segment, line)
            } else if kebab_identifier(segment, line).is_ok()
                || component_identifier(segment, line).is_ok()
            {
                Ok(Id {
                    name: segment.into(),
                    key: None,
                })
            } else {
                Err(error(
                    "E052",
                    line,
                    format!("invalid widget target scope `{segment}`"),
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WidgetTarget { segments })
}

fn parse_window_operation(source: &str, line: &Line) -> Result<Statement, Error> {
    let (source, route) = split_top_marker(source, "->")
        .map_or((source, None), |(source, route)| (source, Some(route)));
    let (source, target) = split_top_marker(source, " target=")
        .map_or((source, None), |(source, target)| {
            (source, Some(parse_expr(target.trim(), line)))
        });
    let target = target.transpose()?;
    let parts = split_words(source);
    let expr = |index: usize| {
        parse_expr(
            strip_wrapping_parens(
                parts
                    .get(index)
                    .ok_or_else(|| error("E053", line, "window task is missing a value"))?,
            ),
            line,
        )
    };
    let size = || match parts.as_slice() {
        [_, value] if value == "none" => Ok(None),
        [_, _, _] => Ok(Some((expr(1)?, expr(2)?))),
        _ => Err(error(
            "E053",
            line,
            "window size task expects `width height` or `none`",
        )),
    };
    let operation = match parts.first().map(String::as_str) {
        Some("open") if parts.len() == 1 => WindowOperation::Open(None),
        Some("open") if parts.len() == 2 => {
            WindowOperation::Open(Some(identifier(&parts[1], line)?))
        }
        Some("oldest") if parts.len() == 1 => WindowOperation::Oldest,
        Some("latest") if parts.len() == 1 => WindowOperation::Latest,
        Some("close") if parts.len() == 1 => WindowOperation::Close,
        Some("drag") if parts.len() == 1 => WindowOperation::Drag,
        Some("drag-resize") if parts.len() == 2 => {
            WindowOperation::DragResize(match parts[1].as_str() {
                "north" => WindowDirection::North,
                "south" => WindowDirection::South,
                "east" => WindowDirection::East,
                "west" => WindowDirection::West,
                "north-east" => WindowDirection::NorthEast,
                "north-west" => WindowDirection::NorthWest,
                "south-east" => WindowDirection::SouthEast,
                "south-west" => WindowDirection::SouthWest,
                _ => return Err(error("E053", line, "unknown window resize direction")),
            })
        }
        Some("resize") if parts.len() == 3 => WindowOperation::Resize(expr(1)?, expr(2)?),
        Some("resizable") if parts.len() == 2 => WindowOperation::Resizable(expr(1)?),
        Some("min-size") => WindowOperation::MinSize(size()?),
        Some("max-size") => WindowOperation::MaxSize(size()?),
        Some("resize-increments") => WindowOperation::ResizeIncrements(size()?),
        Some("size") if parts.len() == 1 => WindowOperation::Size,
        Some("maximized") if parts.len() == 1 => WindowOperation::IsMaximized,
        Some("maximize") if parts.len() == 2 => WindowOperation::Maximize(expr(1)?),
        Some("minimized") if parts.len() == 1 => WindowOperation::IsMinimized,
        Some("minimize") if parts.len() == 2 => WindowOperation::Minimize(expr(1)?),
        Some("position") if parts.len() == 1 => WindowOperation::Position,
        Some("scale-factor") if parts.len() == 1 => WindowOperation::ScaleFactor,
        Some("move") if parts.len() == 3 => WindowOperation::Move(expr(1)?, expr(2)?),
        Some("mode") if parts.len() == 1 => WindowOperation::Mode,
        Some("set-mode") if parts.len() == 2 => WindowOperation::SetMode(match parts[1].as_str() {
            "windowed" => WindowMode::Windowed,
            "fullscreen" => WindowMode::Fullscreen,
            "hidden" => WindowMode::Hidden,
            _ => {
                return Err(error(
                    "E053",
                    line,
                    "window mode must be windowed, fullscreen, or hidden",
                ));
            }
        }),
        Some("toggle-maximize") if parts.len() == 1 => WindowOperation::ToggleMaximize,
        Some("toggle-decorations") if parts.len() == 1 => WindowOperation::ToggleDecorations,
        Some("attention") if parts.len() == 2 => {
            WindowOperation::Attention(match parts[1].as_str() {
                "none" => None,
                "critical" => Some(WindowAttention::Critical),
                "informational" => Some(WindowAttention::Informational),
                _ => {
                    return Err(error(
                        "E053",
                        line,
                        "window attention must be none, critical, or informational",
                    ));
                }
            })
        }
        Some("focus") if parts.len() == 1 => WindowOperation::Focus,
        Some("level") if parts.len() == 2 => WindowOperation::SetLevel(match parts[1].as_str() {
            "normal" => WindowLevel::Normal,
            "always-on-bottom" => WindowLevel::AlwaysOnBottom,
            "always-on-top" => WindowLevel::AlwaysOnTop,
            _ => return Err(error("E053", line, "unknown window level")),
        }),
        Some("system-menu") if parts.len() == 1 => WindowOperation::SystemMenu,
        Some("raw-id") if parts.len() == 1 => WindowOperation::RawId,
        Some("screenshot") if parts.len() == 1 => WindowOperation::Screenshot,
        Some("mouse-passthrough") if parts.len() == 2 => {
            WindowOperation::MousePassthrough(expr(1)?)
        }
        Some("monitor-size") if parts.len() == 1 => WindowOperation::MonitorSize,
        Some("automatic-tabbing") if parts.len() == 2 => {
            WindowOperation::AutomaticTabbing(expr(1)?)
        }
        Some("icon") if parts.len() == 4 => WindowOperation::Icon {
            pixels: expr(1)?,
            width: expr(2)?,
            height: expr(3)?,
        },
        Some(_) if source.contains('(') => {
            let (function, args) = parse_signature(source.trim(), line)?;
            WindowOperation::Callback {
                function,
                args: parse_expr_list(&args, line)?,
            }
        }
        _ => {
            return Err(error(
                "E053",
                line,
                "unknown window task or wrong arguments",
            ));
        }
    };
    Ok(Statement::WindowOperation {
        operation,
        target,
        route: route
            .map(|route| parse_route(route.trim(), line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

fn parse_view(line: &Line) -> Result<ViewNode, Error> {
    if let Some(condition) = line.text.strip_prefix("if ") {
        return Ok(ViewNode::If {
            condition: parse_expr(condition, line)?,
            children: line
                .children
                .iter()
                .map(parse_view)
                .collect::<Result<_, _>>()?,
            span: Span::line(line.number),
        });
    }
    if let Some(loop_source) = line.text.strip_prefix("for ") {
        let Some((item, items)) = loop_source.split_once(" in ") else {
            return Err(error("E060", line, "loops use `for item in items`"));
        };
        return Ok(ViewNode::For {
            item: identifier(item.trim(), line)?,
            items: parse_expr(items.trim(), line)?,
            children: line
                .children
                .iter()
                .map(parse_view)
                .collect::<Result<_, _>>()?,
            span: Span::line(line.number),
        });
    }

    let (without_route, route_source) = split_top_marker(&line.text, "->")
        .map_or((line.text.as_str(), None), |(left, right)| {
            (left, Some(right))
        });
    let (core, styles) = split_style_utilities(without_route);
    let parts = split_words(core);
    let Some(kind) = parts.first().map(String::as_str) else {
        return Err(error("E061", line, "empty view node"));
    };
    if route_source.is_some()
        && !matches!(
            kind,
            "button"
                | "checkbox"
                | "toggler"
                | "slider"
                | "radio"
                | "pick"
                | "combo"
                | "markdown"
                | "rich-text"
                | "editor"
                | "extern"
                | "shader"
        )
    {
        return Err(error(
            "E081",
            line,
            format!("`{kind}` does not emit a route payload"),
        ));
    }
    let span = Span::line(line.number);

    match kind {
        "col" | "row" | "scroll" | "grid" | "stack" => {
            let id = parts
                .get(1)
                .filter(|part| part.starts_with('#'))
                .map(|part| parse_id(part, line))
                .transpose()?;
            let option_start = usize::from(id.is_some()) + 1;
            let mut options = parse_layout_options(kind, &parts[option_start..], line)?;
            let children = if kind == "scroll" {
                let scroll = options.scroll.as_mut().expect("scroll options");
                let mut content = Vec::new();
                for child in &line.children {
                    let parts = split_words(&child.text);
                    if matches!(
                        parts.first().map(String::as_str),
                        Some("active" | "hovered" | "dragged")
                    ) {
                        scroll
                            .styles
                            .push(parse_scroll_status_style(&parts, child)?);
                    } else {
                        content.push(parse_view(child)?);
                    }
                }
                if content.len() != 1 {
                    return Err(error(
                        "E062",
                        line,
                        "scroll must have exactly one content child beside status styles",
                    ));
                }
                content
            } else {
                line.children
                    .iter()
                    .map(parse_view)
                    .collect::<Result<_, _>>()?
            };
            Ok(ViewNode::Layout {
                kind: match kind {
                    "col" => Layout::Column,
                    "row" => Layout::Row,
                    "scroll" => Layout::Scroll,
                    "grid" => Layout::Grid,
                    _ => Layout::Stack,
                },
                options: Box::new(options),
                id,
                styles,
                children,
                span,
            })
        }
        "text" => parse_text(&parts, styles, line),
        "rich-text" => parse_rich_text(&parts, styles, route_source, line),
        "container" => parse_container(&parts, styles, line),
        "overlay" => parse_overlay(&parts, styles, line),
        "pane-grid" => parse_pane_grid(&parts, styles, line),
        "input" => parse_input(&parts, styles, line),
        "button" => parse_button(&parts, styles, route_source, line),
        "checkbox" => parse_checkbox(&parts, styles, route_source, line),
        "toggler" => parse_toggler(&parts, styles, route_source, line),
        "slider" => parse_slider(&parts, styles, route_source, line),
        "progress" => parse_progress(&parts, styles, line),
        "radio" => parse_radio(&parts, styles, route_source, line),
        "pick" => parse_pick_list(&parts, styles, route_source, line),
        "combo" => parse_combo_box(&parts, styles, route_source, line),
        "rule" => parse_rule(&parts, styles, line),
        "qr" => parse_qr_code(&parts, styles, line),
        "space" => parse_space(&parts, styles, line),
        "extern" => parse_extern_component(&parts, styles, route_source, line),
        "shader" => parse_shader(&parts, styles, route_source, line),
        "image" | "svg" | "viewer" => parse_media(kind, &parts, styles, line),
        "tooltip" => parse_tooltip(&parts, styles, line),
        "mouse" => parse_mouse_area(&parts, styles, line),
        "canvas" => parse_canvas(&parts, styles, line),
        "theme" => parse_theme(&parts, styles, line),
        "slot" => parse_slot(&parts, styles, line),
        "keyed" => parse_keyed_column(&parts, styles, line),
        "lazy" => parse_lazy(&parts, styles, line),
        "markdown" => parse_markdown(&parts, styles, route_source, line),
        "editor" => parse_text_editor(&parts, styles, route_source, line),
        "table" => parse_table(&parts, styles, line),
        "float" => parse_float(&parts, styles, line),
        "pin" => parse_pin(&parts, styles, line),
        "sensor" => parse_sensor(&parts, styles, line),
        "responsive" => parse_responsive(&parts, styles, line),
        _ if kind.chars().next().is_some_and(char::is_uppercase) => {
            if !styles.is_empty() {
                return Err(error(
                    "E040",
                    line,
                    "component calls do not accept `@` utilities; style the component root",
                ));
            }
            let (name, args, id) = parse_component_call(&parts, line)?;
            let slots = parse_component_slots(&name, line)?;
            Ok(ViewNode::Component {
                name,
                args,
                id,
                slots,
                span,
            })
        }
        _ => Err(error("E064", line, format!("unknown view node `{kind}`"))),
    }
}

fn split_style_utilities(source: &str) -> (&str, Vec<String>) {
    split_top_marker(source, "@").map_or_else(
        || (source.trim(), Vec::new()),
        |(core, styles)| {
            (
                core.trim(),
                styles.split_whitespace().map(str::to_owned).collect(),
            )
        },
    )
}

fn parse_component_slots(component: &str, line: &Line) -> Result<Vec<ComponentSlot>, Error> {
    if line.children.is_empty() {
        return Ok(Vec::new());
    }
    let named = line.children.iter().any(|child| child.text.ends_with(':'));
    if !named {
        let compound = line
            .children
            .iter()
            .map(|child| compound_slot_name(component, child))
            .collect::<Vec<_>>();
        if compound.iter().all(Option::is_some) {
            return line
                .children
                .iter()
                .zip(compound)
                .map(|(child, name)| {
                    Ok(ComponentSlot {
                        name: name.expect("all compound slots are present"),
                        content: Box::new(parse_view(child)?),
                        span: Span::line(child.number),
                    })
                })
                .collect();
        }
        if compound.iter().any(Option::is_some) {
            return Err(error(
                "E040",
                line,
                "cannot mix compound components with direct component children",
            )
            .hint(format!(
                "use only `{component}.Name` children, or wrap direct children in one layout"
            )));
        }
        return match line.children.as_slice() {
            [content] => Ok(vec![ComponentSlot {
                name: "children".into(),
                content: Box::new(parse_view(content)?),
                span: Span::line(content.number),
            }]),
            _ => Err(error(
                "E040",
                line,
                "component children need one root or named `slot:` blocks",
            )
            .hint("wrap siblings in row or col, or write `header:` and `body:` blocks")),
        };
    }

    line.children
        .iter()
        .map(|section| {
            let Some(name) = section.text.strip_suffix(':') else {
                return Err(error(
                    "E040",
                    section,
                    "cannot mix a direct child with named component slots",
                ));
            };
            if section.children.len() != 1 {
                return Err(error(
                    "E040",
                    section,
                    format!("component slot `{}` needs exactly one root", name.trim()),
                ));
            }
            Ok(ComponentSlot {
                name: identifier(name.trim(), section)?,
                content: Box::new(parse_view(&section.children[0])?),
                span: Span::line(section.number),
            })
        })
        .collect()
}

fn compound_slot_name(component: &str, line: &Line) -> Option<String> {
    let head = line.text.split_ascii_whitespace().next()?;
    let name = head.split_once('(').map_or(head, |(name, _)| name);
    let slot = name.strip_prefix(component)?.strip_prefix('.')?;
    (!slot.contains('.'))
        .then(|| identifier(slot, line).ok())
        .flatten()
}

fn parse_container(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if line.children.len() != 1 {
        return Err(error("E184", line, "container requires exactly one child"));
    }
    let id = parts
        .get(1)
        .filter(|part| part.starts_with('#'))
        .map(|part| parse_id(part, line))
        .transpose()?;
    let mut options = ContainerOptions::default();
    let option_start = usize::from(id.is_some()) + 1;
    for part in &parts[option_start..] {
        if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("max-width=") {
            options.max_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max-height=") {
            options.max_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            options.align_x = Some(parse_flex_alignment(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-y=") {
            options.align_y = Some(parse_flex_alignment(value, line)?);
        } else if let Some(value) = part.strip_prefix("clip=") {
            options.clip = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding=") {
            options.padding.all = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-x=") {
            options.padding.x = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-y=") {
            options.padding.y = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-top=") {
            options.padding.top = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-right=") {
            options.padding.right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-bottom=") {
            options.padding.bottom = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-left=") {
            options.padding.left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line).map_err(|_| {
                error(
                    "E184",
                    line,
                    "container style must be a declared style call",
                )
            })?;
            options.custom_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if parse_container_style_option(part, &mut options.style, line)? {
        } else {
            return Err(error(
                "E184",
                line,
                format!("unknown container property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Container {
        options: Box::new(options),
        id,
        styles,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_overlay(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E185",
            line,
            "overlay uses typed properties and does not accept `@` utilities",
        ));
    }
    if line.children.len() != 2
        || line.children[0].text != "content"
        || line.children[1].text != "layer"
    {
        return Err(error(
            "E185",
            line,
            "overlay requires `content` then `layer` sections",
        ));
    }
    for section in &line.children {
        if section.children.len() != 1 {
            return Err(error(
                "E185",
                section,
                format!(
                    "overlay `{}` requires exactly one child; wrap siblings in row, col, grid, or stack",
                    section.text
                ),
            ));
        }
    }

    let mut visible = None;
    let mut dismiss = None;
    let mut backdrop = "black/50".to_owned();
    let mut padding = Expr::F64(24.0);
    let mut align_x = FlexAlignment::Center;
    let mut align_y = FlexAlignment::Center;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("when=") {
            visible = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("dismiss=") {
            dismiss = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("backdrop=") {
            backdrop = value.to_owned();
        } else if let Some(value) = part.strip_prefix("padding=") {
            padding = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("align-x=") {
            align_x = parse_flex_alignment(value, line)?;
        } else if let Some(value) = part.strip_prefix("align-y=") {
            align_y = parse_flex_alignment(value, line)?;
        } else {
            return Err(error(
                "E185",
                line,
                format!("unknown overlay property `{part}`"),
            ));
        }
    }
    let visible = visible.ok_or_else(|| error("E185", line, "overlay requires `when=`"))?;
    Ok(ViewNode::Overlay {
        options: OverlayOptions {
            visible,
            dismiss,
            backdrop,
            padding,
            align_x,
            align_y,
        },
        content: Box::new(parse_view(&line.children[0].children[0])?),
        layer: Box::new(parse_view(&line.children[1].children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_pane_ratio(value: &str, line: &Line) -> Result<f32, Error> {
    let ratio = value.parse::<f32>().map_err(|_| {
        error(
            "E187",
            line,
            "pane split ratio must be a number from 0 to 1",
        )
    })?;
    if !(0.0..=1.0).contains(&ratio) {
        return Err(error(
            "E187",
            line,
            "pane split ratio must be a number from 0 to 1",
        ));
    }
    Ok(ratio)
}

fn parse_background_value(source: &str, line: &Line) -> Result<BackgroundValue, Error> {
    let Some(inner) = source
        .strip_prefix("linear(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        if source.starts_with("linear(") {
            return Err(error("E189", line, "linear background is missing `)`"));
        }
        return Ok(BackgroundValue::Color(source.to_owned()));
    };
    let parts = split_top(inner, ',');
    let angle = parse_expr(
        parts
            .first()
            .copied()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| error("E189", line, "linear background requires an angle"))?,
        line,
    )?;
    if parts.len() > 9 {
        return Err(error(
            "E189",
            line,
            "linear background accepts at most 8 color stops",
        ));
    }
    let stops = parts[1..]
        .iter()
        .map(|stop| {
            let (color, offset) = split_top_once(stop, '@')
                .ok_or_else(|| error("E189", line, "linear color stops use `color@offset`"))?;
            if color.is_empty() || offset.is_empty() {
                return Err(error("E189", line, "linear color stops use `color@offset`"));
            }
            Ok(GradientStop {
                color: color.to_owned(),
                offset: parse_expr(offset, line)?,
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(BackgroundValue::Linear { angle, stops })
}

fn parse_container_style_option(
    part: &str,
    style: &mut ContainerStyleOptions,
    line: &Line,
) -> Result<bool, Error> {
    let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
    if let Some(value) = part.strip_prefix("background=") {
        style.background = Some(parse_background_value(value, line)?);
    } else if let Some(value) = part.strip_prefix("text=") {
        style.text_color = Some(value.to_owned());
    } else if let Some(value) = part.strip_prefix("border=") {
        style.border_color = Some(value.to_owned());
    } else if let Some(value) = part.strip_prefix("border-width=") {
        style.border_width = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("radius=") {
        style.radius = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("radius-tl=") {
        style.radius_top_left = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("radius-tr=") {
        style.radius_top_right = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("radius-br=") {
        style.radius_bottom_right = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("radius-bl=") {
        style.radius_bottom_left = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("shadow=") {
        style.shadow_color = Some(value.to_owned());
    } else if let Some(value) = part.strip_prefix("shadow-x=") {
        style.shadow_x = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("shadow-y=") {
        style.shadow_y = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("shadow-blur=") {
        style.shadow_blur = Some(parse(value)?);
    } else if let Some(value) = part.strip_prefix("pixel-snap=") {
        style.pixel_snap = Some(parse(value)?);
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn parse_pane_view(
    name: &str,
    style_parts: &[String],
    styles: Vec<String>,
    line: &Line,
    names: &mut std::collections::HashSet<String>,
    panes: &mut Vec<PaneView>,
) -> Result<String, Error> {
    let name = identifier(name, line)?;
    if !names.insert(name.clone()) {
        return Err(error("E187", line, format!("duplicate pane `{name}`")));
    }
    let mut style = ContainerStyleOptions::default();
    for part in style_parts {
        if !parse_container_style_option(part, &mut style, line)? {
            return Err(error(
                "E187",
                line,
                format!("unknown pane style property `{part}`"),
            ));
        }
    }
    let structured = line.children.iter().any(|child| {
        let (core, _) = split_style_utilities(&child.text);
        split_words(core).first().is_some_and(|kind| {
            matches!(
                kind.as_str(),
                "title" | "controls" | "compact-controls" | "content"
            )
        })
    });
    let (content, title) = if structured {
        parse_structured_pane(line)?
    } else {
        if line.children.len() != 1 {
            return Err(error(
                "E187",
                line,
                "pane requires exactly one child; wrap siblings in row or col",
            ));
        }
        (Box::new(parse_view(&line.children[0])?), None)
    };
    panes.push(PaneView {
        name: name.clone(),
        content,
        title,
        styles,
        style,
        span: Span::line(line.number),
    });
    Ok(name)
}

fn parse_structured_pane(line: &Line) -> Result<(Box<ViewNode>, Option<PaneTitle>), Error> {
    let mut content = None;
    let mut title = None;
    let mut controls = None;
    let mut compact_controls = None;
    for section in &line.children {
        let (core, styles) = split_style_utilities(&section.text);
        let parts = split_words(core);
        let kind = parts.first().map(String::as_str).unwrap_or("");
        if section.children.len() != 1 {
            return Err(error(
                "E187",
                section,
                format!("pane `{kind}` section requires exactly one child"),
            ));
        }
        let node = || parse_view(&section.children[0]).map(Box::new);
        match kind {
            "content" if parts.len() == 1 && styles.is_empty() => {
                if content.is_some() {
                    return Err(error("E187", section, "duplicate pane `content` section"));
                }
                content = Some(node()?);
            }
            "title" => {
                if title.is_some() {
                    return Err(error("E187", section, "duplicate pane `title` section"));
                }
                title = Some(parse_pane_title(&parts[1..], styles, section)?);
            }
            "controls" if parts.len() == 1 && styles.is_empty() => {
                if controls.is_some() {
                    return Err(error("E187", section, "duplicate pane `controls` section"));
                }
                controls = Some(node()?);
            }
            "compact-controls" if parts.len() == 1 && styles.is_empty() => {
                if compact_controls.is_some() {
                    return Err(error(
                        "E187",
                        section,
                        "duplicate pane `compact-controls` section",
                    ));
                }
                compact_controls = Some(node()?);
            }
            "content" | "controls" | "compact-controls" => {
                return Err(error(
                    "E187",
                    section,
                    format!("pane `{kind}` section does not accept properties or styles"),
                ));
            }
            _ => {
                return Err(error(
                    "E187",
                    section,
                    "structured pane children must be title, controls, compact-controls, or content sections",
                ));
            }
        }
    }
    let content =
        content.ok_or_else(|| error("E187", line, "structured pane requires `content`"))?;
    if controls.is_some() && title.is_none() {
        return Err(error(
            "E187",
            line,
            "pane controls require a `title` section",
        ));
    }
    if compact_controls.is_some() && controls.is_none() {
        return Err(error(
            "E187",
            line,
            "pane compact-controls require a `controls` section",
        ));
    }
    if title
        .as_ref()
        .is_some_and(|title| title.always_show_controls)
        && controls.is_none()
    {
        return Err(error(
            "E187",
            line,
            "pane title `always-controls` requires a `controls` section",
        ));
    }
    if let Some(title) = &mut title {
        title.controls = controls;
        title.compact_controls = compact_controls;
    }
    Ok((content, title))
}

fn parse_pane_title(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<PaneTitle, Error> {
    let mut padding = PaddingOptions::default();
    let mut always_show_controls = false;
    let mut style = ContainerStyleOptions::default();
    for part in parts {
        let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
        if let Some(value) = part.strip_prefix("padding=") {
            padding.all = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("padding-x=") {
            padding.x = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("padding-y=") {
            padding.y = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("padding-top=") {
            padding.top = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("padding-right=") {
            padding.right = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("padding-bottom=") {
            padding.bottom = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("padding-left=") {
            padding.left = Some(parse(value)?);
        } else if part == "always-controls" {
            always_show_controls = true;
        } else if parse_container_style_option(part, &mut style, line)? {
        } else {
            return Err(error(
                "E187",
                line,
                format!("unknown pane title property `{part}`"),
            ));
        }
    }
    Ok(PaneTitle {
        content: Box::new(parse_view(&line.children[0])?),
        controls: None,
        compact_controls: None,
        padding,
        always_show_controls,
        styles,
        style,
        span: Span::line(line.number),
    })
}

fn parse_pane_configuration(
    line: &Line,
    names: &mut std::collections::HashSet<String>,
    panes: &mut Vec<PaneView>,
) -> Result<PaneConfiguration, Error> {
    let (core, styles) = split_style_utilities(&line.text);
    let parts = split_words(core);
    match parts.first().map(String::as_str) {
        Some("pane") if parts.len() >= 2 => Ok(PaneConfiguration::Pane(parse_pane_view(
            &parts[1],
            &parts[2..],
            styles,
            line,
            names,
            panes,
        )?)),
        Some("split") if (2..=3).contains(&parts.len()) => {
            if !styles.is_empty() {
                return Err(error("E187", line, "nested pane split does not accept `@`"));
            }
            let axis = match parts[1].as_str() {
                "horizontal" => PaneAxis::Horizontal,
                "vertical" => PaneAxis::Vertical,
                _ => {
                    return Err(error(
                        "E187",
                        line,
                        "nested pane split must be horizontal or vertical",
                    ));
                }
            };
            let ratio = parts.get(2).map_or(Ok(0.5), |part| {
                parse_pane_ratio(
                    part.strip_prefix("ratio=").ok_or_else(|| {
                        error("E187", line, "nested pane split ratio uses `ratio=value`")
                    })?,
                    line,
                )
            })?;
            if line.children.len() != 2 {
                return Err(error(
                    "E187",
                    line,
                    "nested pane split requires exactly two pane or split children",
                ));
            }
            Ok(PaneConfiguration::Split {
                axis,
                ratio,
                a: Box::new(parse_pane_configuration(&line.children[0], names, panes)?),
                b: Box::new(parse_pane_configuration(&line.children[1], names, panes)?),
            })
        }
        _ => Err(error(
            "E187",
            line,
            "pane configuration uses `pane name` or `split axis ratio=value`",
        )),
    }
}

fn parse_closed_pane(
    line: &Line,
    names: &mut std::collections::HashSet<String>,
    panes: &mut Vec<PaneView>,
) -> Result<(), Error> {
    let (core, styles) = split_style_utilities(&line.text);
    let parts = split_words(core);
    if parts.len() < 3 || parts[0] != "pane" || parts[2] != "closed" {
        return Err(error(
            "E187",
            line,
            "extra pane templates use `pane name closed`",
        ));
    }
    parse_pane_view(&parts[1], &parts[3..], styles, line, names, panes)?;
    Ok(())
}

fn parse_pane_grid(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E187",
            line,
            "pane-grid does not accept `@` utilities",
        ));
    }
    let name = parts
        .get(1)
        .filter(|part| part.starts_with('#'))
        .ok_or_else(|| error("E187", line, "pane-grid requires a static `#id`"))?;
    let name = identifier(name.trim_start_matches('#'), line)?;
    let mut legacy_axis = None;
    let mut legacy_ratio = 0.5_f32;
    let mut legacy_ratio_set = false;
    let mut options = PaneGridOptions::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("split=") {
            legacy_axis = Some(match value {
                "horizontal" => PaneAxis::Horizontal,
                "vertical" => PaneAxis::Vertical,
                _ => {
                    return Err(error(
                        "E187",
                        line,
                        "pane-grid split must be horizontal or vertical",
                    ));
                }
            });
        } else if let Some(value) = part.strip_prefix("ratio=") {
            legacy_ratio = parse_pane_ratio(value, line)?;
            legacy_ratio_set = true;
        } else if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("spacing=") {
            options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("min-size=") {
            options.min_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("resize=") {
            options.resize_leeway = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "drag" {
            options.draggable = true;
        } else if let Some(value) = part.strip_prefix("click=") {
            options.click = Some(parse_route(value, line)?);
        } else {
            return Err(error(
                "E187",
                line,
                format!("unknown pane-grid property `{part}`"),
            ));
        }
    }
    let children = if line
        .children
        .first()
        .is_some_and(|child| child.text == "style")
    {
        options.style = parse_pane_grid_style(&line.children[0])?;
        &line.children[1..]
    } else {
        if line
            .children
            .iter()
            .skip(1)
            .any(|child| child.text == "style")
        {
            return Err(error(
                "E187",
                line,
                "pane-grid `style` must be its first child",
            ));
        }
        line.children.as_slice()
    };
    let mut names = std::collections::HashSet::new();
    let mut panes = Vec::new();
    let configuration = if let Some(axis) = legacy_axis {
        if children.len() < 2 {
            return Err(error(
                "E187",
                line,
                "pane-grid shorthand requires two open `pane name` children",
            ));
        }
        let open = &children[..2];
        let a = parse_pane_configuration(&open[0], &mut names, &mut panes)?;
        let b = parse_pane_configuration(&open[1], &mut names, &mut panes)?;
        if !matches!(&a, PaneConfiguration::Pane(_)) || !matches!(&b, PaneConfiguration::Pane(_)) {
            return Err(error(
                "E187",
                line,
                "pane-grid shorthand accepts two open panes; use a nested split tree instead",
            ));
        }
        for pane in &children[2..] {
            parse_closed_pane(pane, &mut names, &mut panes)?;
        }
        PaneConfiguration::Split {
            axis,
            ratio: legacy_ratio,
            a: Box::new(a),
            b: Box::new(b),
        }
    } else {
        if legacy_ratio_set {
            return Err(error(
                "E187",
                line,
                "pane-grid `ratio=` requires legacy `split=` or a nested split node",
            ));
        }
        let (configuration, closed) = children.split_first().ok_or_else(|| {
            error(
                "E187",
                line,
                "pane-grid requires an initial pane or split configuration",
            )
        })?;
        let configuration = parse_pane_configuration(configuration, &mut names, &mut panes)?;
        for pane in closed {
            parse_closed_pane(pane, &mut names, &mut panes)?;
        }
        configuration
    };
    Ok(ViewNode::PaneGrid {
        name,
        configuration,
        options,
        panes,
        span: Span::line(line.number),
    })
}

fn parse_pane_grid_style(line: &Line) -> Result<PaneGridStyle, Error> {
    if line.children.is_empty() {
        return Err(error(
            "E187",
            line,
            "pane-grid style requires at least one status",
        ));
    }
    let mut style = PaneGridStyle::default();
    let mut statuses = std::collections::HashSet::new();
    for status in &line.children {
        if !status.children.is_empty() {
            return Err(error("E187", status, "pane-grid style statuses are leaves"));
        }
        let parts = split_words(&status.text);
        let kind = parts.first().map(String::as_str).unwrap_or("");
        if !statuses.insert(kind.to_owned()) {
            return Err(error(
                "E187",
                status,
                format!("duplicate pane-grid style status `{kind}`"),
            ));
        }
        if parts.len() == 1 {
            return Err(error(
                "E187",
                status,
                format!("pane-grid style status `{kind}` requires properties"),
            ));
        }
        let parse = |value: &str| parse_expr(strip_wrapping_parens(value), status);
        for part in &parts[1..] {
            match kind {
                "hovered-region" => {
                    if let Some(value) = part.strip_prefix("background=") {
                        style.region_background = Some(parse_background_value(value, status)?);
                    } else if let Some(value) = part.strip_prefix("border=") {
                        style.region_border = Some(value.to_owned());
                    } else if let Some(value) = part.strip_prefix("border-width=") {
                        style.region_border_width = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("radius=") {
                        style.region_radius = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("radius-tl=") {
                        style.region_radius_top_left = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("radius-tr=") {
                        style.region_radius_top_right = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("radius-br=") {
                        style.region_radius_bottom_right = Some(parse(value)?);
                    } else if let Some(value) = part.strip_prefix("radius-bl=") {
                        style.region_radius_bottom_left = Some(parse(value)?);
                    } else {
                        return Err(error(
                            "E187",
                            status,
                            format!("unknown hovered-region style property `{part}`"),
                        ));
                    }
                }
                "hovered-split" | "picked-split" => {
                    let (color, width) = if kind == "hovered-split" {
                        (&mut style.hovered_split, &mut style.hovered_split_width)
                    } else {
                        (&mut style.picked_split, &mut style.picked_split_width)
                    };
                    if let Some(value) = part.strip_prefix("color=") {
                        *color = Some(value.to_owned());
                    } else if let Some(value) = part.strip_prefix("width=") {
                        *width = Some(parse(value)?);
                    } else {
                        return Err(error(
                            "E187",
                            status,
                            format!("unknown {kind} style property `{part}`"),
                        ));
                    }
                }
                _ => {
                    return Err(error(
                        "E187",
                        status,
                        "pane-grid style status must be hovered-region, hovered-split, or picked-split",
                    ));
                }
            }
        }
    }
    Ok(style)
}

fn parse_component_call(
    parts: &[String],
    line: &Line,
) -> Result<(String, Vec<ComponentArg>, Option<Id>), Error> {
    let head = &parts[0];
    if head.contains('(') {
        let (name, args) = parse_component_signature(head, line)?;
        let id = parts
            .get(1)
            .filter(|part| part.starts_with('#'))
            .map(|part| parse_id(part, line))
            .transpose()?;
        if parts.len() > 1 + usize::from(id.is_some()) {
            return Err(error(
                "E040",
                line,
                "positional component calls only accept `Name(...)` and an optional ID",
            ));
        }
        return Ok((
            name,
            parse_expr_list(&args, line)?
                .into_iter()
                .map(|value| ComponentArg { name: None, value })
                .collect(),
            id,
        ));
    }

    let name = component_identifier(head, line)?;
    let mut args = Vec::new();
    let mut id = None;
    for part in &parts[1..] {
        if part.starts_with('#') {
            if id.is_some() {
                return Err(error("E040", line, "component call has more than one ID"));
            }
            id = Some(parse_id(part, line)?);
            continue;
        }
        let Some((prop, value)) = split_top_once(part, '=') else {
            return Err(error("E040", line, "component props use `name=value`"));
        };
        args.push(ComponentArg {
            name: Some(identifier(prop.trim(), line)?),
            value: parse_expr(strip_wrapping_parens(value.trim()), line)?,
        });
    }
    Ok((name, args, id))
}

fn parse_text_editor(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E099", line, "editor does not accept `@` utilities"));
    }
    let mut binding = None;
    let mut id = None;
    let mut disabled = None;
    let mut options = TextEditorOptions::default();
    let mut index = 1;
    while index < parts.len() {
        let part = &parts[index];
        if part.starts_with('#') {
            id = Some(parse_id(part, line)?);
        } else if part == "<->" {
            index += 1;
            binding = Some(identifier(
                parts
                    .get(index)
                    .ok_or_else(|| error("E099", line, "missing editor binding"))?,
                line,
            )?);
        } else if let Some(value) = part.strip_prefix("placeholder=") {
            options.placeholder = Some(string_literal(value, line)?);
        } else if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("min-height=") {
            options.min_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max-height=") {
            options.max_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            options.line_height = Some(TextLineHeight::Relative(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("line-height-px=") {
            options.line_height = Some(TextLineHeight::Absolute(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("padding=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("wrapping=") {
            options.wrapping = Some(parse_text_wrapping(value, line, "E099")?);
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("highlight=") {
            options.highlight = Some(string_literal(value, line)?);
        } else if let Some(value) = part.strip_prefix("highlight-theme=") {
            options.highlight_theme = Some(match value {
                "solarized-dark" => HighlightTheme::SolarizedDark,
                "base16-mocha" => HighlightTheme::Base16Mocha,
                "base16-ocean" => HighlightTheme::Base16Ocean,
                "base16-eighties" => HighlightTheme::Base16Eighties,
                "inspired-github" => HighlightTheme::InspiredGithub,
                _ => return Err(error("E099", line, "unknown editor highlight theme")),
            });
        } else if let Some(value) = part.strip_prefix("highlighter=") {
            let (function, args) = parse_signature(value, line)?;
            options.highlighter = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("key-binding=") {
            let (function, args) = parse_signature(value, line)?;
            options.key_binding = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E099", line, "editor style must be a declared style call"))?;
            options.custom_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E099",
                line,
                format!("unknown editor property `{part}`"),
            ));
        }
        index += 1;
    }
    if options.highlight.is_none() && options.highlight_theme.is_some() {
        return Err(error("E099", line, "highlight-theme requires highlight"));
    }
    if options.highlight.is_some() && options.highlighter.is_some() {
        return Err(error(
            "E099",
            line,
            "editor accepts either highlight or highlighter, not both",
        ));
    }
    options.key_binding_route = match (&options.key_binding, route) {
        (Some(_), Some(route)) => Some(parse_route(route.trim(), line)?),
        (Some(_), None) => {
            return Err(error(
                "E099",
                line,
                "key-binding requires `-> handler _` for custom bindings",
            ));
        }
        (None, Some(_)) => {
            return Err(error(
                "E099",
                line,
                "an editor route requires key-binding=name(args)",
            ));
        }
        (None, None) => None,
    };
    for child in &line.children {
        let parts = split_words(&child.text);
        match parts.first().map(String::as_str) {
            Some("active" | "hovered" | "focused" | "focused-hovered" | "disabled") => {
                ensure_leaf(child)?;
                parse_text_input_status(
                    &parts,
                    child,
                    &mut options.style,
                    "E099",
                    "editor",
                    false,
                )?;
            }
            _ => {
                return Err(error(
                    "E099",
                    child,
                    "editor blocks use active, hovered, focused, focused-hovered, or disabled",
                ));
            }
        }
    }
    Ok(ViewNode::TextEditor {
        binding: binding.ok_or_else(|| error("E099", line, "editor requires `<-> state`"))?,
        id,
        disabled,
        options,
        span: Span::line(line.number),
    })
}

fn parse_table(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E098", line, "table does not accept `@` utilities"));
    }
    if parts.len() < 4 || parts.get(2).map(String::as_str) != Some("in") {
        return Err(error("E098", line, "table uses `table item in rows`"));
    }
    if line.children.is_empty() {
        return Err(error("E098", line, "table requires at least one column"));
    }
    let mut options = TableOptions::default();
    for part in &parts[4..] {
        if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else {
            let (name, value) = part
                .split_once('=')
                .ok_or_else(|| error("E098", line, format!("unknown table property `{part}`")))?;
            let value = parse_expr(strip_wrapping_parens(value), line)?;
            match name {
                "padding" => options.padding = Some(value),
                "padding-x" => options.padding_x = Some(value),
                "padding-y" => options.padding_y = Some(value),
                "separator" => options.separator = Some(value),
                "separator-x" => options.separator_x = Some(value),
                "separator-y" => options.separator_y = Some(value),
                _ => {
                    return Err(error(
                        "E098",
                        line,
                        format!("unknown table property `{name}`"),
                    ));
                }
            }
        }
    }
    Ok(ViewNode::Table {
        item: identifier(&parts[1], line)?,
        rows: parse_expr(strip_wrapping_parens(&parts[3]), line)?,
        options,
        columns: line
            .children
            .iter()
            .map(parse_table_column)
            .collect::<Result<_, _>>()?,
        span: Span::line(line.number),
    })
}

fn parse_table_column(line: &Line) -> Result<TableColumn, Error> {
    let parts = split_words(&line.text);
    if parts.first().map(String::as_str) != Some("column") {
        return Err(error("E098", line, "table children must be columns"));
    }
    let mut width = None;
    let mut align_x = None;
    let mut align_y = None;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("width=") {
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            align_x = Some(match value {
                "left" => InputAlignment::Left,
                "center" => InputAlignment::Center,
                "right" => InputAlignment::Right,
                _ => {
                    return Err(error(
                        "E098",
                        line,
                        "column align-x must be left, center, or right",
                    ));
                }
            });
        } else if let Some(value) = part.strip_prefix("align-y=") {
            align_y = Some(match value {
                "top" => VerticalAlignment::Top,
                "center" => VerticalAlignment::Center,
                "bottom" => VerticalAlignment::Bottom,
                _ => {
                    return Err(error(
                        "E098",
                        line,
                        "column align-y must be top, center, or bottom",
                    ));
                }
            });
        } else {
            return Err(error(
                "E098",
                line,
                format!("unknown column property `{part}`"),
            ));
        }
    }
    if line.children.len() != 2 {
        return Err(error(
            "E098",
            line,
            "column requires one header and one cell",
        ));
    }
    let parse_part = |part: &Line, expected: &str| {
        if part.text != expected || part.children.len() != 1 {
            return Err(error(
                "E098",
                part,
                format!("column `{expected}` requires exactly one child"),
            ));
        }
        parse_view(&part.children[0])
    };
    Ok(TableColumn {
        width,
        align_x,
        align_y,
        header: parse_part(&line.children[0], "header")?,
        cell: parse_part(&line.children[1], "cell")?,
        span: Span::line(line.number),
    })
}

fn parse_markdown(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E097",
            line,
            "markdown does not accept `@` utilities",
        ));
    }
    let content = parts
        .get(1)
        .ok_or_else(|| error("E097", line, "markdown requires a content state"))?;
    let route = route.ok_or_else(|| {
        error(
            "E097",
            line,
            "markdown requires a link route such as `-> open_link _`",
        )
    })?;
    let mut options = MarkdownOptions::default();
    for part in &parts[2..] {
        let (name, value) = part
            .split_once('=')
            .ok_or_else(|| error("E097", line, format!("unknown markdown property `{part}`")))?;
        match name {
            "text-size" => {
                options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?)
            }
            "h1-size" => options.h1_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h2-size" => options.h2_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h3-size" => options.h3_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h4-size" => options.h4_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h5-size" => options.h5_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "h6-size" => options.h6_size = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "code-size" => {
                options.code_size = Some(parse_expr(strip_wrapping_parens(value), line)?)
            }
            "spacing" => options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?),
            "viewer" => {
                let (function, args) = parse_signature(value, line)?;
                options.viewer = Some(ExternCall {
                    function,
                    args: parse_expr_list(&args, line)?,
                });
            }
            _ => {
                return Err(error(
                    "E097",
                    line,
                    format!("unknown markdown property `{name}`"),
                ));
            }
        }
    }
    options.style = match line.children.as_slice() {
        [] => MarkdownStyleOptions::default(),
        [style] => parse_markdown_style(style)?,
        _ => {
            return Err(error(
                "E097",
                line,
                "markdown accepts at most one `style` child",
            ));
        }
    };
    Ok(ViewNode::Markdown {
        content: identifier(content, line)?,
        options: Box::new(options),
        route: parse_route(route, line)?,
        span: Span::line(line.number),
    })
}

fn parse_markdown_style(line: &Line) -> Result<MarkdownStyleOptions, Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    if parts.first().map(String::as_str) != Some("style") {
        return Err(error("E097", line, "markdown child must be `style`"));
    }
    let mut style = MarkdownStyleOptions::default();
    let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
    for part in &parts[1..] {
        let Some((name, value)) = part.split_once('=') else {
            return Err(error(
                "E097",
                line,
                format!("unknown markdown style property `{part}`"),
            ));
        };
        match name {
            "font" => style.font = Some(parse_font_preset(value, line)?),
            "inline-code-background" => {
                style.inline_code_background = Some(parse_background_value(value, line)?)
            }
            "inline-code-color" => style.inline_code_color = Some(value.to_owned()),
            "inline-code-font" => style.inline_code_font = Some(parse_font_preset(value, line)?),
            "code-block-font" => style.code_block_font = Some(parse_font_preset(value, line)?),
            "link" => style.link_color = Some(value.to_owned()),
            "inline-code-padding" => style.inline_code_padding.all = Some(parse(value)?),
            "inline-code-padding-x" => style.inline_code_padding.x = Some(parse(value)?),
            "inline-code-padding-y" => style.inline_code_padding.y = Some(parse(value)?),
            "inline-code-padding-top" => style.inline_code_padding.top = Some(parse(value)?),
            "inline-code-padding-right" => style.inline_code_padding.right = Some(parse(value)?),
            "inline-code-padding-bottom" => style.inline_code_padding.bottom = Some(parse(value)?),
            "inline-code-padding-left" => style.inline_code_padding.left = Some(parse(value)?),
            "inline-code-border" => style.inline_code_border_color = Some(value.to_owned()),
            "inline-code-border-width" => style.inline_code_border_width = Some(parse(value)?),
            "inline-code-radius" => style.inline_code_radius = Some(parse(value)?),
            "inline-code-radius-tl" => style.inline_code_radius_top_left = Some(parse(value)?),
            "inline-code-radius-tr" => style.inline_code_radius_top_right = Some(parse(value)?),
            "inline-code-radius-br" => style.inline_code_radius_bottom_right = Some(parse(value)?),
            "inline-code-radius-bl" => style.inline_code_radius_bottom_left = Some(parse(value)?),
            _ => {
                return Err(error(
                    "E097",
                    line,
                    format!("unknown markdown style property `{name}`"),
                ));
            }
        }
    }
    Ok(style)
}

fn parse_lazy(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E096", line, "lazy does not accept `@` utilities"));
    }
    if parts.len() != 4 || parts[2] != "as" {
        return Err(error("E096", line, "lazy uses `lazy dependency as name`"));
    }
    if line.children.len() != 1 {
        return Err(error(
            "E096",
            line,
            "lazy requires exactly one child subtree",
        ));
    }
    Ok(ViewNode::Lazy {
        dependency: parse_expr(strip_wrapping_parens(&parts[1]), line)?,
        binding: identifier(&parts[3], line)?,
        child: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_keyed_column(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E095", line, "keyed does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error(
            "E095",
            line,
            "keyed requires exactly one child template",
        ));
    }
    if parts.len() < 5 || parts.get(2).map(String::as_str) != Some("in") {
        return Err(error(
            "E095",
            line,
            "keyed uses `keyed item in items by=item.id`",
        ));
    }
    let key = parts[4]
        .strip_prefix("by=")
        .ok_or_else(|| error("E095", line, "keyed uses `keyed item in items by=item.id`"))?;
    let options = parse_layout_options("col", &parts[5..], line)?;
    if options.clip.is_some() || options.wrap {
        return Err(error(
            "E095",
            line,
            "keyed columns do not support clip or wrap",
        ));
    }
    Ok(ViewNode::KeyedColumn {
        item: identifier(&parts[1], line)?,
        items: parse_expr(strip_wrapping_parens(&parts[3]), line)?,
        key: parse_expr(strip_wrapping_parens(key), line)?,
        options: Box::new(options),
        child: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_slot(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !(1..=2).contains(&parts.len()) || !styles.is_empty() {
        return Err(error(
            "E040",
            line,
            "slot accepts an optional name and no properties or styles",
        ));
    }
    Ok(ViewNode::Slot {
        name: parts
            .get(1)
            .map(|name| identifier(name, line))
            .transpose()?
            .unwrap_or_else(|| "children".into()),
        span: Span::line(line.number),
    })
}

fn parse_theme(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E094", line, "theme does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E094", line, "theme requires exactly one child"));
    }
    let mut preset = ThemePreset::Default;
    let mut text = None;
    let mut background = None;
    let mut start = 1;
    if let Some(value) = parts.get(1)
        && !value.contains('=')
    {
        preset = parse_theme_preset(value, line)?;
        start = 2;
    }
    for part in &parts[start..] {
        if let Some(value) = part.strip_prefix("text=") {
            text = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("background=") {
            background = Some(parse_background_value(value, line)?);
        } else {
            return Err(error(
                "E094",
                line,
                format!("unknown theme property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Theme {
        preset,
        text,
        background,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_theme_preset(value: &str, line: &Line) -> Result<ThemePreset, Error> {
    match value {
        "default" => Ok(ThemePreset::Default),
        "app" => Ok(ThemePreset::App),
        value if BUILT_IN_THEMES.contains(&value) => Ok(ThemePreset::BuiltIn(value.into())),
        _ => Err(error("E094", line, format!("unknown iced theme `{value}`"))),
    }
}

fn parse_qr_code(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error("E093", line, "qr does not accept `@` utilities"));
    }
    let data = parts
        .get(1)
        .ok_or_else(|| error("E093", line, "qr needs a declared data name"))?;
    let mut cell_size = None;
    let mut total_size = None;
    let mut cell = None;
    let mut background = None;
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("cell-size=") {
            cell_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("total-size=") {
            total_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("cell=") {
            cell = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("background=") {
            background = Some(value.to_owned());
        } else {
            return Err(error("E093", line, format!("unknown qr property `{part}`")));
        }
    }
    if cell_size.is_some() && total_size.is_some() {
        return Err(error(
            "E093",
            line,
            "qr accepts either cell-size or total-size, not both",
        ));
    }
    Ok(ViewNode::QrCode {
        data: identifier(data, line)?,
        cell_size,
        total_size,
        cell,
        background,
        span: Span::line(line.number),
    })
}

fn parse_float(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E089", line, "float does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E089", line, "float requires exactly one child"));
    }
    let mut scale = Expr::F64(1.0);
    let mut x = Expr::F64(0.0);
    let mut y = Expr::F64(0.0);
    let mut style = FloatStyleOptions::default();
    let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("scale=") {
            scale = parse(value)?;
        } else if let Some(value) = part.strip_prefix("x=") {
            x = parse(value)?;
        } else if let Some(value) = part.strip_prefix("y=") {
            y = parse(value)?;
        } else if let Some(value) = part.strip_prefix("shadow=") {
            style.shadow_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("shadow-x=") {
            style.shadow_x = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("shadow-y=") {
            style.shadow_y = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("shadow-blur=") {
            style.shadow_blur = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius=") {
            style.radius = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-tl=") {
            style.radius_top_left = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-tr=") {
            style.radius_top_right = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-br=") {
            style.radius_bottom_right = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-bl=") {
            style.radius_bottom_left = Some(parse(value)?);
        } else {
            return Err(error(
                "E089",
                line,
                format!("unknown float property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Float {
        scale,
        x,
        y,
        style,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_pin(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E090", line, "pin does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E090", line, "pin requires exactly one child"));
    }
    let mut width = None;
    let mut height = None;
    let mut x = Expr::F64(0.0);
    let mut y = Expr::F64(0.0);
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("width=") {
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("x=") {
            x = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("y=") {
            y = parse_expr(strip_wrapping_parens(value), line)?;
        } else {
            return Err(error(
                "E090",
                line,
                format!("unknown pin property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Pin {
        width,
        height,
        x,
        y,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_sensor(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E091", line, "sensor does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E091", line, "sensor requires exactly one child"));
    }
    let mut options = SensorOptions::default();
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("show=") {
            options.show = Some(parse_size_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("resize=") {
            options.resize = Some(parse_size_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("hide=") {
            options.hide = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("key=") {
            options.key = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("anticipate=") {
            options.anticipate = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("delay=") {
            options.delay_ms = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E091",
                line,
                format!("unknown sensor property `{part}`"),
            ));
        }
    }
    if options.show.is_none() && options.resize.is_none() && options.hide.is_none() {
        return Err(error("E091", line, "sensor requires show, resize, or hide"));
    }
    Ok(ViewNode::Sensor {
        options,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_responsive(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E092",
            line,
            "responsive does not accept `@` utilities",
        ));
    }
    let mut breakpoint = None;
    let mut size = None;
    let mut width = None;
    let mut height = None;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("at=") {
            if breakpoint.is_some() {
                return Err(error("E092", line, "responsive repeats `at=`"));
            }
            breakpoint = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            if size.is_some() {
                return Err(error("E092", line, "responsive repeats `size=`"));
            }
            let Some(value) = value
                .strip_prefix('(')
                .and_then(|value| value.strip_suffix(')'))
            else {
                return Err(error(
                    "E092",
                    line,
                    "responsive size bindings use `size=(width, height)`",
                ));
            };
            let names = split_top(value, ',');
            let [width, height] = names.as_slice() else {
                return Err(error(
                    "E092",
                    line,
                    "responsive size expects width and height bindings",
                ));
            };
            let width = identifier(width, line)?;
            let height = identifier(height, line)?;
            if width == height {
                return Err(error(
                    "E092",
                    line,
                    "responsive size bindings must have different names",
                ));
            }
            size = Some((width, height));
        } else if let Some(value) = part.strip_prefix("width=") {
            if width.is_some() {
                return Err(error("E092", line, "responsive repeats `width=`"));
            }
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            if height.is_some() {
                return Err(error("E092", line, "responsive repeats `height=`"));
            }
            height = Some(parse_length(value, line)?);
        } else {
            return Err(error(
                "E092",
                line,
                format!("unknown responsive property `{part}`"),
            ));
        }
    }
    let content = match (breakpoint, size) {
        (Some(_), Some(_)) => {
            return Err(error(
                "E092",
                line,
                "responsive accepts either `at=` or `size=`, not both",
            ));
        }
        (Some(breakpoint), None) => {
            if line.children.len() != 2 {
                return Err(error(
                    "E092",
                    line,
                    "responsive with `at=` requires two children: narrow, then wide",
                ));
            }
            ResponsiveContent::Breakpoint {
                breakpoint,
                narrow: Box::new(parse_view(&line.children[0])?),
                wide: Box::new(parse_view(&line.children[1])?),
            }
        }
        (None, Some((width, height))) => {
            if line.children.len() != 1 {
                return Err(error(
                    "E092",
                    line,
                    "responsive with `size=` requires exactly one child",
                ));
            }
            ResponsiveContent::Size {
                width,
                height,
                content: Box::new(parse_view(&line.children[0])?),
            }
        }
        (None, None) => {
            return Err(error(
                "E092",
                line,
                "responsive requires `at=` or `size=(width, height)`",
            ));
        }
    };
    Ok(ViewNode::Responsive {
        content,
        width,
        height,
        span: Span::line(line.number),
    })
}

fn parse_size_route(source: &str, line: &Line) -> Result<Route, Error> {
    parse_payload_route(source, line, 2)
}

fn parse_payload_route(source: &str, line: &Line, count: usize) -> Result<Route, Error> {
    let mut route = parse_route(source, line)?;
    if route.args.is_empty() {
        route.args = vec![RouteArg::Payload; count];
    }
    Ok(route)
}

fn parse_combo_box(
    parts: &[String],
    styles: Vec<String>,
    route_source: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E088",
            line,
            "combo uses typed properties instead of `@` utilities",
        ));
    }
    if parts.len() < 4 {
        return Err(error(
            "E088",
            line,
            "combo expects `combo state selected \"Placeholder\" -> handler _`",
        ));
    }
    let route = route_source.ok_or_else(|| error("E088", line, "combo requires `-> handler _`"))?;
    let mut options = ComboBoxOptions::default();
    for part in &parts[4..] {
        if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("menu-height=") {
            options.menu_height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("padding=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text-size=") {
            options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            options.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shaping=") {
            options.shaping = Some(parse_text_shaping(value, line, "E088")?);
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("input=") {
            let mut route = parse_route(value, line)?;
            if route.args.is_empty() {
                route.args.push(RouteArg::Payload);
            }
            options.input = Some(route);
        } else if let Some(value) = part.strip_prefix("hover=") {
            let mut route = parse_route(value, line)?;
            if route.args.is_empty() {
                route.args.push(RouteArg::Payload);
            }
            options.hover = Some(route);
        } else if let Some(value) = part.strip_prefix("open=") {
            options.open = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("close=") {
            options.close = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E088", line, "combo style must be a declared style call"))?;
            options.custom_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("menu-style=") {
            let (function, args) = parse_signature(value, line).map_err(|_| {
                error(
                    "E088",
                    line,
                    "combo menu style must be a declared style call",
                )
            })?;
            options.custom_menu_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else {
            return Err(error(
                "E088",
                line,
                format!("unknown combo property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_combo_box_child(child, &mut options)?;
    }
    Ok(ViewNode::ComboBox {
        state: identifier(&parts[1], line)?,
        selected: parse_expr(&parts[2], line)?,
        placeholder: string_literal(&parts[3], line)?,
        options,
        route: parse_route(route.trim(), line)?,
        span: Span::line(line.number),
    })
}

fn parse_combo_box_child(line: &Line, options: &mut ComboBoxOptions) -> Result<(), Error> {
    let parts = split_words(&line.text);
    match parts.first().map(String::as_str) {
        Some("active" | "hovered" | "focused" | "focused-hovered" | "disabled") => {
            ensure_leaf(line)?;
            parse_text_input_status(&parts, line, &mut options.style, "E088", "combo", true)
        }
        Some("menu") => {
            ensure_leaf(line)?;
            if options.menu_style.is_some() {
                return Err(error("E088", line, "duplicate combo menu style"));
            }
            options.menu_style = Some(Box::new(parse_menu_style(&parts, line, "E088", "combo")?));
            Ok(())
        }
        Some("icon") => {
            ensure_leaf(line)?;
            if options.icon.is_some() {
                return Err(error("E088", line, "duplicate combo icon"));
            }
            options.icon = Some(parse_text_input_icon(&parts[1..], line, "E088", "combo")?);
            Ok(())
        }
        _ => Err(error(
            "E088",
            line,
            "combo blocks use active, hovered, focused, focused-hovered, disabled, menu, or icon",
        )),
    }
}

fn parse_text_input_status(
    parts: &[String],
    line: &Line,
    styles: &mut TextInputStyleSet,
    code: &'static str,
    widget: &str,
    supports_icon: bool,
) -> Result<(), Error> {
    let status = parts.first().expect("text input status line");
    let slot = match status.as_str() {
        "active" => &mut styles.active,
        "hovered" => &mut styles.hovered,
        "focused" => &mut styles.focused,
        "focused-hovered" => &mut styles.focused_hovered,
        "disabled" => &mut styles.disabled,
        _ => unreachable!("text input status dispatch validates the status"),
    };
    if slot.is_some() {
        return Err(error(
            code,
            line,
            format!("duplicate {widget} {status} style"),
        ));
    }
    let mut style = TextInputStatusStyle {
        span: Some(Span::line(line.number)),
        ..TextInputStatusStyle::default()
    };
    for part in &parts[1..] {
        if supports_icon && let Some(value) = part.strip_prefix("icon=") {
            style.icon_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("placeholder=") {
            style.placeholder_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("value=") {
            style.value_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("selection=") {
            style.selection_color = Some(value.to_owned());
        } else if parse_container_style_option(part, &mut style.options, line)? {
            if style.options.text_color.is_some()
                || style.options.shadow_color.is_some()
                || style.options.shadow_x.is_some()
                || style.options.shadow_y.is_some()
                || style.options.shadow_blur.is_some()
                || style.options.pixel_snap.is_some()
            {
                return Err(error(
                    code,
                    line,
                    format!("unknown {widget} style property `{part}`"),
                ));
            }
        } else {
            return Err(error(
                code,
                line,
                format!("unknown {widget} style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

fn parse_text_input_icon(
    parts: &[String],
    line: &Line,
    code: &'static str,
    widget: &str,
) -> Result<TextInputIcon, Error> {
    let mut code_point = None;
    let mut font = None;
    let mut size = None;
    let mut spacing = None;
    let mut side = IconSide::Left;
    for part in parts {
        if let Some(value) = part.strip_prefix("code=") {
            let value = string_literal(value, line)?;
            let mut chars = value.chars();
            code_point = chars.next();
            if code_point.is_none() || chars.next().is_some() {
                return Err(error(
                    code,
                    line,
                    format!("{widget} icon code must contain one character"),
                ));
            }
        } else if let Some(value) = part.strip_prefix("font=") {
            font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("spacing=") {
            spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("side=") {
            side = match value {
                "left" => IconSide::Left,
                "right" => IconSide::Right,
                _ => {
                    return Err(error(
                        code,
                        line,
                        format!("{widget} icon side must be left or right"),
                    ));
                }
            };
        } else {
            return Err(error(
                code,
                line,
                format!("unknown {widget} icon property `{part}`"),
            ));
        }
    }
    Ok(TextInputIcon {
        code_point: code_point
            .ok_or_else(|| error(code, line, format!("{widget} icon requires code=\"…\"")))?,
        font,
        size,
        spacing,
        side,
        span: Span::line(line.number),
    })
}

fn parse_pick_list(
    parts: &[String],
    styles: Vec<String>,
    route_source: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E087",
            line,
            "pick uses typed properties instead of `@` utilities",
        ));
    }
    if parts.len() < 3 {
        return Err(error(
            "E087",
            line,
            "pick expects `pick options selected -> handler _`",
        ));
    }
    let route = route_source.ok_or_else(|| error("E087", line, "pick requires `-> handler _`"))?;
    let mut config = PickListOptions::default();
    for part in &parts[3..] {
        if let Some(value) = part.strip_prefix("placeholder=") {
            config.placeholder = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("width=") {
            config.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("menu-height=") {
            config.menu_height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("padding=") {
            config.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text-size=") {
            config.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            config.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shaping=") {
            config.shaping = Some(parse_text_shaping(value, line, "E087")?);
        } else if let Some(value) = part.strip_prefix("font=") {
            config.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("open=") {
            config.open = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("close=") {
            config.close = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E087", line, "pick style must be a declared style call"))?;
            config.custom_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("menu-style=") {
            let (function, args) = parse_signature(value, line).map_err(|_| {
                error(
                    "E087",
                    line,
                    "pick menu style must be a declared style call",
                )
            })?;
            config.custom_menu_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown pick property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_pick_list_child(child, &mut config)?;
    }
    Ok(ViewNode::PickList {
        options: parse_expr(&parts[1], line)?,
        selected: parse_expr(&parts[2], line)?,
        options_config: config,
        route: parse_route(route.trim(), line)?,
        span: Span::line(line.number),
    })
}

fn parse_pick_list_child(line: &Line, options: &mut PickListOptions) -> Result<(), Error> {
    let parts = split_words(&line.text);
    match parts.first().map(String::as_str) {
        Some("active" | "hovered" | "opened" | "opened-hovered") => {
            ensure_leaf(line)?;
            parse_pick_list_status(&parts, line, &mut options.style)
        }
        Some("menu") => {
            ensure_leaf(line)?;
            if options.menu_style.is_some() {
                return Err(error("E087", line, "duplicate pick menu style"));
            }
            options.menu_style = Some(Box::new(parse_menu_style(&parts, line, "E087", "pick")?));
            Ok(())
        }
        Some("handle") => {
            if options.handle.is_some() {
                return Err(error("E087", line, "duplicate pick handle"));
            }
            options.handle = Some(parse_pick_list_handle(&parts, line)?);
            Ok(())
        }
        _ => Err(error(
            "E087",
            line,
            "pick blocks use active, hovered, opened, opened-hovered, menu, or handle",
        )),
    }
}

fn parse_pick_list_status(
    parts: &[String],
    line: &Line,
    styles: &mut PickListStyleSet,
) -> Result<(), Error> {
    let status = parts.first().expect("pick status line");
    let slot = match status.as_str() {
        "active" => &mut styles.active,
        "hovered" => &mut styles.hovered,
        "opened" => &mut styles.opened,
        "opened-hovered" => &mut styles.opened_hovered,
        _ => unreachable!("pick status dispatch validates the status"),
    };
    if slot.is_some() {
        return Err(error(
            "E087",
            line,
            format!("duplicate pick {status} style"),
        ));
    }
    let mut style = PickListStatusStyle {
        span: Some(Span::line(line.number)),
        ..PickListStatusStyle::default()
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("placeholder=") {
            style.placeholder_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("handle=") {
            style.handle_color = Some(value.to_owned());
        } else if parse_container_style_option(part, &mut style.options, line)? {
            if style.options.shadow_color.is_some()
                || style.options.shadow_x.is_some()
                || style.options.shadow_y.is_some()
                || style.options.shadow_blur.is_some()
                || style.options.pixel_snap.is_some()
            {
                return Err(error(
                    "E087",
                    line,
                    format!("unknown pick status property `{part}`"),
                ));
            }
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown pick status property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

fn parse_menu_style(
    parts: &[String],
    line: &Line,
    code: &'static str,
    widget: &str,
) -> Result<MenuStyleOptions, Error> {
    let mut style = MenuStyleOptions {
        span: Some(Span::line(line.number)),
        ..MenuStyleOptions::default()
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("selected-text=") {
            style.selected_text_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("selected-background=") {
            style.selected_background = Some(parse_background_value(value, line)?);
        } else if parse_container_style_option(part, &mut style.options, line)? {
            if style.options.pixel_snap.is_some() {
                return Err(error(
                    code,
                    line,
                    format!("{widget} menu does not support pixel-snap"),
                ));
            }
        } else {
            return Err(error(
                code,
                line,
                format!("unknown {widget} menu property `{part}`"),
            ));
        }
    }
    Ok(style)
}

fn parse_pick_list_handle(parts: &[String], line: &Line) -> Result<PickListHandle, Error> {
    let kind = parts.get(1).map(String::as_str).ok_or_else(|| {
        error(
            "E087",
            line,
            "pick handle uses arrow, static, dynamic, or none",
        )
    })?;
    match kind {
        "arrow" => {
            ensure_leaf(line)?;
            let mut size = None;
            for part in &parts[2..] {
                if let Some(value) = part.strip_prefix("size=") {
                    size = Some(parse_expr(strip_wrapping_parens(value), line)?);
                } else {
                    return Err(error(
                        "E087",
                        line,
                        format!("unknown arrow handle property `{part}`"),
                    ));
                }
            }
            Ok(PickListHandle::Arrow { size })
        }
        "static" => {
            ensure_leaf(line)?;
            Ok(PickListHandle::Static(parse_pick_list_icon(
                &parts[2..],
                line,
            )?))
        }
        "dynamic" => {
            if parts.len() != 2
                || line.children.len() != 2
                || line.children[0].text.split_ascii_whitespace().next() != Some("closed")
                || line.children[1].text.split_ascii_whitespace().next() != Some("open")
            {
                return Err(error(
                    "E087",
                    line,
                    "dynamic pick handle requires closed then open icon lines",
                ));
            }
            let closed = split_words(&line.children[0].text);
            let open = split_words(&line.children[1].text);
            ensure_leaf(&line.children[0])?;
            ensure_leaf(&line.children[1])?;
            Ok(PickListHandle::Dynamic {
                closed: parse_pick_list_icon(&closed[1..], &line.children[0])?,
                open: parse_pick_list_icon(&open[1..], &line.children[1])?,
            })
        }
        "none" => {
            ensure_leaf(line)?;
            if parts.len() != 2 {
                return Err(error("E087", line, "none handle has no properties"));
            }
            Ok(PickListHandle::None)
        }
        _ => Err(error(
            "E087",
            line,
            "pick handle uses arrow, static, dynamic, or none",
        )),
    }
}

fn parse_pick_list_icon(parts: &[String], line: &Line) -> Result<PickListIcon, Error> {
    let mut code_point = None;
    let mut font = None;
    let mut size = None;
    let mut line_height = None;
    let mut shaping = None;
    for part in parts {
        if let Some(value) = part.strip_prefix("code=") {
            let value = string_literal(value, line)?;
            let mut chars = value.chars();
            code_point = chars.next();
            if code_point.is_none() || chars.next().is_some() {
                return Err(error(
                    "E087",
                    line,
                    "pick handle code must contain one character",
                ));
            }
        } else if let Some(value) = part.strip_prefix("font=") {
            font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shaping=") {
            shaping = Some(parse_text_shaping(value, line, "E087")?);
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown pick handle icon property `{part}`"),
            ));
        }
    }
    Ok(PickListIcon {
        code_point: code_point.ok_or_else(|| {
            error(
                "E087",
                line,
                "static and dynamic pick handles require code=\"…\"",
            )
        })?,
        font,
        size,
        line_height,
        shaping,
        span: Span::line(line.number),
    })
}

fn parse_media(
    kind: &str,
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error(
            "E085",
            line,
            "media uses typed properties instead of `@` utilities",
        ));
    }
    let source = parts
        .get(1)
        .ok_or_else(|| error("E085", line, format!("{kind} requires a source expression")))?;
    let media_kind = match kind {
        "image" => MediaKind::Image,
        "svg" => MediaKind::Svg,
        "viewer" => MediaKind::Viewer,
        _ => unreachable!(),
    };
    let mut options = MediaOptions::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("fit=") {
            options.fit = Some(match value {
                "contain" => ContentFit::Contain,
                "cover" => ContentFit::Cover,
                "fill" => ContentFit::Fill,
                "none" => ContentFit::None,
                "scale-down" => ContentFit::ScaleDown,
                _ => {
                    return Err(error(
                        "E085",
                        line,
                        "fit must be contain, cover, fill, none, or scale-down",
                    ));
                }
            });
        } else if let Some(value) = part.strip_prefix("rotation=") {
            if media_kind == MediaKind::Viewer {
                return Err(error("E085", line, "rotation is not available on viewer"));
            }
            let (value, solid) = value
                .strip_prefix("solid(")
                .and_then(|value| value.strip_suffix(')'))
                .map_or((value, false), |value| (value, true));
            options.rotation = Some(parse_expr(strip_wrapping_parens(value), line)?);
            options.rotation_solid = solid;
        } else if let Some(value) = part.strip_prefix("opacity=") {
            if media_kind == MediaKind::Viewer {
                return Err(error("E085", line, "opacity is not available on viewer"));
            }
            options.opacity = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "memory" {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "memory is only available on svg"));
            }
            options.svg_memory = true;
        } else if let Some(value) = part.strip_prefix("color=") {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "color is only available on svg"));
            }
            options.svg_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("hover=") {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "hover is only available on svg"));
            }
            options.svg_hover_color = Some((value != "none").then(|| value.to_owned()));
        } else if let Some(value) = part.strip_prefix("style=") {
            if media_kind != MediaKind::Svg {
                return Err(error("E085", line, "style is only available on svg"));
            }
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E085", line, "svg style must be a declared style call"))?;
            options.svg_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("filter=") {
            if media_kind == MediaKind::Svg {
                return Err(error(
                    "E085",
                    line,
                    "filter is only available on image and viewer",
                ));
            }
            options.filter = Some(match value {
                "linear" => ImageFilter::Linear,
                "nearest" => ImageFilter::Nearest,
                _ => return Err(error("E085", line, "filter must be linear or nearest")),
            });
        } else if let Some(value) = part.strip_prefix("scale=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "scale is only available on image"));
            }
            options.scale = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("expand=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "expand is only available on image"));
            }
            options.expand = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "radius is only available on image"));
            }
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some((field, value)) = [
            ("radius-tl=", &mut options.radius_top_left),
            ("radius-tr=", &mut options.radius_top_right),
            ("radius-br=", &mut options.radius_bottom_right),
            ("radius-bl=", &mut options.radius_bottom_left),
        ]
        .into_iter()
        .find_map(|(prefix, field)| part.strip_prefix(prefix).map(|value| (field, value)))
        {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "radius is only available on image"));
            }
            *field = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("crop=") {
            if media_kind != MediaKind::Image {
                return Err(error("E085", line, "crop is only available on image"));
            }
            options.crop = Some(
                parse_expr_list(strip_wrapping_parens(value), line)?
                    .try_into()
                    .map_err(|_| error("E085", line, "crop requires x, y, width, and height"))?,
            );
        } else if let Some((property, field, value)) = [
            ("padding=", &mut options.padding),
            ("min-scale=", &mut options.min_scale),
            ("max-scale=", &mut options.max_scale),
            ("scale-step=", &mut options.scale_step),
        ]
        .into_iter()
        .find_map(|(property, field)| {
            part.strip_prefix(property)
                .map(|value| (property, field, value))
        }) {
            if media_kind != MediaKind::Viewer {
                return Err(error(
                    "E085",
                    line,
                    format!(
                        "{} is only available on viewer",
                        property.trim_end_matches('=')
                    ),
                ));
            }
            *field = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E085",
                line,
                format!("unknown {kind} property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Media {
        kind: media_kind,
        source: parse_expr(source, line)?,
        options,
        span: Span::line(line.number),
    })
}

fn parse_length(source: &str, line: &Line) -> Result<LengthValue, Error> {
    Ok(match source {
        "fill" => LengthValue::Fill,
        "shrink" => LengthValue::Shrink,
        source => {
            if let Some(value) = source
                .strip_prefix("fill(")
                .and_then(|value| value.strip_suffix(')'))
            {
                LengthValue::FillPortion(value.parse().map_err(|_| {
                    error(
                        "E074",
                        line,
                        "fill portion must be an integer from 0 to 65535",
                    )
                })?)
            } else {
                LengthValue::Fixed(parse_expr(strip_wrapping_parens(source), line)?)
            }
        }
    })
}

fn parse_tooltip(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error(
            "E086",
            line,
            "tooltip content owns its styling; the wrapper does not accept `@`",
        ));
    }
    if line.children.len() != 2 {
        return Err(error(
            "E086",
            line,
            "tooltip requires exactly two children: content, then tip",
        ));
    }
    let mut options = TooltipOptions {
        position: TooltipPosition::Top,
        gap: Expr::F64(0.0),
        padding: Expr::F64(5.0),
        delay_ms: Expr::I64(0),
        snap: Expr::Bool(true),
        style: None,
        custom_style: None,
        background: None,
        text_color: None,
        border_color: None,
        border_width: None,
        radius: None,
        radius_top_left: None,
        radius_top_right: None,
        radius_bottom_right: None,
        radius_bottom_left: None,
        shadow_color: None,
        shadow_x: None,
        shadow_y: None,
        shadow_blur: None,
        pixel_snap: None,
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("position=") {
            options.position = match value {
                "top" => TooltipPosition::Top,
                "bottom" => TooltipPosition::Bottom,
                "left" => TooltipPosition::Left,
                "right" => TooltipPosition::Right,
                "cursor" => TooltipPosition::FollowCursor,
                _ => {
                    return Err(error(
                        "E086",
                        line,
                        "tooltip position must be top, bottom, left, right, or cursor",
                    ));
                }
            };
        } else if let Some(value) = part.strip_prefix("gap=") {
            options.gap = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("padding=") {
            options.padding = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("delay=") {
            options.delay_ms = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("snap=") {
            options.snap = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("style=") {
            options.custom_style = None;
            options.style = match value {
                "transparent" => Some(TooltipStyle::Transparent),
                "rounded" => Some(TooltipStyle::Rounded),
                "bordered" => Some(TooltipStyle::Bordered),
                "dark" => Some(TooltipStyle::Dark),
                "primary" => Some(TooltipStyle::Primary),
                "secondary" => Some(TooltipStyle::Secondary),
                "success" => Some(TooltipStyle::Success),
                "warning" => Some(TooltipStyle::Warning),
                "danger" => Some(TooltipStyle::Danger),
                _ => {
                    let (function, args) = parse_signature(value, line).map_err(|_| {
                        error(
                            "E086",
                            line,
                            "tooltip style must be a preset or declared container style call",
                        )
                    })?;
                    options.custom_style = Some(ExternCall {
                        function,
                        args: parse_expr_list(&args, line)?,
                    });
                    None
                }
            };
        } else if let Some(value) = part.strip_prefix("background=") {
            options.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("text=") {
            options.text_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border=") {
            options.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-width=") {
            options.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius=") {
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tl=") {
            options.radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tr=") {
            options.radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-br=") {
            options.radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-bl=") {
            options.radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shadow=") {
            options.shadow_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("shadow-x=") {
            options.shadow_x = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shadow-y=") {
            options.shadow_y = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shadow-blur=") {
            options.shadow_blur = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("pixel-snap=") {
            options.pixel_snap = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E086",
                line,
                format!("unknown tooltip property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Tooltip {
        options,
        content: Box::new(parse_view(&line.children[0])?),
        tip: Box::new(parse_view(&line.children[1])?),
        span: Span::line(line.number),
    })
}

fn parse_mouse_area(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E087", line, "mouse does not accept `@` utilities"));
    }
    if line.children.len() != 1 {
        return Err(error("E087", line, "mouse requires exactly one child"));
    }
    let mut options = MouseAreaOptions::default();
    for part in &parts[1..] {
        let route = |value: &str| parse_route(value, line);
        if let Some(value) = part.strip_prefix("press=") {
            options.press = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("release=") {
            options.release = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("double=") {
            options.double_click = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("right_press=") {
            options.right_press = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("right_release=") {
            options.right_release = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("middle_press=") {
            options.middle_press = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("middle_release=") {
            options.middle_release = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("enter=") {
            options.enter = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("move=") {
            options.move_route = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("scroll=") {
            options.scroll = Some(parse_payload_route(value, line, 3)?);
        } else if let Some(value) = part.strip_prefix("exit=") {
            options.exit = Some(route(value)?);
        } else if let Some(value) = part.strip_prefix("cursor=") {
            options.interaction = Some(parse_mouse_interaction(value, line)?);
        } else {
            return Err(error(
                "E087",
                line,
                format!("unknown mouse property `{part}`"),
            ));
        }
    }
    if parts.len() == 1 {
        return Err(error(
            "E087",
            line,
            "mouse needs an event route or cursor property",
        ));
    }
    Ok(ViewNode::MouseArea {
        options,
        content: Box::new(parse_view(&line.children[0])?),
        span: Span::line(line.number),
    })
}

fn parse_mouse_interaction(source: &str, line: &Line) -> Result<MouseInteraction, Error> {
    Ok(match source {
        "none" => MouseInteraction::None,
        "hidden" => MouseInteraction::Hidden,
        "idle" => MouseInteraction::Idle,
        "context-menu" => MouseInteraction::ContextMenu,
        "help" => MouseInteraction::Help,
        "pointer" => MouseInteraction::Pointer,
        "progress" => MouseInteraction::Progress,
        "wait" => MouseInteraction::Wait,
        "cell" => MouseInteraction::Cell,
        "crosshair" => MouseInteraction::Crosshair,
        "text" => MouseInteraction::Text,
        "alias" => MouseInteraction::Alias,
        "copy" => MouseInteraction::Copy,
        "move" => MouseInteraction::Move,
        "no-drop" => MouseInteraction::NoDrop,
        "not-allowed" => MouseInteraction::NotAllowed,
        "grab" => MouseInteraction::Grab,
        "grabbing" => MouseInteraction::Grabbing,
        "resize-horizontal" => MouseInteraction::ResizingHorizontally,
        "resize-vertical" => MouseInteraction::ResizingVertically,
        "resize-diagonal-up" => MouseInteraction::ResizingDiagonallyUp,
        "resize-diagonal-down" => MouseInteraction::ResizingDiagonallyDown,
        "resize-column" => MouseInteraction::ResizingColumn,
        "resize-row" => MouseInteraction::ResizingRow,
        "all-scroll" => MouseInteraction::AllScroll,
        "zoom-in" => MouseInteraction::ZoomIn,
        "zoom-out" => MouseInteraction::ZoomOut,
        _ => return Err(error("E087", line, format!("unknown cursor `{source}`"))),
    })
}

fn parse_canvas(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if !styles.is_empty() {
        return Err(error("E190", line, "canvas does not accept `@` utilities"));
    }
    let mut options = CanvasOptions::default();
    for part in &parts[1..] {
        let expr = |value: &str| parse_expr(strip_wrapping_parens(value), line);
        if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("cache=") {
            options.cache = Some(expr(value)?);
        } else if let Some(value) = part.strip_prefix("cache-group=") {
            options.cache_group = Some(identifier(value, line)?);
        } else if let Some(value) = part.strip_prefix("capture=") {
            options.capture = Some(expr(value)?);
        } else if let Some(value) = part.strip_prefix("press=") {
            options.press = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("release=") {
            options.release = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("right_press=") {
            options.right_press = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("right_release=") {
            options.right_release = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("middle_press=") {
            options.middle_press = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("middle_release=") {
            options.middle_release = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("enter=") {
            options.enter = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("move=") {
            options.move_route = Some(parse_payload_route(value, line, 2)?);
        } else if let Some(value) = part.strip_prefix("scroll=") {
            options.scroll = Some(parse_payload_route(value, line, 3)?);
        } else if let Some(value) = part.strip_prefix("exit=") {
            options.exit = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("cursor=") {
            if options.interaction.is_some() || options.interaction_expr.is_some() {
                return Err(error("E190", line, "duplicate canvas cursor property"));
            }
            if value.starts_with('(') {
                options.interaction_expr = Some(expr(value)?);
            } else {
                options.interaction = Some(parse_mouse_interaction(value, line)?);
            }
        } else if let Some(value) = part.strip_prefix("cursor-outside=") {
            if options.interaction_outside.is_some() {
                return Err(error(
                    "E190",
                    line,
                    "duplicate canvas cursor-outside property",
                ));
            }
            options.interaction_outside = Some(expr(value)?);
        } else {
            return Err(error(
                "E190",
                line,
                format!("unknown canvas property `{part}`"),
            ));
        }
    }
    let mut commands = Vec::new();
    let mut events = Vec::new();
    let mut locals = Vec::new();
    for child in &line.children {
        if child.text == "state" {
            if !locals.is_empty() {
                return Err(error("E190", child, "canvas may only have one state block"));
            }
            locals = child
                .children
                .iter()
                .map(parse_state)
                .collect::<Result<_, _>>()?;
            if locals.is_empty() {
                return Err(error("E190", child, "canvas state cannot be empty"));
            }
        } else if child.text.starts_with("event ")
            || child.text.starts_with("capture ")
            || child.text.starts_with("redraw ")
        {
            events.push(parse_canvas_event(child)?);
        } else {
            commands.push(parse_canvas_command(child)?);
        }
    }
    Ok(ViewNode::Canvas {
        options: Box::new(options),
        locals,
        commands,
        events,
        span: Span::line(line.number),
    })
}

fn parse_canvas_event(line: &Line) -> Result<CanvasEvent, Error> {
    if let Some(source) = line.text.strip_prefix("event ")
        && source.contains(" -> ")
    {
        ensure_leaf(line)?;
        let mut event_line = line.clone();
        event_line.text = source.to_owned();
        let subscription = parse_subscription(&event_line)?;
        if subscription.condition.is_some()
            || subscription.status.is_some()
            || subscription.window_id
        {
            return Err(error(
                "E190",
                line,
                "canvas events do not use subscription `when`, `status`, or `with-id` options",
            ));
        }
        validate_canvas_event_source(&subscription.source, line)?;
        return Ok(CanvasEvent {
            source: subscription.source,
            bindings: Vec::new(),
            updates: Vec::new(),
            action: Some(CanvasEventAction::Route(subscription.route)),
            capture: false,
            route_payload: true,
            span: Span::line(line.number),
        });
    }

    if let Some(header) = line.text.strip_prefix("event ") {
        if line.children.is_empty() {
            return Err(error(
                "E190",
                line,
                "canvas event blocks need indented `set`, `emit`, `redraw`, or `capture` actions",
            ));
        }
        let (source, bindings) = header
            .split_once(" as ")
            .map_or((header, ""), |(source, bindings)| (source, bindings));
        let source = parse_canvas_event_source(source, line)?;
        validate_canvas_event_source(&source, line)?;
        let mut seen_bindings = std::collections::HashSet::new();
        let bindings = bindings
            .split(',')
            .map(str::trim)
            .filter(|binding| !binding.is_empty())
            .map(|binding| {
                let binding = identifier(binding, line)?;
                if !seen_bindings.insert(binding.clone()) {
                    return Err(error(
                        "E190",
                        line,
                        format!("duplicate canvas event binding `{binding}`"),
                    ));
                }
                Ok(binding)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut updates = Vec::new();
        let mut action = None;
        let mut capture = false;
        for child in &line.children {
            ensure_leaf(child)?;
            if let Some(update) = child.text.strip_prefix("set ") {
                let (name, value) = split_top_once(update, '=').ok_or_else(|| {
                    error("E190", child, "canvas state updates use `set name = value`")
                })?;
                updates.push(CanvasStateUpdate {
                    name: identifier(name.trim(), child)?,
                    value: parse_expr(value.trim(), child)?,
                    span: Span::line(child.number),
                });
            } else if let Some(route) = child.text.strip_prefix("emit ") {
                if action.is_some() {
                    return Err(error(
                        "E190",
                        child,
                        "canvas event blocks allow one `emit` or `redraw` action",
                    ));
                }
                action = Some(CanvasEventAction::Route(parse_route(route, child)?));
            } else if child.text == "redraw" || child.text.starts_with("redraw ") {
                if action.is_some() {
                    return Err(error(
                        "E190",
                        child,
                        "canvas event blocks allow one `emit` or `redraw` action",
                    ));
                }
                let after_ms = child
                    .text
                    .strip_prefix("redraw ")
                    .map(|after| {
                        after.strip_prefix("after=").ok_or_else(|| {
                            error(
                                "E190",
                                child,
                                "scheduled canvas redraw uses `redraw after=16ms`",
                            )
                        })
                    })
                    .transpose()?
                    .map(|after| parse_duration(after, child))
                    .transpose()?;
                action = Some(CanvasEventAction::Redraw { after_ms });
            } else if child.text == "capture" {
                if capture {
                    return Err(error("E190", child, "duplicate canvas capture action"));
                }
                capture = true;
            } else {
                return Err(error(
                    "E190",
                    child,
                    "canvas event blocks accept `set`, `emit`, `redraw`, or `capture`",
                ));
            }
        }
        return Ok(CanvasEvent {
            source,
            bindings,
            updates,
            action,
            capture,
            route_payload: false,
            span: Span::line(line.number),
        });
    }

    ensure_leaf(line)?;
    let (source, action, capture) = {
        let (source, redraw) = line
            .text
            .strip_prefix("capture ")
            .map(|source| (source, false))
            .or_else(|| {
                line.text
                    .strip_prefix("redraw ")
                    .map(|source| (source, true))
            })
            .expect("canvas event prefix checked by caller");
        let mut parts = split_words(source);
        let after_ms = if redraw && parts.len() == 3 {
            let after = parts
                .pop()
                .and_then(|part| part.strip_prefix("after=").map(str::to_owned))
                .ok_or_else(|| {
                    error(
                        "E190",
                        line,
                        "scheduled canvas redraw uses `after=16ms` or `after=1s`",
                    )
                })?;
            Some(parse_duration(&after, line)?)
        } else {
            None
        };
        if parts.len() != 2 {
            return Err(error(
                "E190",
                line,
                "canvas capture/redraw requires an event family and kind",
            ));
        }
        let source = parse_canvas_event_source(&parts.join(" "), line)?;
        let action = if redraw {
            Some(CanvasEventAction::Redraw { after_ms })
        } else {
            None
        };
        (source, action, !redraw)
    };
    validate_canvas_event_source(&source, line)?;
    Ok(CanvasEvent {
        source,
        bindings: Vec::new(),
        updates: Vec::new(),
        action,
        capture,
        route_payload: false,
        span: Span::line(line.number),
    })
}

fn validate_canvas_event_source(source: &SubscriptionSource, line: &Line) -> Result<(), Error> {
    if !matches!(
        source,
        SubscriptionSource::InputMethod(_)
            | SubscriptionSource::Keyboard(_)
            | SubscriptionSource::Mouse(_)
            | SubscriptionSource::Touch(_)
            | SubscriptionSource::Window(_)
    ) {
        return Err(error(
            "E190",
            line,
            "canvas events accept input-method, keyboard, mouse, touch, or window sources",
        ));
    }
    Ok(())
}

fn parse_canvas_event_source(source: &str, line: &Line) -> Result<SubscriptionSource, Error> {
    let mut event_line = line.clone();
    event_line.text = format!("{source} -> __canvas_event");
    event_line.children.clear();
    let subscription = parse_subscription(&event_line)?;
    if subscription.window_id {
        return Err(error(
            "E190",
            line,
            "canvas window events do not use `with-id`",
        ));
    }
    Ok(subscription.source)
}

fn parse_canvas_commands(lines: &[Line]) -> Result<Vec<CanvasCommand>, Error> {
    lines.iter().map(parse_canvas_command).collect()
}

fn parse_canvas_command(line: &Line) -> Result<CanvasCommand, Error> {
    if let Some(condition) = line.text.strip_prefix("if ") {
        return Ok(CanvasCommand::If {
            condition: parse_expr(condition, line)?,
            commands: parse_canvas_commands(&line.children)?,
            span: Span::line(line.number),
        });
    }
    if let Some(source) = line.text.strip_prefix("for ") {
        let (item, items) = source
            .split_once(" in ")
            .ok_or_else(|| error("E190", line, "canvas loops use `for item in items`"))?;
        return Ok(CanvasCommand::For {
            item: identifier(item.trim(), line)?,
            items: parse_expr(items.trim(), line)?,
            commands: parse_canvas_commands(&line.children)?,
            span: Span::line(line.number),
        });
    }
    let (core, styles) = split_style_utilities(&line.text);
    if !styles.is_empty() {
        return Err(error(
            "E190",
            line,
            "canvas drawing commands do not accept `@` utilities",
        ));
    }
    let parts = split_words(core);
    let kind = parts
        .first()
        .map(String::as_str)
        .ok_or_else(|| error("E190", line, "empty canvas command"))?;
    let span = Span::line(line.number);
    match kind {
        "rect" => {
            ensure_leaf(line)?;
            let fields = canvas_fields(
                &parts[1..],
                &[
                    "x",
                    "y",
                    "width",
                    "height",
                    "radius",
                    "radius-tl",
                    "radius-tr",
                    "radius-br",
                    "radius-bl",
                    "fill",
                    "fill-rule",
                    "stroke",
                    "stroke-width",
                    "cap",
                    "join",
                    "dash",
                    "dash-offset",
                ],
                line,
            )?;
            let paint = parse_canvas_paint(&fields, line)?;
            require_canvas_paint(&paint, line)?;
            Ok(CanvasCommand::Rectangle {
                x: canvas_required_expr(&fields, "x", line)?,
                y: canvas_required_expr(&fields, "y", line)?,
                width: canvas_required_expr(&fields, "width", line)?,
                height: canvas_required_expr(&fields, "height", line)?,
                radius: Box::new(parse_canvas_radius(&fields, line)?),
                paint: Box::new(paint),
                span,
            })
        }
        "circle" => {
            ensure_leaf(line)?;
            let fields = canvas_fields(
                &parts[1..],
                &[
                    "x",
                    "y",
                    "radius",
                    "fill",
                    "fill-rule",
                    "stroke",
                    "stroke-width",
                    "cap",
                    "join",
                    "dash",
                    "dash-offset",
                ],
                line,
            )?;
            let paint = parse_canvas_paint(&fields, line)?;
            require_canvas_paint(&paint, line)?;
            Ok(CanvasCommand::Circle {
                x: canvas_required_expr(&fields, "x", line)?,
                y: canvas_required_expr(&fields, "y", line)?,
                radius: canvas_required_expr(&fields, "radius", line)?,
                paint: Box::new(paint),
                span,
            })
        }
        "line" => {
            ensure_leaf(line)?;
            let fields = canvas_fields(
                &parts[1..],
                &[
                    "x1",
                    "y1",
                    "x2",
                    "y2",
                    "stroke",
                    "stroke-width",
                    "cap",
                    "join",
                    "dash",
                    "dash-offset",
                ],
                line,
            )?;
            Ok(CanvasCommand::Line {
                x1: canvas_required_expr(&fields, "x1", line)?,
                y1: canvas_required_expr(&fields, "y1", line)?,
                x2: canvas_required_expr(&fields, "x2", line)?,
                y2: canvas_required_expr(&fields, "y2", line)?,
                stroke: Box::new(
                    parse_canvas_stroke(&fields, line)?.ok_or_else(|| {
                        error("E190", line, "canvas line requires `stroke=color`")
                    })?,
                ),
                span,
            })
        }
        "text" => parse_canvas_text(&parts, line),
        "image" => {
            ensure_leaf(line)?;
            let source = parts
                .get(1)
                .ok_or_else(|| error("E190", line, "canvas image requires a source"))?;
            let fields = canvas_fields(
                &parts[2..],
                &[
                    "x",
                    "y",
                    "width",
                    "height",
                    "filter",
                    "rotation",
                    "opacity",
                    "snap",
                    "radius",
                    "radius-tl",
                    "radius-tr",
                    "radius-br",
                    "radius-bl",
                ],
                line,
            )?;
            let filter = match fields.get("filter").map(String::as_str) {
                None | Some("linear") => ImageFilter::Linear,
                Some("nearest") => ImageFilter::Nearest,
                Some(_) => {
                    return Err(error(
                        "E190",
                        line,
                        "canvas image filter must be linear or nearest",
                    ));
                }
            };
            Ok(CanvasCommand::Image {
                source: parse_expr(source, line)?,
                x: canvas_required_expr(&fields, "x", line)?,
                y: canvas_required_expr(&fields, "y", line)?,
                width: canvas_required_expr(&fields, "width", line)?,
                height: canvas_required_expr(&fields, "height", line)?,
                filter,
                rotation: fields.get("rotation").map_or_else(
                    || Ok(Expr::F64(0.0)),
                    |value| parse_expr(strip_wrapping_parens(value), line),
                )?,
                opacity: fields.get("opacity").map_or_else(
                    || Ok(Expr::F64(1.0)),
                    |value| parse_expr(strip_wrapping_parens(value), line),
                )?,
                snap: fields.get("snap").map_or_else(
                    || Ok(Expr::Bool(false)),
                    |value| parse_expr(strip_wrapping_parens(value), line),
                )?,
                radius: Box::new(parse_canvas_radius(&fields, line)?),
                span,
            })
        }
        "svg" => {
            ensure_leaf(line)?;
            let source = parts
                .get(1)
                .ok_or_else(|| error("E190", line, "canvas svg requires a source"))?;
            let memory_count = parts[2..]
                .iter()
                .filter(|part| part.as_str() == "memory")
                .count();
            if memory_count > 1 {
                return Err(error("E190", line, "duplicate canvas svg `memory` flag"));
            }
            let properties = parts[2..]
                .iter()
                .filter(|part| part.as_str() != "memory")
                .cloned()
                .collect::<Vec<_>>();
            let fields = canvas_fields(
                &properties,
                &["x", "y", "width", "height", "color", "rotation", "opacity"],
                line,
            )?;
            Ok(CanvasCommand::Svg {
                source: parse_expr(source, line)?,
                memory: memory_count == 1,
                x: canvas_required_expr(&fields, "x", line)?,
                y: canvas_required_expr(&fields, "y", line)?,
                width: canvas_required_expr(&fields, "width", line)?,
                height: canvas_required_expr(&fields, "height", line)?,
                color: fields.get("color").cloned(),
                rotation: fields.get("rotation").map_or_else(
                    || Ok(Expr::F64(0.0)),
                    |value| parse_expr(strip_wrapping_parens(value), line),
                )?,
                opacity: fields.get("opacity").map_or_else(
                    || Ok(Expr::F64(1.0)),
                    |value| parse_expr(strip_wrapping_parens(value), line),
                )?,
                span,
            })
        }
        "path" => {
            let fields = canvas_fields(
                &parts[1..],
                &[
                    "fill",
                    "fill-rule",
                    "stroke",
                    "stroke-width",
                    "cap",
                    "join",
                    "dash",
                    "dash-offset",
                ],
                line,
            )?;
            if line.children.is_empty() {
                return Err(error("E190", line, "canvas path requires path segments"));
            }
            let paint = parse_canvas_paint(&fields, line)?;
            require_canvas_paint(&paint, line)?;
            Ok(CanvasCommand::Path {
                segments: line
                    .children
                    .iter()
                    .map(parse_canvas_path_segment)
                    .collect::<Result<_, _>>()?,
                paint: Box::new(paint),
                span,
            })
        }
        "group" => {
            let fields = canvas_fields(
                &parts[1..],
                &["x", "y", "rotate", "scale", "scale-x", "scale-y", "clip"],
                line,
            )?;
            let clip = fields
                .get("clip")
                .map(|value| {
                    parse_expr_list(strip_wrapping_parens(value), line)?
                        .try_into()
                        .map_err(|_| error("E190", line, "canvas clip needs x, y, width, height"))
                })
                .transpose()?;
            Ok(CanvasCommand::Group {
                transform: Box::new(CanvasTransform {
                    x: canvas_optional_expr(&fields, "x", line)?,
                    y: canvas_optional_expr(&fields, "y", line)?,
                    rotate: canvas_optional_expr(&fields, "rotate", line)?,
                    scale: canvas_optional_expr(&fields, "scale", line)?,
                    scale_x: canvas_optional_expr(&fields, "scale-x", line)?,
                    scale_y: canvas_optional_expr(&fields, "scale-y", line)?,
                    clip,
                }),
                commands: parse_canvas_commands(&line.children)?,
                span,
            })
        }
        _ => Err(error(
            "E190",
            line,
            format!("unknown canvas command `{kind}`"),
        )),
    }
}

fn parse_canvas_text(parts: &[String], line: &Line) -> Result<CanvasCommand, Error> {
    ensure_leaf(line)?;
    let value = parts
        .get(1)
        .ok_or_else(|| error("E190", line, "canvas text requires a value"))?;
    let fields = canvas_fields(
        &parts[2..],
        &[
            "x",
            "y",
            "max-width",
            "color",
            "size",
            "line-height",
            "line-height-px",
            "font",
            "align-x",
            "align-y",
            "shaping",
        ],
        line,
    )?;
    if fields.contains_key("line-height") && fields.contains_key("line-height-px") {
        return Err(error(
            "E190",
            line,
            "canvas text accepts only one line-height property",
        ));
    }
    let line_height = if let Some(value) = fields.get("line-height") {
        Some(TextLineHeight::Relative(parse_expr(
            strip_wrapping_parens(value),
            line,
        )?))
    } else if let Some(value) = fields.get("line-height-px") {
        Some(TextLineHeight::Absolute(parse_expr(
            strip_wrapping_parens(value),
            line,
        )?))
    } else {
        None
    };
    let align_x = fields
        .get("align-x")
        .map(|value| match value.as_str() {
            "default" => Ok(TextAlignment::Default),
            "left" => Ok(TextAlignment::Left),
            "center" => Ok(TextAlignment::Center),
            "right" => Ok(TextAlignment::Right),
            "justified" => Ok(TextAlignment::Justified),
            _ => Err(error(
                "E190",
                line,
                "unknown canvas text horizontal alignment",
            )),
        })
        .transpose()?;
    let align_y = fields
        .get("align-y")
        .map(|value| match value.as_str() {
            "top" => Ok(VerticalAlignment::Top),
            "center" => Ok(VerticalAlignment::Center),
            "bottom" => Ok(VerticalAlignment::Bottom),
            _ => Err(error(
                "E190",
                line,
                "unknown canvas text vertical alignment",
            )),
        })
        .transpose()?;
    Ok(CanvasCommand::Text {
        value: parse_expr(value, line)?,
        x: canvas_required_expr(&fields, "x", line)?,
        y: canvas_required_expr(&fields, "y", line)?,
        max_width: canvas_optional_expr(&fields, "max-width", line)?,
        color: fields.get("color").cloned(),
        size: canvas_optional_expr(&fields, "size", line)?,
        line_height,
        font: fields
            .get("font")
            .map(|value| parse_font_preset(value, line))
            .transpose()?,
        align_x,
        align_y,
        shaping: fields
            .get("shaping")
            .map(|value| parse_text_shaping(value, line, "E190"))
            .transpose()?,
        span: Span::line(line.number),
    })
}

fn parse_canvas_path_segment(line: &Line) -> Result<CanvasPathSegment, Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let kind = parts
        .first()
        .map(String::as_str)
        .ok_or_else(|| error("E190", line, "empty canvas path segment"))?;
    let allowed = match kind {
        "move" | "line" => &["x", "y"][..],
        "arc" => &["x", "y", "radius", "start", "end"],
        "arc-to" => &["ax", "ay", "bx", "by", "radius"],
        "ellipse" => &["x", "y", "radius-x", "radius-y", "rotation", "start", "end"],
        "bezier" => &["ax", "ay", "bx", "by", "x", "y"],
        "quadratic" => &["cx", "cy", "x", "y"],
        "rect" => &["x", "y", "width", "height"],
        "rounded" => &[
            "x",
            "y",
            "width",
            "height",
            "radius",
            "radius-tl",
            "radius-tr",
            "radius-br",
            "radius-bl",
        ],
        "circle" => &["x", "y", "radius"],
        "close" if parts.len() == 1 => return Ok(CanvasPathSegment::Close),
        _ => {
            return Err(error(
                "E190",
                line,
                format!("unknown canvas path segment `{kind}`"),
            ));
        }
    };
    let fields = canvas_fields(&parts[1..], allowed, line)?;
    if kind == "rounded"
        && !["radius", "radius-tl", "radius-tr", "radius-br", "radius-bl"]
            .iter()
            .any(|name| fields.contains_key(*name))
    {
        return Err(error(
            "E190",
            line,
            "rounded path segment requires a radius",
        ));
    }
    let value = |name| canvas_required_expr(&fields, name, line);
    Ok(match kind {
        "move" => CanvasPathSegment::Move(value("x")?, value("y")?),
        "line" => CanvasPathSegment::Line(value("x")?, value("y")?),
        "arc" => CanvasPathSegment::Arc {
            x: value("x")?,
            y: value("y")?,
            radius: value("radius")?,
            start: value("start")?,
            end: value("end")?,
        },
        "arc-to" => CanvasPathSegment::ArcTo {
            ax: value("ax")?,
            ay: value("ay")?,
            bx: value("bx")?,
            by: value("by")?,
            radius: value("radius")?,
        },
        "ellipse" => CanvasPathSegment::Ellipse {
            x: value("x")?,
            y: value("y")?,
            radius_x: value("radius-x")?,
            radius_y: value("radius-y")?,
            rotation: value("rotation")?,
            start: value("start")?,
            end: value("end")?,
        },
        "bezier" => CanvasPathSegment::Bezier {
            control_ax: value("ax")?,
            control_ay: value("ay")?,
            control_bx: value("bx")?,
            control_by: value("by")?,
            x: value("x")?,
            y: value("y")?,
        },
        "quadratic" => CanvasPathSegment::Quadratic {
            control_x: value("cx")?,
            control_y: value("cy")?,
            x: value("x")?,
            y: value("y")?,
        },
        "rect" => CanvasPathSegment::Rectangle {
            x: value("x")?,
            y: value("y")?,
            width: value("width")?,
            height: value("height")?,
        },
        "rounded" => CanvasPathSegment::RoundedRectangle {
            x: value("x")?,
            y: value("y")?,
            width: value("width")?,
            height: value("height")?,
            radius: parse_canvas_radius(&fields, line)?,
        },
        "circle" => CanvasPathSegment::Circle {
            x: value("x")?,
            y: value("y")?,
            radius: value("radius")?,
        },
        _ => unreachable!("canvas path kind checked above"),
    })
}

fn canvas_fields(
    parts: &[String],
    allowed: &[&str],
    line: &Line,
) -> Result<BTreeMap<String, String>, Error> {
    let mut fields = BTreeMap::new();
    for part in parts {
        let (name, value) = part.split_once('=').ok_or_else(|| {
            error(
                "E190",
                line,
                format!("canvas properties use `name=value`, got `{part}`"),
            )
        })?;
        if !allowed.contains(&name) {
            return Err(error(
                "E190",
                line,
                format!("unknown canvas property `{name}`"),
            ));
        }
        if value.is_empty() || fields.insert(name.to_owned(), value.to_owned()).is_some() {
            return Err(error(
                "E190",
                line,
                format!("invalid or duplicate canvas property `{name}`"),
            ));
        }
    }
    Ok(fields)
}

fn canvas_required_expr(
    fields: &BTreeMap<String, String>,
    name: &str,
    line: &Line,
) -> Result<Expr, Error> {
    fields
        .get(name)
        .ok_or_else(|| error("E190", line, format!("canvas command requires `{name}=`")))
        .and_then(|value| parse_expr(strip_wrapping_parens(value), line))
}

fn canvas_optional_expr(
    fields: &BTreeMap<String, String>,
    name: &str,
    line: &Line,
) -> Result<Option<Expr>, Error> {
    fields
        .get(name)
        .map(|value| parse_expr(strip_wrapping_parens(value), line))
        .transpose()
}

fn parse_canvas_radius(
    fields: &BTreeMap<String, String>,
    line: &Line,
) -> Result<CanvasRadius, Error> {
    Ok(CanvasRadius {
        all: canvas_optional_expr(fields, "radius", line)?,
        top_left: canvas_optional_expr(fields, "radius-tl", line)?,
        top_right: canvas_optional_expr(fields, "radius-tr", line)?,
        bottom_right: canvas_optional_expr(fields, "radius-br", line)?,
        bottom_left: canvas_optional_expr(fields, "radius-bl", line)?,
    })
}

fn parse_canvas_paint(
    fields: &BTreeMap<String, String>,
    line: &Line,
) -> Result<CanvasPaint, Error> {
    let fill_rule = match fields.get("fill-rule").map(String::as_str) {
        None | Some("non-zero") => CanvasFillRule::NonZero,
        Some("even-odd") => CanvasFillRule::EvenOdd,
        Some(_) => {
            return Err(error(
                "E190",
                line,
                "fill-rule must be non-zero or even-odd",
            ));
        }
    };
    Ok(CanvasPaint {
        fill: fields
            .get("fill")
            .map(|value| parse_background_value(value, line))
            .transpose()?,
        fill_rule,
        stroke: parse_canvas_stroke(fields, line)?,
    })
}

fn require_canvas_paint(paint: &CanvasPaint, line: &Line) -> Result<(), Error> {
    if paint.fill.is_none() && paint.stroke.is_none() {
        Err(error(
            "E190",
            line,
            "canvas shape requires `fill=` or `stroke=`",
        ))
    } else {
        Ok(())
    }
}

fn parse_canvas_stroke(
    fields: &BTreeMap<String, String>,
    line: &Line,
) -> Result<Option<CanvasStroke>, Error> {
    let Some(style) = fields.get("stroke") else {
        if ["stroke-width", "cap", "join", "dash", "dash-offset"]
            .iter()
            .any(|name| fields.contains_key(*name))
        {
            return Err(error("E190", line, "stroke options require `stroke=color`"));
        }
        return Ok(None);
    };
    let cap = match fields.get("cap").map(String::as_str) {
        None | Some("butt") => CanvasLineCap::Butt,
        Some("square") => CanvasLineCap::Square,
        Some("round") => CanvasLineCap::Round,
        Some(_) => {
            return Err(error(
                "E190",
                line,
                "canvas line cap must be butt, square, or round",
            ));
        }
    };
    let join = match fields.get("join").map(String::as_str) {
        None | Some("miter") => CanvasLineJoin::Miter,
        Some("round") => CanvasLineJoin::Round,
        Some("bevel") => CanvasLineJoin::Bevel,
        Some(_) => {
            return Err(error(
                "E190",
                line,
                "canvas line join must be miter, round, or bevel",
            ));
        }
    };
    let dash = fields
        .get("dash")
        .map(|value| parse_expr_list(strip_wrapping_parens(value), line))
        .transpose()?
        .unwrap_or_default();
    Ok(Some(CanvasStroke {
        style: parse_background_value(style, line)?,
        width: fields.get("stroke-width").map_or_else(
            || Ok(Expr::F64(1.0)),
            |value| parse_expr(strip_wrapping_parens(value), line),
        )?,
        cap,
        join,
        dash,
        dash_offset: fields.get("dash-offset").map_or_else(
            || Ok(Expr::I64(0)),
            |value| parse_expr(strip_wrapping_parens(value), line),
        )?,
    }))
}

fn parse_extern_component(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error(
            "E083",
            line,
            "extern components own their styling and do not accept `@` utilities",
        ));
    }
    if parts.len() != 2 {
        return Err(error(
            "E083",
            line,
            "extern component uses `extern name(args) -> handler _`",
        ));
    }
    let (function, args) = parse_signature(&parts[1], line)?;
    Ok(ViewNode::ExternComponent {
        function,
        args: parse_expr_list(&args, line)?,
        route: route.map(|route| parse_route(route, line)).transpose()?,
        span: Span::line(line.number),
    })
}

fn parse_shader(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    if !styles.is_empty() {
        return Err(error("E191", line, "shader does not accept `@` utilities"));
    }
    if parts.len() < 2 {
        return Err(error(
            "E191",
            line,
            "shader uses `shader name(args) width=fill height=120.0 -> handler _`",
        ));
    }
    let (function, args) = parse_signature(&parts[1], line)?;
    let mut width = None;
    let mut height = None;
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("width=") {
            if width.is_some() {
                return Err(error("E191", line, "duplicate shader width"));
            }
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            if height.is_some() {
                return Err(error("E191", line, "duplicate shader height"));
            }
            height = Some(parse_length(value, line)?);
        } else {
            return Err(error(
                "E191",
                line,
                format!("unknown shader property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Shader {
        function,
        args: parse_expr_list(&args, line)?,
        width,
        height,
        route: route.map(|route| parse_route(route, line)).transpose()?,
        span: Span::line(line.number),
    })
}

fn parse_layout_options(kind: &str, parts: &[String], line: &Line) -> Result<LayoutOptions, Error> {
    let mut options = LayoutOptions::default();
    let is_flex = matches!(kind, "row" | "col");
    if kind == "scroll" {
        options.scroll = Some(ScrollOptions::default());
    }
    for part in parts {
        if let Some(value) = part.strip_prefix("columns=") {
            if kind != "grid" || options.columns.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.columns = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("clip=") {
            if !(is_flex || kind == "stack") || options.clip.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.clip = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if (is_flex || kind == "stack")
            && let Some(value) = part.strip_prefix("width=")
        {
            if options.width.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.width = Some(parse_length(value, line)?);
        } else if (is_flex || kind == "stack")
            && let Some(value) = part.strip_prefix("height=")
        {
            if options.height.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.height = Some(parse_length(value, line)?);
        } else if kind == "grid"
            && let Some(value) = part.strip_prefix("width=")
        {
            if options.width.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.width = Some(LengthValue::Fixed(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if kind == "grid"
            && let Some(value) = part.strip_prefix("height=")
        {
            if options.grid_height.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.grid_height = Some(parse_grid_sizing(value, line)?);
        } else if kind == "stack"
            && let Some(value) = part.strip_prefix("under=")
        {
            options.under = value.parse().map_err(|_| {
                error(
                    "E074",
                    line,
                    "stack under must be an integer from 0 to 65535",
                )
            })?;
        } else if (is_flex || kind == "grid")
            && let Some(value) = part.strip_prefix("spacing=")
        {
            if options.spacing.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if kind == "grid"
            && let Some(value) = part.strip_prefix("fluid=")
        {
            if options.fluid.is_some() {
                return Err(error(
                    "E074",
                    line,
                    format!("invalid layout property `{part}`"),
                ));
            }
            options.fluid = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("padding=") {
            options.padding.all = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("padding-x=") {
            options.padding.x = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("padding-y=") {
            options.padding.y = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("padding-top=") {
            options.padding.top = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("padding-right=") {
            options.padding.right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("padding-bottom=") {
            options.padding.bottom = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("padding-left=") {
            options.padding.left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if kind == "col"
            && let Some(value) = part.strip_prefix("max-width=")
        {
            options.max_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("align=") {
            options.align = Some(parse_flex_alignment(value, line)?);
        } else if is_flex && part == "wrap" {
            options.wrap = true;
        } else if is_flex && let Some(value) = part.strip_prefix("wrap-spacing=") {
            options.wrap_spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if is_flex && let Some(value) = part.strip_prefix("wrap-align=") {
            options.wrap_align = Some(parse_flex_alignment(value, line)?);
        } else if kind == "scroll" {
            let scroll = options.scroll.as_mut().expect("scroll options");
            if let Some(value) = part.strip_prefix("direction=") {
                scroll.direction = match value {
                    "vertical" => ScrollDirection::Vertical,
                    "horizontal" => ScrollDirection::Horizontal,
                    "both" => ScrollDirection::Both,
                    _ => {
                        return Err(error(
                            "E074",
                            line,
                            "scroll direction must be vertical, horizontal, or both",
                        ));
                    }
                };
            } else if let Some(value) = part.strip_prefix("width=") {
                scroll.width = Some(parse_length(value, line)?);
            } else if let Some(value) = part.strip_prefix("height=") {
                scroll.height = Some(parse_length(value, line)?);
            } else if let Some(value) = part.strip_prefix("bar=") {
                scroll.hidden_bar = match value {
                    "visible" => false,
                    "hidden" => true,
                    _ => return Err(error("E074", line, "scroll bar must be visible or hidden")),
                };
            } else if let Some(value) = part.strip_prefix("bar-width=") {
                scroll.bar_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("bar-margin=") {
                scroll.bar_margin = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("scroller-width=") {
                scroll.scroller_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("bar-spacing=") {
                scroll.bar_spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("anchor-x=") {
                scroll.anchor_x = parse_scroll_anchor(value, line)?;
            } else if let Some(value) = part.strip_prefix("anchor-y=") {
                scroll.anchor_y = parse_scroll_anchor(value, line)?;
            } else if let Some(value) = part.strip_prefix("auto=") {
                scroll.auto_scroll = Some(parse_expr(strip_wrapping_parens(value), line)?);
            } else if let Some(value) = part.strip_prefix("scroll=") {
                scroll.route = Some(parse_payload_route(value, line, 4)?);
            } else if let Some(value) = part.strip_prefix("viewport=") {
                scroll.viewport_route = Some(parse_payload_route(value, line, 14)?);
            } else if let Some(value) = part.strip_prefix("style=") {
                let (function, args) = parse_signature(value, line).map_err(|_| {
                    error("E074", line, "scroll style must be a declared style call")
                })?;
                scroll.custom_style = Some(ExternCall {
                    function,
                    args: parse_expr_list(&args, line)?,
                });
            } else {
                return Err(error(
                    "E074",
                    line,
                    format!("unknown scroll property `{part}`"),
                ));
            }
        } else {
            return Err(error(
                "E074",
                line,
                format!("unknown layout property `{part}`"),
            ));
        }
    }
    if !options.wrap && (options.wrap_spacing.is_some() || options.wrap_align.is_some()) {
        return Err(error(
            "E074",
            line,
            "wrap-spacing and wrap-align require `wrap`",
        ));
    }
    if options.columns.is_some() && options.fluid.is_some() {
        return Err(error(
            "E074",
            line,
            "grid columns and fluid are mutually exclusive",
        ));
    }
    if let Some(scroll) = &options.scroll
        && scroll.route.is_some()
        && scroll.viewport_route.is_some()
    {
        return Err(error(
            "E074",
            line,
            "scroll accepts either scroll= or viewport=, not both",
        ));
    }
    Ok(options)
}

fn parse_grid_sizing(source: &str, line: &Line) -> Result<GridSizing, Error> {
    if let Some(values) = source
        .strip_prefix("aspect(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let values = parse_expr_list(values, line)?;
        return match values.as_slice() {
            [width, height] => Ok(GridSizing::AspectRatio {
                width: width.clone(),
                height: height.clone(),
            }),
            _ => Err(error("E074", line, "grid aspect expects width and height")),
        };
    }
    Ok(GridSizing::EvenlyDistribute(parse_length(source, line)?))
}

fn parse_scroll_status_style(parts: &[String], line: &Line) -> Result<ScrollStatusStyle, Error> {
    let status = match parts.first().map(String::as_str) {
        Some("active") => ScrollStatus::Active,
        Some("hovered") => ScrollStatus::Hovered,
        Some("dragged") => ScrollStatus::Dragged,
        _ => unreachable!("scroll style dispatch validates the status"),
    };
    let mut style = ScrollStatusStyle {
        status,
        horizontal_interaction: None,
        vertical_interaction: None,
        horizontal_disabled: None,
        vertical_disabled: None,
        container: ContainerStyleOptions::default(),
        horizontal_rail: ScrollRailStyle::default(),
        vertical_rail: ScrollRailStyle::default(),
        gap: None,
        auto_scroll: ContainerStyleOptions::default(),
        auto_scroll_icon: None,
        span: Span::line(line.number),
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("horizontal-disabled=") {
            style.horizontal_disabled = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("vertical-disabled=") {
            style.vertical_disabled = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("horizontal-hovered=") {
            if status != ScrollStatus::Hovered {
                return Err(error("E074", line, "horizontal-hovered requires hovered"));
            }
            style.horizontal_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("vertical-hovered=") {
            if status != ScrollStatus::Hovered {
                return Err(error("E074", line, "vertical-hovered requires hovered"));
            }
            style.vertical_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("horizontal-dragged=") {
            if status != ScrollStatus::Dragged {
                return Err(error("E074", line, "horizontal-dragged requires dragged"));
            }
            style.horizontal_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else if let Some(value) = part.strip_prefix("vertical-dragged=") {
            if status != ScrollStatus::Dragged {
                return Err(error("E074", line, "vertical-dragged requires dragged"));
            }
            style.vertical_interaction = Some(parse_scroll_style_bool(value, line)?);
        } else {
            return Err(error(
                "E074",
                line,
                format!("unknown scroll selector `{part}`; put styles in nested sections"),
            ));
        }
    }
    for child in &line.children {
        ensure_leaf(child)?;
        let parts = split_words(&child.text);
        let Some(kind) = parts.first().map(String::as_str) else {
            return Err(error("E074", child, "empty scroll style section"));
        };
        if kind == "gap" {
            let [property] = &parts[1..] else {
                return Err(error("E074", child, "scroll gap uses `gap background=…`"));
            };
            let Some(value) = property.strip_prefix("background=") else {
                return Err(error("E074", child, "scroll gap uses `gap background=…`"));
            };
            parse_scroll_style_property(&mut style, &format!("gap={value}"), child)?;
            continue;
        }
        let prefix = match kind {
            "container" => "container-",
            "horizontal-rail" => "horizontal-rail-",
            "horizontal-scroller" => "horizontal-scroller-",
            "vertical-rail" => "vertical-rail-",
            "vertical-scroller" => "vertical-scroller-",
            "auto" => "auto-",
            _ => {
                return Err(error(
                    "E074",
                    child,
                    format!("unknown scroll style section `{kind}`"),
                ));
            }
        };
        for property in &parts[1..] {
            parse_scroll_style_property(&mut style, &format!("{prefix}{property}"), child)?;
        }
    }
    Ok(style)
}

fn parse_scroll_style_property(
    style: &mut ScrollStatusStyle,
    part: &str,
    line: &Line,
) -> Result<(), Error> {
    if let Some(property) = part.strip_prefix("container-") {
        if !parse_container_style_option(property, &mut style.container, line)? {
            return Err(error(
                "E074",
                line,
                format!("unknown scroll container style property `{part}`"),
            ));
        }
    } else if parse_scroll_surface_property(
        part,
        "horizontal-scroller-",
        &mut style.horizontal_rail.scroller,
        false,
        line,
    )? || parse_scroll_surface_property(
        part,
        "horizontal-rail-",
        &mut style.horizontal_rail.rail,
        false,
        line,
    )? || parse_scroll_surface_property(
        part,
        "vertical-scroller-",
        &mut style.vertical_rail.scroller,
        false,
        line,
    )? || parse_scroll_surface_property(
        part,
        "vertical-rail-",
        &mut style.vertical_rail.rail,
        false,
        line,
    )? {
    } else if let Some(value) = part.strip_prefix("gap=") {
        style.gap = Some(parse_background_value(value, line)?);
    } else if let Some(value) = part.strip_prefix("auto-icon=") {
        style.auto_scroll_icon = Some(value.to_owned());
    } else if parse_scroll_surface_property(part, "auto-", &mut style.auto_scroll, true, line)? {
    } else {
        return Err(error(
            "E074",
            line,
            format!("unknown scroll style property `{part}`"),
        ));
    }
    Ok(())
}

fn parse_scroll_surface_property(
    part: &str,
    prefix: &str,
    style: &mut ContainerStyleOptions,
    allow_shadow: bool,
    line: &Line,
) -> Result<bool, Error> {
    let Some(property) = part.strip_prefix(prefix) else {
        return Ok(false);
    };
    if !parse_container_style_option(property, style, line)? {
        return Ok(false);
    }
    if style.text_color.is_some()
        || style.pixel_snap.is_some()
        || (!allow_shadow
            && (style.shadow_color.is_some()
                || style.shadow_x.is_some()
                || style.shadow_y.is_some()
                || style.shadow_blur.is_some()))
    {
        return Err(error(
            "E074",
            line,
            format!("unsupported scroll style property `{part}`"),
        ));
    }
    Ok(true)
}

fn parse_scroll_style_bool(source: &str, line: &Line) -> Result<bool, Error> {
    match source {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error(
            "E074",
            line,
            "scroll status selectors must be true or false",
        )),
    }
}

fn parse_flex_alignment(source: &str, line: &Line) -> Result<FlexAlignment, Error> {
    match source {
        "start" => Ok(FlexAlignment::Start),
        "center" => Ok(FlexAlignment::Center),
        "end" => Ok(FlexAlignment::End),
        _ => Err(error(
            "E074",
            line,
            "layout alignment must be start, center, or end",
        )),
    }
}

fn parse_scroll_anchor(source: &str, line: &Line) -> Result<ScrollAnchor, Error> {
    match source {
        "start" => Ok(ScrollAnchor::Start),
        "end" => Ok(ScrollAnchor::End),
        _ => Err(error("E074", line, "scroll anchor must be start or end")),
    }
}

fn parse_text(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    let value = parts
        .get(1)
        .ok_or_else(|| error("E063", line, "text expects one expression before `@`"))?;
    let mut options = TextOptions::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            options.line_height = Some(TextLineHeight::Relative(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("line-height-px=") {
            options.line_height = Some(TextLineHeight::Absolute(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            options.align_x = Some(match value {
                "default" => TextAlignment::Default,
                "left" => TextAlignment::Left,
                "center" => TextAlignment::Center,
                "right" => TextAlignment::Right,
                "justified" => TextAlignment::Justified,
                _ => return Err(error("E063", line, "unknown horizontal text alignment")),
            });
        } else if let Some(value) = part.strip_prefix("align-y=") {
            options.align_y = Some(match value {
                "top" => VerticalAlignment::Top,
                "center" => VerticalAlignment::Center,
                "bottom" => VerticalAlignment::Bottom,
                _ => return Err(error("E063", line, "unknown vertical text alignment")),
            });
        } else if let Some(value) = part.strip_prefix("shaping=") {
            options.shaping = Some(parse_text_shaping(value, line, "E063")?);
        } else if let Some(value) = part.strip_prefix("wrapping=") {
            options.wrapping = Some(parse_text_wrapping(value, line, "E063")?);
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E063", line, "text style must be a declared style call"))?;
            options.custom_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else {
            return Err(error(
                "E063",
                line,
                format!("unknown text property `{part}`"),
            ));
        }
    }
    ensure_leaf(line)?;
    Ok(ViewNode::Text {
        value: parse_expr(value, line)?,
        options,
        styles,
        span: Span::line(line.number),
    })
}

fn parse_rich_text(
    parts: &[String],
    styles: Vec<String>,
    route_source: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let mut options = TextOptions::default();
    let mut color = None;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            options.line_height = Some(TextLineHeight::Relative(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("line-height-px=") {
            options.line_height = Some(TextLineHeight::Absolute(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("align-x=") {
            options.align_x = Some(match value {
                "default" => TextAlignment::Default,
                "left" => TextAlignment::Left,
                "center" => TextAlignment::Center,
                "right" => TextAlignment::Right,
                "justified" => TextAlignment::Justified,
                _ => return Err(error("E186", line, "unknown rich text alignment")),
            });
        } else if let Some(value) = part.strip_prefix("align-y=") {
            options.align_y = Some(match value {
                "top" => VerticalAlignment::Top,
                "center" => VerticalAlignment::Center,
                "bottom" => VerticalAlignment::Bottom,
                _ => return Err(error("E186", line, "unknown rich text alignment")),
            });
        } else if let Some(value) = part.strip_prefix("wrapping=") {
            options.wrapping = Some(parse_text_wrapping(value, line, "E186")?);
        } else if let Some(value) = part.strip_prefix("color=") {
            color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line).map_err(|_| {
                error(
                    "E186",
                    line,
                    "rich-text style must be a declared style call",
                )
            })?;
            options.custom_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else {
            return Err(error(
                "E186",
                line,
                format!("unknown rich-text property `{part}`"),
            ));
        }
    }
    let spans = line
        .children
        .iter()
        .map(parse_rich_span)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ViewNode::RichText {
        options,
        color,
        spans,
        styles,
        route: route_source
            .map(|route| parse_route(route, line))
            .transpose()?,
        span: Span::line(line.number),
    })
}

fn parse_rich_span(line: &Line) -> Result<RichSpan, Error> {
    ensure_leaf(line)?;
    let (core, styles) = split_top_marker(&line.text, "@").map_or_else(
        || (line.text.trim(), Vec::new()),
        |(core, styles)| {
            (
                core.trim(),
                styles.split_whitespace().map(str::to_owned).collect(),
            )
        },
    );
    let parts = split_words(core);
    if parts.first().map(String::as_str) != Some("span") {
        return Err(error(
            "E186",
            line,
            "rich-text children must be `span` lines",
        ));
    }
    let value = parts
        .get(1)
        .ok_or_else(|| error("E186", line, "span expects one text expression"))?;
    let mut options = RichSpanOptions::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("size=") {
            options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            options.line_height = Some(TextLineHeight::Relative(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("line-height-px=") {
            options.line_height = Some(TextLineHeight::Absolute(parse_expr(
                strip_wrapping_parens(value),
                line,
            )?));
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("color=") {
            options.color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("link=") {
            options.link = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("background=") {
            options.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("border=") {
            options.border = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-width=") {
            options.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius=") {
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tl=") {
            options.radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tr=") {
            options.radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-br=") {
            options.radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-bl=") {
            options.radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding=") {
            options.padding.all = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-x=") {
            options.padding.x = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-y=") {
            options.padding.y = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-top=") {
            options.padding.top = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-right=") {
            options.padding.right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-bottom=") {
            options.padding.bottom = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("padding-left=") {
            options.padding.left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "underline" {
            options.underline = Some(Expr::Bool(true));
        } else if let Some(value) = part.strip_prefix("underline=") {
            options.underline = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "strike" {
            options.strikethrough = Some(Expr::Bool(true));
        } else if let Some(value) = part.strip_prefix("strike=") {
            options.strikethrough = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E186",
                line,
                format!("unknown span property `{part}`"),
            ));
        }
    }
    Ok(RichSpan {
        value: parse_expr(value, line)?,
        options,
        styles,
        span: Span::line(line.number),
    })
}

fn parse_input(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    if parts.len() < 4 {
        return Err(error(
            "E065",
            line,
            "input uses `input \"Label\" #id <-> state`",
        ));
    }
    let label = string_literal(&parts[1], line)?;
    let mut id = None;
    let mut binding = None;
    let mut hint = String::new();
    let mut disabled = None;
    let mut options = InputOptions::default();
    let mut icon_code = None;
    let mut icon_font = None;
    let mut icon_size = None;
    let mut icon_spacing = None;
    let mut icon_side = IconSide::Left;
    let mut index = 2;
    while index < parts.len() {
        let part = &parts[index];
        if part.starts_with('#') {
            id = Some(parse_id(part, line)?);
        } else if part == "<->" {
            index += 1;
            let value = parts
                .get(index)
                .ok_or_else(|| error("E065", line, "missing binding after `<->`"))?;
            binding = Some(identifier(value, line)?);
        } else if let Some(value) = part.strip_prefix("hint=") {
            hint = string_literal(value, line)?;
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("secure=") {
            options.secure = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("submit=") {
            options.submit = Some(parse_route(value, line)?);
        } else if let Some(value) = part.strip_prefix("paste=") {
            options.paste = Some(parse_payload_route(value, line, 1)?);
        } else if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("padding=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text-size=") {
            options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("line-height=") {
            options.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("align=") {
            options.align = Some(match value {
                "left" => InputAlignment::Left,
                "center" => InputAlignment::Center,
                "right" => InputAlignment::Right,
                _ => {
                    return Err(error(
                        "E065",
                        line,
                        "input align must be left, center, or right",
                    ));
                }
            });
        } else if let Some(value) = part.strip_prefix("font=") {
            options.font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E065", line, "input style must be a declared style call"))?;
            options.custom_style = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if let Some(value) = part.strip_prefix("icon=") {
            let value = string_literal(value, line)?;
            let mut chars = value.chars();
            let icon = chars
                .next()
                .ok_or_else(|| error("E065", line, "input icon must contain one character"))?;
            if chars.next().is_some() {
                return Err(error("E065", line, "input icon must contain one character"));
            }
            icon_code = Some(icon);
        } else if let Some(value) = part.strip_prefix("icon-font=") {
            icon_font = Some(parse_font_preset(value, line)?);
        } else if let Some(value) = part.strip_prefix("icon-side=") {
            icon_side = match value {
                "left" => IconSide::Left,
                "right" => IconSide::Right,
                _ => return Err(error("E065", line, "input icon side must be left or right")),
            };
        } else if let Some(value) = part.strip_prefix("icon-size=") {
            icon_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("icon-spacing=") {
            icon_spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E065",
                line,
                format!("unknown input property `{part}`"),
            ));
        }
        index += 1;
    }
    if icon_code.is_some()
        || icon_font.is_some()
        || icon_size.is_some()
        || icon_spacing.is_some()
        || icon_side != IconSide::Left
    {
        options.icon = Some(TextInputIcon {
            code_point: icon_code
                .ok_or_else(|| error("E129", line, "input icon properties require `icon=\"x\"`"))?,
            font: icon_font,
            size: icon_size,
            spacing: icon_spacing,
            side: icon_side,
            span: Span::line(line.number),
        });
    }
    for child in &line.children {
        let parts = split_words(&child.text);
        match parts.first().map(String::as_str) {
            Some("active" | "hovered" | "focused" | "focused-hovered" | "disabled") => {
                ensure_leaf(child)?;
                parse_text_input_status(&parts, child, &mut options.style, "E065", "input", true)?;
            }
            Some("icon") => {
                ensure_leaf(child)?;
                if options.icon.is_some() {
                    return Err(error("E065", child, "duplicate input icon"));
                }
                options.icon = Some(parse_text_input_icon(&parts[1..], child, "E065", "input")?);
            }
            _ => {
                return Err(error(
                    "E065",
                    child,
                    "input blocks use active, hovered, focused, focused-hovered, disabled, or icon",
                ));
            }
        }
    }
    Ok(ViewNode::Input {
        label,
        id,
        binding: binding.ok_or_else(|| error("E065", line, "input requires `<-> state`"))?,
        hint,
        disabled,
        options,
        styles,
        span: Span::line(line.number),
    })
}

fn parse_button(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .filter(|part| part.starts_with('"'))
        .map(|part| string_literal(part, line))
        .transpose()?;
    let mut id = None;
    let mut disabled = None;
    let mut options = ButtonOptions::default();
    let option_start = if label.is_some() { 2 } else { 1 };
    for part in &parts[option_start..] {
        if part.starts_with('#') {
            id = Some(parse_id(part, line)?);
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("padding=") {
            options.padding = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("clip=") {
            options.clip = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            if let Some(preset) = match value {
                "primary" => Some(ButtonStylePreset::Primary),
                "secondary" => Some(ButtonStylePreset::Secondary),
                "success" => Some(ButtonStylePreset::Success),
                "warning" => Some(ButtonStylePreset::Warning),
                "danger" => Some(ButtonStylePreset::Danger),
                "text" => Some(ButtonStylePreset::Text),
                "background" => Some(ButtonStylePreset::Background),
                "subtle" => Some(ButtonStylePreset::Subtle),
                _ => None,
            } {
                options.style.preset = preset;
                options.style.custom = None;
            } else {
                let (function, args) = parse_signature(value, line).map_err(|_| {
                    error(
                        "E066",
                        line,
                        "button style must be a preset or declared style call",
                    )
                })?;
                options.style.custom = Some(ExternCall {
                    function,
                    args: parse_expr_list(&args, line)?,
                });
            }
        } else {
            return Err(error(
                "E066",
                line,
                format!("unknown button property `{part}`"),
            ));
        }
    }
    let mut content = None;
    for child in &line.children {
        let parts = split_words(&child.text);
        if parts.first().is_some_and(|part| {
            matches!(part.as_str(), "active" | "hovered" | "pressed" | "disabled")
        }) {
            parse_button_status_style(child, &mut options.style)?;
        } else {
            if content.is_some() {
                return Err(error("E066", line, "button accepts at most one child"));
            }
            content = Some(parse_view(child)?);
        }
    }
    if label.is_some() && content.is_some() {
        return Err(error(
            "E066",
            line,
            "button uses either a string label or one child, not both",
        ));
    }
    if label.is_none() && content.is_none() {
        return Err(error("E066", line, "button needs a label or one child"));
    }
    Ok(ViewNode::Button {
        label,
        content: content.map(Box::new),
        id,
        disabled,
        options,
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E066", line, "button requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

fn parse_button_status_style(line: &Line, styles: &mut ButtonStyleSet) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let (slot, status) = match parts.first().map(String::as_str) {
        Some("active") => (&mut styles.active, "active"),
        Some("hovered") => (&mut styles.hovered, "hovered"),
        Some("pressed") => (&mut styles.pressed, "pressed"),
        Some("disabled") => (&mut styles.disabled, "disabled"),
        _ => unreachable!("button status was classified before parsing"),
    };
    if slot.is_some() {
        return Err(error(
            "E066",
            line,
            format!("duplicate button {status} style"),
        ));
    }
    let mut options = ContainerStyleOptions::default();
    for part in &parts[1..] {
        if !parse_container_style_option(part, &mut options, line)? {
            return Err(error(
                "E066",
                line,
                format!("unknown button style property `{part}`"),
            ));
        }
    }
    *slot = Some(ButtonStatusStyle {
        options,
        span: Span::line(line.number),
    });
    Ok(())
}

fn parse_checkbox(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .ok_or_else(|| error("E067", line, "checkbox needs a label expression"))?;
    let mut id = None;
    let mut checked = None;
    let mut disabled = None;
    let mut options = BoolControlOptions::default();
    let mut style = CheckboxStyleSet::default();
    for part in &parts[2..] {
        if part.starts_with('#') {
            id = Some(parse_id(part, line)?);
        } else if let Some(value) = part.strip_prefix("checked=") {
            checked = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            if let Some(preset) = match value {
                "primary" => Some(CheckboxStylePreset::Primary),
                "secondary" => Some(CheckboxStylePreset::Secondary),
                "success" => Some(CheckboxStylePreset::Success),
                "danger" => Some(CheckboxStylePreset::Danger),
                _ => None,
            } {
                style.preset = preset;
                style.custom = None;
            } else {
                let (function, args) = parse_signature(value, line).map_err(|_| {
                    error(
                        "E067",
                        line,
                        "checkbox style must be a preset or declared style call",
                    )
                })?;
                style.custom = Some(ExternCall {
                    function,
                    args: parse_expr_list(&args, line)?,
                });
            }
        } else if parse_bool_control_option(part, &mut options, false, true, line)? {
        } else {
            return Err(error(
                "E067",
                line,
                format!("unknown checkbox property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_checkbox_status_style(child, &mut style)?;
    }
    Ok(ViewNode::Checkbox {
        label: parse_expr(label, line)?,
        id,
        checked: checked.ok_or_else(|| error("E067", line, "checkbox requires `checked=value`"))?,
        disabled,
        options,
        style: Box::new(style),
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E067", line, "checkbox requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

fn parse_checkbox_status_style(line: &Line, styles: &mut CheckboxStyleSet) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let status = parts.first().map(String::as_str);
    let checked = parts.get(1).map(String::as_str);
    let slot = match (status, checked) {
        (Some("active"), Some("checked")) => &mut styles.active_checked,
        (Some("active"), Some("unchecked")) => &mut styles.active_unchecked,
        (Some("hovered"), Some("checked")) => &mut styles.hovered_checked,
        (Some("hovered"), Some("unchecked")) => &mut styles.hovered_unchecked,
        (Some("disabled"), Some("checked")) => &mut styles.disabled_checked,
        (Some("disabled"), Some("unchecked")) => &mut styles.disabled_unchecked,
        _ => {
            return Err(error(
                "E067",
                line,
                "checkbox style lines use `<active|hovered|disabled> <checked|unchecked>`",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E067",
            line,
            format!(
                "duplicate checkbox {} {} style",
                status.unwrap(),
                checked.unwrap()
            ),
        ));
    }
    let mut style = CheckboxStatusStyle {
        span: Some(Span::line(line.number)),
        ..CheckboxStatusStyle::default()
    };
    for part in &parts[2..] {
        let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
        if let Some(value) = part.strip_prefix("background=") {
            style.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("icon=") {
            style.icon_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("text=") {
            style.text_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border=") {
            style.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-width=") {
            style.border_width = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius=") {
            style.radius = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-tl=") {
            style.radius_top_left = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-tr=") {
            style.radius_top_right = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-br=") {
            style.radius_bottom_right = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-bl=") {
            style.radius_bottom_left = Some(parse(value)?);
        } else {
            return Err(error(
                "E067",
                line,
                format!("unknown checkbox style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

fn parse_toggler(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .ok_or_else(|| error("E075", line, "toggler needs a label expression"))?;
    let mut checked = None;
    let mut disabled = None;
    let mut options = BoolControlOptions::default();
    let mut style = TogglerStyleSet::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("checked=") {
            checked = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("disabled=") {
            disabled = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E075", line, "toggler style must be a declared style call"))?;
            style.custom = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if parse_bool_control_option(part, &mut options, true, false, line)? {
        } else {
            return Err(error(
                "E075",
                line,
                format!("unknown toggler property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_toggler_status_style(child, &mut style)?;
    }
    Ok(ViewNode::Toggler {
        label: parse_expr(label, line)?,
        checked: checked.ok_or_else(|| error("E075", line, "toggler requires `checked=value`"))?,
        disabled,
        options,
        style: Box::new(style),
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E075", line, "toggler requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

fn parse_toggler_status_style(line: &Line, styles: &mut TogglerStyleSet) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let status = parts.first().map(String::as_str);
    let checked = parts.get(1).map(String::as_str);
    let slot = match (status, checked) {
        (Some("active"), Some("checked")) => &mut styles.active_checked,
        (Some("active"), Some("unchecked")) => &mut styles.active_unchecked,
        (Some("hovered"), Some("checked")) => &mut styles.hovered_checked,
        (Some("hovered"), Some("unchecked")) => &mut styles.hovered_unchecked,
        (Some("disabled"), Some("checked")) => &mut styles.disabled_checked,
        (Some("disabled"), Some("unchecked")) => &mut styles.disabled_unchecked,
        _ => {
            return Err(error(
                "E075",
                line,
                "toggler style lines use `<active|hovered|disabled> <checked|unchecked>`",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E075",
            line,
            format!(
                "duplicate toggler {} {} style",
                status.unwrap(),
                checked.unwrap()
            ),
        ));
    }
    let mut style = TogglerStatusStyle {
        span: Some(Span::line(line.number)),
        ..TogglerStatusStyle::default()
    };
    for part in &parts[2..] {
        let parse = |value: &str| parse_expr(strip_wrapping_parens(value), line);
        if let Some(value) = part.strip_prefix("background=") {
            style.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("background-border=") {
            style.background_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("background-border-width=") {
            style.background_border_width = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("foreground=") {
            style.foreground = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("foreground-border=") {
            style.foreground_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("foreground-border-width=") {
            style.foreground_border_width = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("text=") {
            style.text_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("radius=") {
            style.radius = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-tl=") {
            style.radius_top_left = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-tr=") {
            style.radius_top_right = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-br=") {
            style.radius_bottom_right = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("radius-bl=") {
            style.radius_bottom_left = Some(parse(value)?);
        } else if let Some(value) = part.strip_prefix("padding-ratio=") {
            style.padding_ratio = Some(parse(value)?);
        } else {
            return Err(error(
                "E075",
                line,
                format!("unknown toggler style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

fn parse_bool_control_option(
    part: &str,
    options: &mut BoolControlOptions,
    allow_alignment: bool,
    allow_icon: bool,
    line: &Line,
) -> Result<bool, Error> {
    if let Some(value) = part.strip_prefix("size=") {
        options.size = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("width=") {
        options.width = Some(parse_length(value, line)?);
    } else if let Some(value) = part.strip_prefix("spacing=") {
        options.spacing = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("text-size=") {
        options.text_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("line-height=") {
        options.line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if let Some(value) = part.strip_prefix("shaping=") {
        options.shaping = Some(parse_text_shaping(value, line, "E075")?);
    } else if let Some(value) = part.strip_prefix("wrapping=") {
        options.wrapping = Some(parse_text_wrapping(value, line, "E075")?);
    } else if let Some(value) = part.strip_prefix("font=") {
        options.font = Some(parse_font_preset(value, line)?);
    } else if allow_alignment && let Some(value) = part.strip_prefix("align=") {
        options.alignment = Some(match value {
            "default" => TextAlignment::Default,
            "left" => TextAlignment::Left,
            "center" => TextAlignment::Center,
            "right" => TextAlignment::Right,
            "justified" => TextAlignment::Justified,
            _ => return Err(error("E075", line, "unknown text alignment")),
        });
    } else if allow_icon && let Some(value) = part.strip_prefix("icon=") {
        let value = string_literal(value, line)?;
        let mut chars = value.chars();
        options.icon = chars.next();
        if options.icon.is_none() || chars.next().is_some() {
            return Err(error(
                "E067",
                line,
                "checkbox icon must contain one character",
            ));
        }
    } else if allow_icon && let Some(value) = part.strip_prefix("icon-size=") {
        options.icon_size = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if allow_icon && let Some(value) = part.strip_prefix("icon-line-height=") {
        options.icon_line_height = Some(parse_expr(strip_wrapping_parens(value), line)?);
    } else if allow_icon && let Some(value) = part.strip_prefix("icon-shaping=") {
        options.icon_shaping = Some(parse_text_shaping(value, line, "E075")?);
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn parse_text_shaping(source: &str, line: &Line, code: &'static str) -> Result<TextShaping, Error> {
    match source {
        "auto" => Ok(TextShaping::Auto),
        "basic" => Ok(TextShaping::Basic),
        "advanced" => Ok(TextShaping::Advanced),
        _ => Err(error(
            code,
            line,
            "shaping must be auto, basic, or advanced",
        )),
    }
}

fn parse_text_wrapping(
    source: &str,
    line: &Line,
    code: &'static str,
) -> Result<TextWrapping, Error> {
    match source {
        "none" => Ok(TextWrapping::None),
        "word" => Ok(TextWrapping::Word),
        "glyph" => Ok(TextWrapping::Glyph),
        "word-or-glyph" => Ok(TextWrapping::WordOrGlyph),
        _ => Err(error(
            code,
            line,
            "wrapping must be none, word, glyph, or word-or-glyph",
        )),
    }
}

fn parse_font_preset(source: &str, line: &Line) -> Result<FontPreset, Error> {
    Ok(match source {
        "default" => FontPreset::Default,
        "mono" => FontPreset::Monospace,
        name => FontPreset::Named(identifier(name, line)?),
    })
}

fn parse_slider(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let value = parts
        .get(1)
        .ok_or_else(|| error("E076", line, "slider needs a value expression"))?;
    let mut min = None;
    let mut max = None;
    let mut step = Expr::F64(1.0);
    let mut options = SliderOptions::default();
    let mut vertical = false;
    let mut release = None;
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("min=") {
            min = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("max=") {
            max = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("step=") {
            step = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("default=") {
            options.default = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("shift-step=") {
            options.shift_step = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("width=") {
            options.width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            options.height = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(value, line)
                .map_err(|_| error("E076", line, "slider style must be a declared style call"))?;
            options.style.custom = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if part == "vertical" {
            vertical = true;
        } else if let Some(value) = part.strip_prefix("release=") {
            release = Some(parse_route(value, line)?);
        } else {
            return Err(error(
                "E076",
                line,
                format!("unknown slider property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_slider_style(child, &mut options.style)?;
    }
    Ok(ViewNode::Slider {
        value: parse_expr(value, line)?,
        min: min.ok_or_else(|| error("E076", line, "slider requires `min=value`"))?,
        max: max.ok_or_else(|| error("E076", line, "slider requires `max=value`"))?,
        step,
        options: Box::new(options),
        vertical,
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E076", line, "slider requires `-> handler`"))?,
            line,
        )?,
        release,
        span: Span::line(line.number),
    })
}

fn parse_slider_style(line: &Line, styles: &mut SliderStyleSet) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let (slot, status) = match parts.first().map(String::as_str) {
        Some("active") => (&mut styles.active, "active"),
        Some("hovered") => (&mut styles.hovered, "hovered"),
        Some("dragged") => (&mut styles.dragged, "dragged"),
        _ => {
            return Err(error(
                "E076",
                line,
                "slider style block must be active, hovered, or dragged",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E076",
            line,
            format!("duplicate slider {status} style"),
        ));
    }
    let mut style = SliderStyle {
        span: Some(Span::line(line.number)),
        ..SliderStyle::default()
    };
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("rail-start=") {
            style.rail_start = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("rail-end=") {
            style.rail_end = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("rail-width=") {
            style.rail_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-border=") {
            style.rail_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("rail-border-width=") {
            style.rail_border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-radius=") {
            style.rail_radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-radius-tl=") {
            style.rail_radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-radius-tr=") {
            style.rail_radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-radius-br=") {
            style.rail_radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("rail-radius-bl=") {
            style.rail_radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle=") {
            style.handle_shape = Some(parse_slider_handle(value, line)?);
        } else if let Some(value) = part.strip_prefix("handle-color=") {
            style.handle_color = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("handle-border=") {
            style.handle_border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("handle-border-width=") {
            style.handle_border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-radius=") {
            style.handle_radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-radius-tl=") {
            style.handle_radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-radius-tr=") {
            style.handle_radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-radius-br=") {
            style.handle_radius_bottom_right =
                Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("handle-radius-bl=") {
            style.handle_radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E076",
                line,
                format!("unknown slider style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

fn parse_slider_handle(source: &str, line: &Line) -> Result<SliderHandleShape, Error> {
    if let Some(value) = source
        .strip_prefix("circle(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(SliderHandleShape::Circle(parse_expr(value, line)?));
    }
    if let Some(value) = source
        .strip_prefix("rect(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(SliderHandleShape::Rectangle {
            width: value
                .parse()
                .map_err(|_| error("E076", line, "slider rectangle width must be a u16"))?,
        });
    }
    Err(error(
        "E076",
        line,
        "slider handle must be circle(N) or rect(N)",
    ))
}

fn parse_progress(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    let value = parts
        .get(1)
        .ok_or_else(|| error("E077", line, "progress needs a value expression"))?;
    let mut min = Expr::F64(0.0);
    let mut max = Expr::F64(100.0);
    let mut options = ProgressOptions::default();
    let mut vertical = false;
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("min=") {
            min = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("max=") {
            max = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("length=") {
            options.length = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("girth=") {
            options.girth = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("style=") {
            if let Some(style) = match value {
                "primary" => Some(ProgressStyle::Primary),
                "secondary" => Some(ProgressStyle::Secondary),
                "success" => Some(ProgressStyle::Success),
                "warning" => Some(ProgressStyle::Warning),
                "danger" => Some(ProgressStyle::Danger),
                _ => None,
            } {
                options.style = Some(style);
                options.custom_style = None;
            } else {
                let (function, args) = parse_signature(value, line).map_err(|_| {
                    error(
                        "E077",
                        line,
                        "progress style must be a preset or declared style call",
                    )
                })?;
                options.custom_style = Some(ExternCall {
                    function,
                    args: parse_expr_list(&args, line)?,
                });
                options.style = None;
            }
        } else if let Some(value) = part.strip_prefix("background=") {
            options.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("bar=") {
            options.bar = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("border=") {
            options.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-width=") {
            options.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius=") {
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tl=") {
            options.radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tr=") {
            options.radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-br=") {
            options.radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-bl=") {
            options.radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if part == "vertical" {
            vertical = true;
        } else {
            return Err(error(
                "E077",
                line,
                format!("unknown progress property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Progress {
        value: parse_expr(value, line)?,
        min,
        max,
        options,
        vertical,
        styles,
        span: Span::line(line.number),
    })
}

fn parse_radio(
    parts: &[String],
    styles: Vec<String>,
    route: Option<&str>,
    line: &Line,
) -> Result<ViewNode, Error> {
    let label = parts
        .get(1)
        .ok_or_else(|| error("E078", line, "radio needs a label expression"))?;
    let mut value = None;
    let mut selected = None;
    let mut options = BoolControlOptions::default();
    let mut style = RadioStyleSet::default();
    for part in &parts[2..] {
        if let Some(source) = part.strip_prefix("value=") {
            value = Some(parse_expr(strip_wrapping_parens(source), line)?);
        } else if let Some(source) = part.strip_prefix("selected=") {
            selected = Some(parse_expr(strip_wrapping_parens(source), line)?);
        } else if let Some(source) = part.strip_prefix("style=") {
            let (function, args) = parse_signature(source, line)
                .map_err(|_| error("E078", line, "radio style must be a declared style call"))?;
            style.custom = Some(ExternCall {
                function,
                args: parse_expr_list(&args, line)?,
            });
        } else if parse_bool_control_option(part, &mut options, false, false, line)? {
        } else {
            return Err(error(
                "E078",
                line,
                format!("unknown radio property `{part}`"),
            ));
        }
    }
    for child in &line.children {
        parse_radio_status_style(child, &mut style)?;
    }
    Ok(ViewNode::Radio {
        label: parse_expr(label, line)?,
        value: value.ok_or_else(|| error("E078", line, "radio requires `value=value`"))?,
        selected: selected
            .ok_or_else(|| error("E078", line, "radio requires `selected=condition`"))?,
        options,
        style: Box::new(style),
        styles,
        route: parse_route(
            route.ok_or_else(|| error("E078", line, "radio requires `-> handler`"))?,
            line,
        )?,
        span: Span::line(line.number),
    })
}

fn parse_radio_status_style(line: &Line, styles: &mut RadioStyleSet) -> Result<(), Error> {
    ensure_leaf(line)?;
    let parts = split_words(&line.text);
    let status = parts.first().map(String::as_str);
    let selected = parts.get(1).map(String::as_str);
    let slot = match (status, selected) {
        (Some("active"), Some("selected")) => &mut styles.active_selected,
        (Some("active"), Some("unselected")) => &mut styles.active_unselected,
        (Some("hovered"), Some("selected")) => &mut styles.hovered_selected,
        (Some("hovered"), Some("unselected")) => &mut styles.hovered_unselected,
        _ => {
            return Err(error(
                "E078",
                line,
                "radio style lines use `<active|hovered> <selected|unselected>`",
            ));
        }
    };
    if slot.is_some() {
        return Err(error(
            "E078",
            line,
            format!(
                "duplicate radio {} {} style",
                status.unwrap(),
                selected.unwrap()
            ),
        ));
    }
    let mut style = RadioStatusStyle {
        span: Some(Span::line(line.number)),
        ..RadioStatusStyle::default()
    };
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("background=") {
            style.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("dot=") {
            style.dot_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border=") {
            style.border_color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-width=") {
            style.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("text=") {
            style.text_color = Some(value.to_owned());
        } else {
            return Err(error(
                "E078",
                line,
                format!("unknown radio style property `{part}`"),
            ));
        }
    }
    *slot = Some(style);
    Ok(())
}

fn parse_rule(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    let axis = match parts.get(1).map(String::as_str) {
        Some("horizontal") => Axis::Horizontal,
        Some("vertical") => Axis::Vertical,
        _ => return Err(error("E079", line, "rule uses `rule horizontal|vertical`")),
    };
    let mut thickness = Expr::F64(1.0);
    let mut options = RuleOptions::default();
    for part in &parts[2..] {
        if let Some(value) = part.strip_prefix("thickness=") {
            thickness = parse_expr(strip_wrapping_parens(value), line)?;
        } else if let Some(value) = part.strip_prefix("style=") {
            options.style = Some(match value {
                "default" => RuleStyle::Default,
                "weak" => RuleStyle::Weak,
                _ => return Err(error("E079", line, "rule style must be default or weak")),
            });
        } else if let Some(value) = part.strip_prefix("fill=") {
            options.fill = Some(parse_rule_fill(value, line)?);
        } else if let Some(value) = part.strip_prefix("color=") {
            options.color = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("radius=") {
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tl=") {
            options.radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-tr=") {
            options.radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-br=") {
            options.radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("radius-bl=") {
            options.radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("snap=") {
            options.snap = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else {
            return Err(error(
                "E079",
                line,
                format!("unknown rule property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Rule {
        axis,
        thickness,
        options,
        styles,
        span: Span::line(line.number),
    })
}

fn parse_rule_fill(source: &str, line: &Line) -> Result<RuleFill, Error> {
    if source == "full" {
        return Ok(RuleFill::Full);
    }
    if let Some(value) = source
        .strip_prefix("percent(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(RuleFill::Percent(parse_expr(value, line)?));
    }
    if let Some(value) = source
        .strip_prefix("pad(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let values = split_top(value, ',');
        let parse = |value: &str| {
            value
                .trim()
                .parse::<u16>()
                .map_err(|_| error("E079", line, "rule padding must be a u16"))
        };
        return match values.as_slice() {
            [value] => Ok(RuleFill::Padded(parse(value)?)),
            [first, second] => Ok(RuleFill::AsymmetricPadding(parse(first)?, parse(second)?)),
            _ => Err(error("E079", line, "rule pad expects one or two values")),
        };
    }
    Err(error(
        "E079",
        line,
        "rule fill must be full, percent(N), pad(N), or pad(A,B)",
    ))
}

fn parse_space(parts: &[String], styles: Vec<String>, line: &Line) -> Result<ViewNode, Error> {
    ensure_leaf(line)?;
    let mut width = None;
    let mut height = None;
    for part in &parts[1..] {
        if let Some(value) = part.strip_prefix("width=") {
            width = Some(parse_length(value, line)?);
        } else if let Some(value) = part.strip_prefix("height=") {
            height = Some(parse_length(value, line)?);
        } else {
            return Err(error(
                "E080",
                line,
                format!("unknown space property `{part}`"),
            ));
        }
    }
    Ok(ViewNode::Space {
        width,
        height,
        styles,
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
        "point" => Type::Point,
        "rectangle" => Type::Rectangle,
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
            ('<', _) => (Token::Lt, 1),
            ('>', _) => (Token::Gt, 1),
            _ => return Err(error("E070", line, format!("unexpected character `{ch}`"))),
        };
        tokens.push(token);
        index += width;
    }
    Ok(tokens)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"app Demo

extern crate::backend
  Item(id:i64, name:str)
  load() -> [Item] ! Item

theme
  background #000000

qr docs "https://example.com/ice docs" correction=high version=normal(4)

state
  items:[Item] = []
  query = ""

on mount
  run load() -> loaded _ | failed _

on loaded(next)
  items = next

on failed(error)
  query = error.name

view
  input "Query" #query <-> query @w-full
"#;

    #[test]
    fn parses_compact_app() {
        let document = parse(SOURCE).unwrap();
        assert_eq!(document.app, "Demo");
        assert_eq!(document.structs.len(), 1);
        assert_eq!(document.handlers.len(), 3);
        assert_eq!(document.qr_codes.len(), 1);
        assert_eq!(
            document.qr_codes[0].data,
            QrPayload::Text("https://example.com/ice docs".into())
        );
    }

    #[test]
    fn parses_all_native_time_operations() {
        let source = include_str!("../../../examples/iced-app/src/ui/timer.ice");
        let document = parse(source).unwrap();
        assert_eq!(document.states[1].ty, Type::Option(Box::new(Type::Instant)));
        assert!(matches!(
            &document.handlers[0].statements[0],
            Statement::Run { function, .. } if function == "__ice_time_now"
        ));
        assert!(matches!(
            document.subscriptions[0].source,
            SubscriptionSource::Every { milliseconds: 250 }
        ));
        assert!(matches!(
            &document.subscriptions[1].source,
            SubscriptionSource::Repeat {
                function,
                milliseconds: 1000
            } if function == "refresh_time"
        ));
        assert_eq!(
            document.subscriptions[1].filter.as_deref(),
            Some("even_refresh")
        );
        assert!(matches!(
            document.subscriptions[1].context,
            Some(Expr::Path(ref path)) if path == &["generation"]
        ));
        assert_eq!(
            document.subscriptions[2].filter.as_deref(),
            Some("visible_pointer")
        );
        assert!(document.subscriptions[3].context.is_none());

        let error =
            parse(&source.replace("refresh_time() every", "refresh_time(1) every")).unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("cannot take arguments"));
    }

    #[test]
    fn parses_structured_task_groups() {
        let source = SOURCE.replace(
            "  run load() -> loaded _ | failed _",
            "  parallel\n    run load() -> loaded _ | failed _\n    sequential\n      task clipboard read -> clipboard_read _\n      task system theme -> theme_read _",
        );
        let document = parse(&source).unwrap();
        let Statement::TaskGroup {
            kind, statements, ..
        } = &document.handlers[0].statements[0]
        else {
            panic!("expected task group");
        };
        assert_eq!(*kind, TaskGroupKind::Parallel);
        assert_eq!(statements.len(), 2);
        assert!(matches!(
            &statements[1],
            Statement::TaskGroup {
                kind: TaskGroupKind::Sequential,
                statements,
                ..
            } if statements.len() == 2
        ));

        let error = parse(&SOURCE.replace("  run load() -> loaded _ | failed _", "  parallel"))
            .unwrap_err();
        assert_eq!(error.code, "E050");
        assert!(error.message.contains("at least one"));
    }

    #[test]
    fn parses_abortable_tasks_and_handles() {
        let source = SOURCE
            .replace(
                "  query = \"\"",
                "  query = \"\"\n  request:task-handle? = none",
            )
            .replace(
                "  run load() -> loaded _ | failed _",
                "  abortable request abort-on-drop\n    run load() -> loaded _ | failed _",
            );
        let document = parse(&source).unwrap();
        assert_eq!(
            document.states[2].ty,
            Type::Option(Box::new(Type::TaskHandle))
        );
        assert!(matches!(
            &document.handlers[0].statements[0],
            Statement::Abortable {
                handle,
                abort_on_drop: true,
                task,
                ..
            } if handle == "request" && matches!(task.as_ref(), Statement::Run { .. })
        ));

        let error = parse(&SOURCE.replace(
            "  run load() -> loaded _ | failed _",
            "  abortable request later\n    run load() -> loaded _ | failed _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E050");
        assert!(error.message.contains("abort-on-drop"));

        let error = parse(&SOURCE.replace(
            "  run load() -> loaded _ | failed _",
            "  abortable request\n    run load() -> loaded _ | failed _\n    run load() -> loaded _ | failed _",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E050");
        assert!(error.message.contains("exactly one"));
    }

    #[test]
    fn parses_dynamic_widget_operation_ids() {
        let source = r#"app Operations
theme
  background #000000
state
  selected = 1
  value = ""
on focus
  task widget focus #outer(selected)/inner/field
view
  input "Value" #field(selected) <-> value
"#;
        let document = parse(source).unwrap();
        let Statement::WidgetOperation {
            operation: WidgetOperation::Focus { target },
            ..
        } = &document.handlers[0].statements[0]
        else {
            panic!("expected dynamic focus operation");
        };
        assert_eq!(target.segments.len(), 3);
        let id = &target.segments[0];
        assert_eq!(id.name, "outer");
        assert!(matches!(
            id.key.as_ref(),
            Some(Expr::Path(path)) if path == &["selected"]
        ));
        assert_eq!(target.segments[1].name, "inner");
        assert!(target.segments[1].key.is_none());
        assert_eq!(target.segments[2].name, "field");
        assert!(target.segments[2].key.is_none());

        let error = parse(&source.replace(
            "focus #outer(selected)/inner/field",
            "focus outer(selected)/inner/field",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E052");
        assert!(error.message.contains("#id(key)"));
    }

    #[test]
    fn parses_widget_selectors() {
        let source = include_str!("../../../examples/iced-app/src/ui/widget_selectors.ice");
        let document = parse(source).unwrap();
        assert_eq!(document.functions[0].kind, ExternKind::Selector);
        assert!(matches!(
            &document.handlers[0].statements[0],
            Statement::WidgetOperation {
                operation: WidgetOperation::Find {
                    selector: WidgetSelector::Id(_),
                    all: false,
                },
                ..
            }
        ));
        assert!(matches!(
            &document.handlers[4].statements[0],
            Statement::WidgetOperation {
                operation: WidgetOperation::Find {
                    selector: WidgetSelector::Text(_),
                    all: true,
                },
                ..
            }
        ));
        assert!(matches!(
            &document.handlers[5].statements[0],
            Statement::WidgetOperation {
                operation: WidgetOperation::Find {
                    selector: WidgetSelector::Extern { function, args },
                    all: true,
                },
                ..
            } if function == "by_kind" && args.len() == 1
        ));

        let error = parse(&source.replace("point 12.0 24.0", "point 12.0")).unwrap_err();
        assert_eq!(error.code, "E052");
        assert!(error.message.contains("requires x and y"));
    }

    #[test]
    fn parses_typed_keyboard_values() {
        let source = include_str!("../../../examples/iced-app/src/ui/keyboard_values.ice");
        let document = parse(source).unwrap();
        assert_eq!(document.states[0].ty, Type::Key);
        assert_eq!(document.states[1].ty, Type::PhysicalKey);
        assert_eq!(
            document.states[3].ty,
            Type::Option(Box::new(Type::PhysicalKey))
        );
        assert_eq!(document.states[4].ty, Type::KeyLocation);
        assert_eq!(document.states[5].ty, Type::KeyModifiers);
        assert!(matches!(
            &document.states[0].initial,
            Expr::Call { name, args } if name == "key.unidentified" && args.is_empty()
        ));
        assert!(matches!(
            &document.handlers[0].statements[4],
            Statement::Assign {
                value: Expr::Call { name, args },
                ..
            } if name == "key.latin" && args.len() == 2
        ));
    }

    #[test]
    fn parses_typed_pointer_values() {
        let source = include_str!("../../../examples/iced-app/src/ui/pointer_values.ice");
        let document = parse(source).unwrap();
        assert_eq!(document.states[0].ty, Type::Point);
        assert_eq!(document.states[1].ty, Type::Rectangle);
        assert_eq!(document.states[2].ty, Type::MouseButton);
        assert_eq!(document.states[5].ty, Type::MouseCursor);
        assert_eq!(document.states[7].ty, Type::MouseClick);
        assert_eq!(document.states[8].ty, Type::TouchFinger);
        assert!(matches!(
            &document.states[7].initial,
            Expr::Call { name, args } if name == "mouse.click" && args.len() == 3
        ));
        assert!(matches!(
            &document.handlers[0].statements[0],
            Statement::Assign {
                value: Expr::Call { name, args },
                ..
            } if name == "mouse.cursor_position" && args.len() == 1
        ));
    }

    #[test]
    fn parses_typed_task_streams() {
        let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream range(start:i64, limit:i64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(id:i64) -> str
  event-filter raw_event() -> str
theme
  background #000000
on start
  parallel
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
on number(value)
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(3) -> number _
  run range(1, 3) -> number _
  recipe snapshot(3) -> text _
  events 3 using=raw_event -> text _
view
  text "Streams"
"#;
        let document = parse(source).unwrap();
        assert_eq!(document.functions[0].kind, ExternKind::Stream);
        assert_eq!(
            document.functions[2].error,
            Some(Type::Named("AppError".into()))
        );
        let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
            panic!("expected task group");
        };
        assert!(statements.iter().all(|statement| matches!(
            statement,
            Statement::Run {
                kind: EffectKind::Stream,
                ..
            }
        )));
        assert!(matches!(
            &document.subscriptions[0].source,
            SubscriptionSource::Run { function, args }
                if function == "fallible" && args.is_empty()
        ));
        assert!(matches!(
            &document.subscriptions[2].source,
            SubscriptionSource::Run { function, args }
                if function == "range" && args.len() == 2
        ));
        assert!(matches!(
            &document.subscriptions[3].source,
            SubscriptionSource::Recipe { function, args }
                if function == "snapshot" && args.len() == 1
        ));
        assert!(matches!(
            &document.subscriptions[4].source,
            SubscriptionSource::Events { id: Expr::I64(3), filter }
                if filter == "raw_event"
        ));

        let error = parse(&source.replace(
            "recipe snapshot(id:i64) -> str",
            "recipe snapshot(id:i64) -> str ! AppError",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E023");

        let error = parse(&source.replace(
            "event-filter raw_event() -> str",
            "event-filter raw_event(value:i64) -> str",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E022");

        let error = parse(&source.replace("stream numbers(3) -> number _", "stream numbers(3)"))
            .unwrap_err();
        assert_eq!(error.code, "E050");
        assert!(error.message.contains("stream requires"));
    }

    #[test]
    fn parses_generic_event_subscriptions() {
        let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on received(value)
on identified(id, value)
subscribe
  event -> received _
  event status=any -> received _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id -> identified _ _
view
  text "Events"
"#;
        let document = parse(source).unwrap();
        assert_eq!(document.functions[0].params[0].1, Type::Event);
        assert!(matches!(
            document.subscriptions[0].source,
            SubscriptionSource::Event { raw: false }
        ));
        assert!(!document.subscriptions[0].window_id);
        assert_eq!(document.subscriptions[2].status, Some(EventStatus::Ignored));
        assert!(document.subscriptions[2].window_id);
        assert!(matches!(
            document.subscriptions[3].source,
            SubscriptionSource::Event { raw: true }
        ));
        assert!(document.subscriptions[4].window_id);

        let error = parse(&source.replace("event -> received _", "event redraw -> received _"))
            .unwrap_err();
        assert_eq!(error.code, "E084");
        assert!(error.message.contains("event [raw] [with-id]"));
    }

    #[test]
    fn parses_typed_task_sips() {
        let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip download(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
theme
  background #000000
on start
  parallel
    sip download(3)
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
        let document = parse(source).unwrap();
        assert_eq!(document.functions[0].kind, ExternKind::Sip);
        assert_eq!(document.functions[0].progress, Some(Type::F64));
        assert_eq!(
            document.functions[1].error,
            Some(Type::Named("AppError".into()))
        );
        let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
            panic!("expected task group");
        };
        assert!(
            statements
                .iter()
                .all(|statement| matches!(statement, Statement::Sip { .. }))
        );

        let error = parse(&source.replace("      progress -> advanced _\n", "")).unwrap_err();
        assert_eq!(error.code, "E050");
        assert!(error.message.contains("progress"));
    }

    #[test]
    fn parses_structured_task_flows() {
        let source = r#"app Flows
extern crate::backend
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
theme
  background #000000
on start
  flow
    from stream numbers(3)
    then value -> task double(value)
    collect
    done -> collected _
    units -> planned _
on collected(values)
on planned(units)
view
  text "Flows"
"#;
        let document = parse(source).unwrap();
        let Statement::TaskFlow {
            source: task_source,
            transforms,
            success,
            units,
            ..
        } = &document.handlers[0].statements[0]
        else {
            panic!("expected task flow");
        };
        assert!(matches!(
            task_source,
            TaskSource::Effect {
                kind: EffectKind::Stream,
                ..
            }
        ));
        assert_eq!(transforms.len(), 2);
        assert!(matches!(transforms[0], TaskTransform::Then { .. }));
        assert!(matches!(transforms[1], TaskTransform::Collect { .. }));
        assert!(success.is_some());
        assert!(units.is_some());

        let error =
            parse(&source.replace("    from stream numbers(3)", "    collect")).unwrap_err();
        assert_eq!(error.code, "E050");
        assert!(error.message.contains("first flow line"));
    }

    #[test]
    fn parses_task_error_mapping_and_native_sources() {
        let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
theme
  background #000000
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
on finished(value)
view
  text "Errors"
"#;
        let document = parse(source).unwrap();
        assert_eq!(document.functions[0].kind, ExternKind::Sync);
        assert_eq!(
            document.states[0].ty,
            Type::List(Box::new(Type::Result(
                Box::new(Type::I64),
                Box::new(Type::Named("AppError".into()))
            )))
        );
        let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
            panic!("expected task group");
        };
        assert!(matches!(
            &statements[0],
            Statement::TaskFlow { transforms, .. }
                if matches!(transforms[0], TaskTransform::MapError { .. })
        ));
        assert!(matches!(
            &statements[1],
            Statement::TaskFlow {
                source: TaskSource::Done { .. },
                ..
            }
        ));
        assert!(matches!(
            &statements[2],
            Statement::TaskFlow {
                source: TaskSource::None {
                    output: Type::I64,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn parses_checked_application_and_window_settings() {
        let source = SOURCE.replace(
            "app Demo",
            r##"app Demo
  title "Configured"
  theme "dark"
  background "#123456"
  text-color "#abcdef"
  id "dev.example.demo"
  executor iced::executor::Default
  font "assets/Brand.ttf"
  font "assets/Icons.otf"
  default-text-size 15
  antialiasing false
  vsync false
  scale-factor 1.25
  window
    icon-rgba "assets/app.rgba" 2 1
    size 960 720
    min-size 480 360
    max-size 1920 1080
    position centered
    level always-on-top
    visible true
    platform linux
      application-id "dev.example.demo"
      override-redirect false
    platform windows
      drag-and-drop true
      skip-taskbar false
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target none
  window child
    size 640 480
    position centered"##,
        );
        let document = parse(&source).unwrap();
        assert!(matches!(
            document.settings.title.as_ref().map(|setting| &setting.value),
            Some(Expr::Str(value)) if value == "Configured"
        ));
        assert_eq!(
            document.settings.executor.as_deref(),
            Some("iced::executor::Default")
        );
        assert!(matches!(
            document
                .settings
                .scale_factor
                .as_ref()
                .map(|setting| &setting.value),
            Some(Expr::F64(value)) if *value == 1.25
        ));
        assert!(matches!(
            document.settings.theme.as_ref().map(|setting| &setting.value),
            Some(Expr::Str(value)) if value == "dark"
        ));
        assert_eq!(document.settings.fonts.len(), 2);
        assert_eq!(document.settings.fonts[0].path, "assets/Brand.ttf");
        let window = document.settings.window.unwrap();
        assert_eq!(window.size, Some((960.0, 720.0)));
        assert!(matches!(window.position, Some(WindowPosition::Centered)));
        assert!(matches!(window.level, Some(WindowLevel::AlwaysOnTop)));
        assert_eq!(
            window
                .linux
                .as_ref()
                .and_then(|settings| settings.application_id.as_deref()),
            Some("dev.example.demo")
        );
        assert!(matches!(
            window.windows.as_ref().and_then(|settings| settings.corner),
            Some(WindowCorner::RoundSmall)
        ));
        assert_eq!(
            window
                .macos
                .as_ref()
                .and_then(|settings| settings.fullsize_content_view),
            Some(true)
        );
        assert_eq!(
            window
                .wasm
                .as_ref()
                .and_then(|settings| settings.target.clone()),
            Some(None)
        );
        let icon = window.icon.unwrap();
        assert_eq!(
            (icon.path.as_str(), icon.width, icon.height, icon.byte_len),
            ("assets/app.rgba", 2, 1, 8)
        );
        assert_eq!(document.settings.windows.len(), 1);
        assert_eq!(document.settings.windows[0].name, "child");
        assert_eq!(
            document.settings.windows[0].settings.size,
            Some((640.0, 480.0))
        );

        let duplicate_window = source.replace(
            "  window child\n    size 640 480\n    position centered",
            "  window child\n    size 640 480\n    position centered\n  window child\n    size 320 240",
        );
        let error = parse(&duplicate_window).unwrap_err();
        assert_eq!(error.code, "E014");
        assert!(error.message.contains("duplicate app window"));

        let error = parse(&source.replace("min-size 480 360", "min-size 2000 360")).unwrap_err();
        assert_eq!(error.code, "E015");
        assert!(error.message.contains("min-size cannot exceed max-size"));

        let error = parse(&source.replace("size 960 720", "size 0 720")).unwrap_err();
        assert_eq!(error.code, "E015");
        assert!(error.message.contains("greater than zero"));

        let error = parse(&source.replace(
            "  antialiasing false",
            "  antialiasing false\n  antialiasing true",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E014");
        assert!(error.message.contains("duplicate"));

        let duplicate_font =
            source.replace("  font \"assets/Icons.otf\"", "  font \"assets/Brand.ttf\"");
        let error = parse(&duplicate_font).unwrap_err();
        assert_eq!(error.code, "E014");
        assert!(error.message.contains("duplicate app font"));

        let error =
            parse(&source.replace("  font \"assets/Brand.ttf\"", "  font \"\"")).unwrap_err();
        assert_eq!(error.code, "E015");
        assert!(error.message.contains("relative `/` paths"));

        let error =
            parse(&source.replace("  font \"assets/Brand.ttf\"", "  font \"/tmp/Brand.ttf\""))
                .unwrap_err();
        assert_eq!(error.code, "E015");
        assert!(error.message.contains("relative `/` paths"));

        let error = parse(&source.replace(
            "icon-rgba \"assets/app.rgba\" 2 1",
            "icon-rgba \"assets/app.rgba\" 2 0",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E015");
        assert!(error.message.contains("positive integers"));

        let error = parse(&source.replace(
            "executor iced::executor::Default",
            "executor iced::bad-path",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E073");

        let error = parse(&source.replace(
            "    platform linux\n      application-id \"dev.example.demo\"\n      override-redirect false",
            "    platform plan9\n      application-id \"dev.example.demo\"",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E015");
        assert!(error.message.contains("linux, windows, macos, or wasm"));

        let error =
            parse(&source.replace("corner round-small", "corner softly-rounded")).unwrap_err();
        assert_eq!(error.code, "E015");
        assert!(error.message.contains("window corner"));

        let error = parse(&source.replace(
            "    platform wasm\n      target none",
            "    platform wasm\n      target none\n    platform wasm\n      target \"app\"",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E014");
        assert!(error.message.contains("duplicate setting `platform wasm`"));

        let error = parse(&source.replace(
            "      skip-taskbar false",
            "      skip-taskbar false\n      skip-taskbar true",
        ))
        .unwrap_err();
        assert_eq!(error.code, "E014");
        assert!(error.message.contains("duplicate setting `skip-taskbar`"));
    }

    #[test]
    fn rejects_non_assignment_preset_state() {
        let source = SOURCE.replace(
            "view\n",
            "preset seeded\n  state\n    return if true\nview\n",
        );
        let error = parse(&source).unwrap_err();
        assert_eq!(error.code, "E016");
        assert!(error.message.contains("only accepts"));
    }

    #[test]
    fn accepts_an_input_without_an_id() {
        let source = SOURCE.replace(
            "input \"Query\" #query <-> query",
            "input \"Query\" <-> query",
        );
        parse(&source).unwrap();
    }

    #[test]
    fn parses_every_pick_list_handle() {
        for handle in [
            "handle arrow size=12.0",
            "handle static code=\"⌄\" font=default size=12.0 line-height=1.0 shaping=basic",
            "handle dynamic\n      closed code=\"⌄\"\n      open code=\"⌃\"",
            "handle none",
        ] {
            let source = format!(
                r#"app Selection
state
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
view
  pick choices selected -> selected _
    active text=foreground placeholder=muted handle=primary background=surface border=border border-width=1.0 radius=4.0
    hovered text=foreground
    opened text=foreground
    opened-hovered text=foreground
    menu text=foreground selected-text=foreground selected-background=primary background=surface shadow=black shadow-y=2.0
    {handle}
"#
            );
            parse(&source).unwrap_or_else(|error| panic!("{handle}: {error:?}"));
        }
    }

    #[test]
    fn names_missing_qr_data() {
        let source = SOURCE.replace(
            "qr docs \"https://example.com/ice docs\" correction=high version=normal(4)",
            "qr",
        );
        let error = parse(&source).unwrap_err();
        assert_eq!(error.code, "E093");
        assert!(error.message.contains("needs a name"));
    }

    #[test]
    fn parses_editor_extension_boundaries() {
        let source = r#"app Notes
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-style editor_surface(readonly:bool)
state
  body:editor = ""
  readonly = false
  language = "rs"
on command(value)
view
  editor <-> body highlighter=editor_highlight(language) key-binding=editor_keys(readonly) style=editor_surface(readonly) -> command _
"#;
        let document = parse(source).unwrap();
        assert_eq!(document.functions[0].kind, ExternKind::EditorBinding);
        assert_eq!(document.functions[1].kind, ExternKind::EditorHighlighter);
        assert_eq!(document.functions[2].kind, ExternKind::EditorStyle);
        let ViewNode::TextEditor { options, .. } = &document.view else {
            panic!("expected editor");
        };
        assert_eq!(
            options.highlighter.as_ref().unwrap().function,
            "editor_highlight"
        );
        assert_eq!(
            options.key_binding.as_ref().unwrap().function,
            "editor_keys"
        );
        assert_eq!(
            options.custom_style.as_ref().unwrap().function,
            "editor_surface"
        );
        assert!(options.key_binding_route.is_some());

        let error = parse(&source.replace(" key-binding=editor_keys(readonly)", "")).unwrap_err();
        assert!(error.message.contains("route requires key-binding"));

        let error = parse(&source.replace(" -> command _", "")).unwrap_err();
        assert!(error.message.contains("key-binding requires"));

        let error =
            parse(&source.replace(" highlighter=", " highlight=\"rs\" highlighter=")).unwrap_err();
        assert!(error.message.contains("either highlight or highlighter"));
    }

    #[test]
    fn accepts_every_built_in_nested_theme() {
        for preset in [
            "light",
            "dark",
            "dracula",
            "nord",
            "solarized-light",
            "solarized-dark",
            "gruvbox-light",
            "gruvbox-dark",
            "catppuccin-latte",
            "catppuccin-frappe",
            "catppuccin-macchiato",
            "catppuccin-mocha",
            "tokyo-night",
            "tokyo-night-storm",
            "tokyo-night-light",
            "kanagawa-wave",
            "kanagawa-dragon",
            "kanagawa-lotus",
            "moonfly",
            "nightfly",
            "oxocarbon",
            "ferra",
        ] {
            let source = SOURCE.replace(
                "view\n  input",
                &format!("view\n  theme {preset}\n    input"),
            );
            parse(&source).unwrap_or_else(|error| panic!("{preset}: {error:?}"));
        }
    }
}
