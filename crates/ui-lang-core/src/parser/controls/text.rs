use super::*;

pub(in crate::parser) fn parse_text(
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

pub(in crate::parser) fn parse_rich_text(
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

pub(in crate::parser) fn parse_rich_span(line: &Line) -> Result<RichSpan, Error> {
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
        } else if let Some(value) = part.strip_prefix("bg=") {
            options.background = Some(parse_background_value(value, line)?);
        } else if let Some(value) = part.strip_prefix("border=") {
            options.border = Some(value.to_owned());
        } else if let Some(value) = part.strip_prefix("border-w=") {
            options.border_width = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r=") {
            options.radius = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-tl=") {
            options.radius_top_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-tr=") {
            options.radius_top_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-br=") {
            options.radius_bottom_right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("r-bl=") {
            options.radius_bottom_left = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("p=") {
            options.padding.all = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("px=") {
            options.padding.x = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("py=") {
            options.padding.y = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("pt=") {
            options.padding.top = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("pr=") {
            options.padding.right = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("pb=") {
            options.padding.bottom = Some(parse_expr(strip_wrapping_parens(value), line)?);
        } else if let Some(value) = part.strip_prefix("pl=") {
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
