use super::*;

pub(super) fn parse_extern_component(
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

pub(super) fn parse_shader(
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

pub(super) fn parse_layout_options(
    kind: &str,
    parts: &[String],
    line: &Line,
) -> Result<LayoutOptions, Error> {
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

pub(super) fn parse_grid_sizing(source: &str, line: &Line) -> Result<GridSizing, Error> {
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

pub(super) fn parse_scroll_status_style(
    parts: &[String],
    line: &Line,
) -> Result<ScrollStatusStyle, Error> {
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

pub(super) fn parse_scroll_style_property(
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

pub(super) fn parse_scroll_surface_property(
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

pub(super) fn parse_scroll_style_bool(source: &str, line: &Line) -> Result<bool, Error> {
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

pub(super) fn parse_flex_alignment(source: &str, line: &Line) -> Result<FlexAlignment, Error> {
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

pub(super) fn parse_scroll_anchor(source: &str, line: &Line) -> Result<ScrollAnchor, Error> {
    match source {
        "start" => Ok(ScrollAnchor::Start),
        "end" => Ok(ScrollAnchor::End),
        _ => Err(error("E074", line, "scroll anchor must be start or end")),
    }
}

pub(super) fn parse_text(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_rich_text(
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

pub(super) fn parse_rich_span(line: &Line) -> Result<RichSpan, Error> {
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

pub(super) fn parse_input(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_button(
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

pub(super) fn parse_button_status_style(
    line: &Line,
    styles: &mut ButtonStyleSet,
) -> Result<(), Error> {
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

pub(super) fn parse_checkbox(
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

pub(super) fn parse_checkbox_status_style(
    line: &Line,
    styles: &mut CheckboxStyleSet,
) -> Result<(), Error> {
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

pub(super) fn parse_toggler(
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

pub(super) fn parse_toggler_status_style(
    line: &Line,
    styles: &mut TogglerStyleSet,
) -> Result<(), Error> {
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

pub(super) fn parse_bool_control_option(
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

pub(super) fn parse_text_shaping(
    source: &str,
    line: &Line,
    code: &'static str,
) -> Result<TextShaping, Error> {
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

pub(super) fn parse_text_wrapping(
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

pub(super) fn parse_font_preset(source: &str, line: &Line) -> Result<FontPreset, Error> {
    Ok(match source {
        "default" => FontPreset::Default,
        "mono" => FontPreset::Monospace,
        name => FontPreset::Named(identifier(name, line)?),
    })
}

pub(super) fn parse_slider(
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

pub(super) fn parse_slider_style(line: &Line, styles: &mut SliderStyleSet) -> Result<(), Error> {
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

pub(super) fn parse_slider_handle(source: &str, line: &Line) -> Result<SliderHandleShape, Error> {
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

pub(super) fn parse_progress(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_radio(
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

pub(super) fn parse_radio_status_style(
    line: &Line,
    styles: &mut RadioStyleSet,
) -> Result<(), Error> {
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

pub(super) fn parse_rule(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_rule_fill(source: &str, line: &Line) -> Result<RuleFill, Error> {
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

pub(super) fn parse_space(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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
