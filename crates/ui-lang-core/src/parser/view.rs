use super::*;

pub(super) fn parse_view(line: &Line) -> Result<ViewNode, Error> {
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

pub(super) fn split_style_utilities(source: &str) -> (&str, Vec<String>) {
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

pub(super) fn parse_component_slots(
    component: &str,
    line: &Line,
) -> Result<Vec<ComponentSlot>, Error> {
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

pub(super) fn compound_slot_name(component: &str, line: &Line) -> Option<String> {
    let head = line.text.split_ascii_whitespace().next()?;
    let name = head.split_once('(').map_or(head, |(name, _)| name);
    let slot = name.strip_prefix(component)?.strip_prefix('.')?;
    (!slot.contains('.'))
        .then(|| identifier(slot, line).ok())
        .flatten()
}

pub(super) fn parse_container(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_overlay(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_pane_ratio(value: &str, line: &Line) -> Result<f32, Error> {
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

pub(super) fn parse_background_value(source: &str, line: &Line) -> Result<BackgroundValue, Error> {
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

pub(super) fn parse_container_style_option(
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

pub(super) fn parse_pane_view(
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

pub(super) fn parse_structured_pane(
    line: &Line,
) -> Result<(Box<ViewNode>, Option<PaneTitle>), Error> {
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

pub(super) fn parse_pane_title(
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

pub(super) fn parse_pane_configuration(
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

pub(super) fn parse_closed_pane(
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

pub(super) fn parse_pane_grid(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_pane_grid_style(line: &Line) -> Result<PaneGridStyle, Error> {
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

pub(super) fn parse_component_call(
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

pub(super) fn parse_text_editor(
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

pub(super) fn parse_table(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_table_column(line: &Line) -> Result<TableColumn, Error> {
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

pub(super) fn parse_markdown(
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

pub(super) fn parse_markdown_style(line: &Line) -> Result<MarkdownStyleOptions, Error> {
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

pub(super) fn parse_lazy(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_keyed_column(
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

pub(super) fn parse_slot(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_theme(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_theme_preset(value: &str, line: &Line) -> Result<ThemePreset, Error> {
    match value {
        "default" => Ok(ThemePreset::Default),
        "app" => Ok(ThemePreset::App),
        value if BUILT_IN_THEMES.contains(&value) => Ok(ThemePreset::BuiltIn(value.into())),
        _ => Err(error("E094", line, format!("unknown iced theme `{value}`"))),
    }
}

pub(super) fn parse_qr_code(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_float(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_pin(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_sensor(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_responsive(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_size_route(source: &str, line: &Line) -> Result<Route, Error> {
    parse_payload_route(source, line, 2)
}

pub(super) fn parse_payload_route(source: &str, line: &Line, count: usize) -> Result<Route, Error> {
    let mut route = parse_route(source, line)?;
    if route.args.is_empty() {
        route.args = vec![RouteArg::Payload; count];
    }
    Ok(route)
}

pub(super) fn parse_combo_box(
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

pub(super) fn parse_combo_box_child(
    line: &Line,
    options: &mut ComboBoxOptions,
) -> Result<(), Error> {
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

pub(super) fn parse_text_input_status(
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

pub(super) fn parse_text_input_icon(
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

pub(super) fn parse_pick_list(
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

pub(super) fn parse_pick_list_child(
    line: &Line,
    options: &mut PickListOptions,
) -> Result<(), Error> {
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

pub(super) fn parse_pick_list_status(
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

pub(super) fn parse_menu_style(
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

pub(super) fn parse_pick_list_handle(
    parts: &[String],
    line: &Line,
) -> Result<PickListHandle, Error> {
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

pub(super) fn parse_pick_list_icon(parts: &[String], line: &Line) -> Result<PickListIcon, Error> {
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

pub(super) fn parse_media(
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

pub(super) fn parse_length(source: &str, line: &Line) -> Result<LengthValue, Error> {
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

pub(super) fn parse_tooltip(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_mouse_area(
    parts: &[String],
    styles: Vec<String>,
    line: &Line,
) -> Result<ViewNode, Error> {
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

pub(super) fn parse_mouse_interaction(
    source: &str,
    line: &Line,
) -> Result<MouseInteraction, Error> {
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
