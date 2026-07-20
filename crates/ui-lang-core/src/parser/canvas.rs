use super::*;

pub(in crate::parser) fn parse_canvas(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(in crate::parser) fn parse_canvas_event(line: &Line) -> Result<CanvasEvent, Error> {
    if let Some(source) = line.text.strip_prefix("event ")
        && source.contains(" -> ")
    {
        ensure_leaf(line)?;
        let mut event_line = line.clone();
        event_line.text = source.to_owned();
        event_line.track_symbols = false;
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
        let (_, route) = split_top_marker(source, "->").expect("compact canvas route checked");
        line.record_symbol(
            SymbolKind::Handler,
            &subscription.route.handler,
            false,
            route.trim(),
        );
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

pub(in crate::parser) fn validate_canvas_event_source(
    source: &SubscriptionSource,
    line: &Line,
) -> Result<(), Error> {
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

pub(in crate::parser) fn parse_canvas_event_source(
    source: &str,
    line: &Line,
) -> Result<SubscriptionSource, Error> {
    let mut event_line = line.clone();
    event_line.text = format!("{source} -> __canvas_event");
    event_line.children.clear();
    event_line.track_symbols = false;
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

pub(in crate::parser) fn parse_canvas_commands(
    lines: &[Line],
) -> Result<Vec<CanvasCommand>, Error> {
    lines.iter().map(parse_canvas_command).collect()
}

pub(in crate::parser) fn parse_canvas_command(line: &Line) -> Result<CanvasCommand, Error> {
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

pub(in crate::parser) fn parse_canvas_text(
    parts: &[String],
    line: &Line,
) -> Result<CanvasCommand, Error> {
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

pub(in crate::parser) fn parse_canvas_path_segment(
    line: &Line,
) -> Result<CanvasPathSegment, Error> {
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

pub(in crate::parser) fn canvas_fields(
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

pub(in crate::parser) fn canvas_required_expr(
    fields: &BTreeMap<String, String>,
    name: &str,
    line: &Line,
) -> Result<Expr, Error> {
    fields
        .get(name)
        .ok_or_else(|| error("E190", line, format!("canvas command requires `{name}=`")))
        .and_then(|value| parse_expr(strip_wrapping_parens(value), line))
}

pub(in crate::parser) fn canvas_optional_expr(
    fields: &BTreeMap<String, String>,
    name: &str,
    line: &Line,
) -> Result<Option<Expr>, Error> {
    fields
        .get(name)
        .map(|value| parse_expr(strip_wrapping_parens(value), line))
        .transpose()
}

pub(in crate::parser) fn parse_canvas_radius(
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

pub(in crate::parser) fn parse_canvas_paint(
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

pub(in crate::parser) fn require_canvas_paint(
    paint: &CanvasPaint,
    line: &Line,
) -> Result<(), Error> {
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

pub(in crate::parser) fn parse_canvas_stroke(
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
