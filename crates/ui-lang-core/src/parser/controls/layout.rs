use super::*;

pub(in crate::parser) fn parse_layout_options(
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

pub(in crate::parser) fn parse_grid_sizing(source: &str, line: &Line) -> Result<GridSizing, Error> {
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

pub(in crate::parser) fn parse_scroll_status_style(
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

pub(in crate::parser) fn parse_scroll_style_property(
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

pub(in crate::parser) fn parse_scroll_surface_property(
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

pub(in crate::parser) fn parse_scroll_style_bool(source: &str, line: &Line) -> Result<bool, Error> {
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

pub(in crate::parser) fn parse_flex_alignment(
    source: &str,
    line: &Line,
) -> Result<FlexAlignment, Error> {
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

pub(in crate::parser) fn parse_scroll_anchor(
    source: &str,
    line: &Line,
) -> Result<ScrollAnchor, Error> {
    match source {
        "start" => Ok(ScrollAnchor::Start),
        "end" => Ok(ScrollAnchor::End),
        _ => Err(error("E074", line, "scroll anchor must be start or end")),
    }
}
