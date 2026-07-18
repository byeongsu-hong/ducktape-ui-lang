use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_pane_grid(
    name: &str,
    options: &PaneGridOptions,
    panes: &[PaneView],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let arms = panes
        .iter()
        .map(|pane| {
            let pane_scope = format!("format!(\"{{}}/{}\", {scope})", pane.name);
            Ok(format!(
                "{} => {}",
                rust_string(&pane.name),
                render_pane_content(pane, document, message, env, &pane_scope, slot)?
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let field = pane_field(name);
    let mut code = format!(
        "::iced::widget::pane_grid(&self.{field}, move |_, __pane_name, _| match *__pane_name {{ {arms}, _ => ::core::unreachable!() }})"
    );
    for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    for (value, method) in [
        (&options.spacing, "spacing"),
        (&options.min_size, "min_size"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(leeway) = &options.resize_leeway {
        write!(
            code,
            ".on_resize({} as f32, {message}::{})",
            expr_code(leeway, env, document, ValueMode::Owned)?,
            pane_resize_variant(name)
        )
        .unwrap();
    }
    if options.draggable {
        write!(code, ".on_drag({message}::{})", pane_drag_variant(name)).unwrap();
    }
    if let Some(route) = &options.click {
        let route = route_code(route, "__pane_name.to_owned()", env, document, message)?;
        write!(
            code,
            ".on_click(move |__pane| {{ let __pane_name = self.{field}.get(__pane).copied().unwrap_or(\"\"); {route} }})"
        )
        .unwrap();
    }
    append_pane_grid_style(&mut code, &options.style, env, document)?;
    Ok(format!("{code}.into()"))
}

pub(in crate::codegen) fn append_pane_grid_style(
    code: &mut String,
    style: &PaneGridStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = style.region_radius.is_some()
        || style.region_radius_top_left.is_some()
        || style.region_radius_top_right.is_some()
        || style.region_radius_bottom_right.is_some()
        || style.region_radius_bottom_left.is_some();
    if style.region_background.is_none()
        && style.region_border.is_none()
        && style.region_border_width.is_none()
        && !has_radius
        && style.hovered_split.is_none()
        && style.hovered_split_width.is_none()
        && style.picked_split.is_none()
        && style.picked_split_width.is_none()
    {
        return Ok(());
    }
    code.push_str(
        ".style(move |__theme| { let mut __style = ::iced::widget::pane_grid::default(__theme);",
    );
    if let Some(background) = &style.region_background {
        write!(
            code,
            " __style.hovered_region.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(border) = &style.region_border {
        write!(
            code,
            " __style.hovered_region.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if let Some(width) = &style.region_border_width {
        write!(
            code,
            " __style.hovered_region.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius = radius_code(
            style.region_radius.as_ref(),
            [
                style.region_radius_top_left.as_ref(),
                style.region_radius_top_right.as_ref(),
                style.region_radius_bottom_right.as_ref(),
                style.region_radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("pane-grid region radius options were present");
        write!(code, " __style.hovered_region.border.radius = {radius};").unwrap();
    }
    for (color, width, field) in [
        (
            &style.hovered_split,
            &style.hovered_split_width,
            "hovered_split",
        ),
        (
            &style.picked_split,
            &style.picked_split_width,
            "picked_split",
        ),
    ] {
        if let Some(color) = color {
            write!(
                code,
                " __style.{field}.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = width {
            write!(
                code,
                " __style.{field}.width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn render_pane_content(
    pane: &PaneView,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let body = render_node(&pane.content, document, message, env, scope, slot)?;
    let mut declarations = format!("let __pane_content: ::iced::Element<'_, {message}> = {body};");
    let mut content = String::from("::iced::widget::pane_grid::Content::new(__pane_content)");
    if let Some(style) = container_surface_style_value(
        &Style::parse(&pane.styles, document),
        &pane.style,
        None,
        env,
        document,
    )? {
        write!(content, ".style(move |_| {style})").unwrap();
    }
    if let Some(title) = &pane.title {
        let title_content = render_node(&title.content, document, message, env, scope, slot)?;
        write!(
            declarations,
            " let __pane_title: ::iced::Element<'_, {message}> = {title_content};"
        )
        .unwrap();
        let mut title_bar = String::from("::iced::widget::pane_grid::TitleBar::new(__pane_title)");
        if let Some(padding) = typed_padding_code(&title.padding, env, document)? {
            write!(title_bar, ".padding({padding})").unwrap();
        }
        if let Some(controls) = &title.controls {
            let controls = render_node(controls, document, message, env, scope, slot)?;
            write!(
                declarations,
                " let __pane_controls: ::iced::Element<'_, {message}> = {controls};"
            )
            .unwrap();
            if let Some(compact) = &title.compact_controls {
                let compact = render_node(compact, document, message, env, scope, slot)?;
                write!(
                    declarations,
                    " let __pane_compact_controls: ::iced::Element<'_, {message}> = {compact};"
                )
                .unwrap();
                title_bar.push_str(".controls(::iced::widget::pane_grid::Controls::dynamic(__pane_controls, __pane_compact_controls))");
            } else {
                title_bar.push_str(
                    ".controls(::iced::widget::pane_grid::Controls::new(__pane_controls))",
                );
            }
        }
        if title.always_show_controls {
            title_bar.push_str(".always_show_controls()");
        }
        if let Some(style) = container_surface_style_value(
            &Style::parse(&title.styles, document),
            &title.style,
            None,
            env,
            document,
        )? {
            write!(title_bar, ".style(move |_| {style})").unwrap();
        }
        write!(content, ".title_bar({title_bar})").unwrap();
    }
    Ok(format!("{{ {declarations} {content} }}"))
}

pub(in crate::codegen) fn render_rich_span(
    item: &RichSpan,
    document: &Document,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let style = Style::parse(&item.styles, document);
    let value = expr_code(&item.value, env, document, ValueMode::Owned)?;
    let mut code = format!("::iced::widget::span({value})");
    if let Some(size) = &item.options.size {
        write!(
            code,
            ".size({} as f32)",
            expr_code(size, env, document, ValueMode::Owned)?
        )
        .unwrap();
    } else if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    if let Some(line_height) = &item.options.line_height {
        let line_height = match line_height {
            TextLineHeight::Relative(value) => format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            TextLineHeight::Absolute(value) => format!(
                "::iced::widget::text::LineHeight::Absolute(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
        };
        write!(code, ".line_height({line_height})").unwrap();
    }
    if let Some(font) = &item.options.font {
        let font = font_preset_code(font, document)?;
        if style.bold {
            write!(
                code,
                ".font(::iced::Font {{ weight: ::iced::font::Weight::Bold, ..{font} }})"
            )
            .unwrap();
        } else {
            write!(code, ".font({font})").unwrap();
        }
    } else if style.bold {
        code.push_str(
            ".font(::iced::Font { weight: ::iced::font::Weight::Bold, ..::iced::Font::DEFAULT })",
        );
    }
    if let Some(color) = item.options.color.as_ref().or(style.text_color.as_ref()) {
        write!(code, ".color({})", theme_color(document, color)).unwrap();
    }
    if let Some(link) = &item.options.link {
        write!(
            code,
            ".link({})",
            expr_code(link, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(background) = &item.options.background {
        write!(
            code,
            ".background({})",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    let has_border = item.options.border.is_some()
        || item.options.border_width.is_some()
        || item.options.radius.is_some()
        || item.options.radius_top_left.is_some()
        || item.options.radius_top_right.is_some()
        || item.options.radius_bottom_right.is_some()
        || item.options.radius_bottom_left.is_some();
    if has_border {
        let color = item
            .options
            .border
            .as_ref()
            .map(|color| theme_color(document, color))
            .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
        let width = item.options.border_width.as_ref().map_or_else(
            || Ok("0.0".to_owned()),
            |width| expr_code(width, env, document, ValueMode::Owned),
        )?;
        let radius = radius_code(
            item.options.radius.as_ref(),
            [
                item.options.radius_top_left.as_ref(),
                item.options.radius_top_right.as_ref(),
                item.options.radius_bottom_right.as_ref(),
                item.options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .unwrap_or_else(|| "::iced::border::Radius::default()".into());
        write!(
            code,
            ".border(::iced::Border {{ color: {color}, width: {width} as f32, radius: {radius} }})"
        )
        .unwrap();
    }
    if let Some(padding) = typed_padding_code(&item.options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(underline) = &item.options.underline {
        write!(
            code,
            ".underline({})",
            expr_code(underline, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(strikethrough) = &item.options.strikethrough {
        write!(
            code,
            ".strikethrough({})",
            expr_code(strikethrough, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(code)
}
