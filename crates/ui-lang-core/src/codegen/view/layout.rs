use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_layout(
    kind: Layout,
    options: &LayoutOptions,
    id: &Option<Id>,
    styles: &[String],
    children: &[ViewNode],
    span: &Span,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let style = Style::parse(styles, document);
    let accessibility_key =
        accessibility_key_code(id.as_ref(), "layout", span, scope, env, document)?;
    if kind == Layout::Scroll {
        let child_scope = id.as_ref().map_or_else(
            || Ok(scope.to_owned()),
            |id| id_code(id, scope, env, document),
        )?;
        let child = render_node(&children[0], document, message, env, &child_scope, slot)?;
        let mut code = String::from("::iced::widget::scrollable(__scroll_content)");
        let scroll = options.scroll.as_ref().expect("scroll options");
        let bar = scroll_bar_code(scroll, env, document)?;
        let direction = match scroll.direction {
            ScrollDirection::Vertical => {
                format!("::iced::widget::scrollable::Direction::Vertical({bar})")
            }
            ScrollDirection::Horizontal => {
                format!("::iced::widget::scrollable::Direction::Horizontal({bar})")
            }
            ScrollDirection::Both => format!(
                "::iced::widget::scrollable::Direction::Both {{ vertical: {bar}, horizontal: {bar} }}"
            ),
        };
        write!(code, ".direction({direction})").unwrap();
        if let Some(id) = id {
            write!(
                code,
                ".id(::iced::widget::Id::from({}))",
                id_code(id, scope, env, document)?
            )
            .unwrap();
        }
        let anchor = |anchor| match anchor {
            ScrollAnchor::Start => "Start",
            ScrollAnchor::End => "End",
        };
        write!(
            code,
            ".anchor_x(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_x)
        )
        .unwrap();
        write!(
            code,
            ".anchor_y(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_y)
        )
        .unwrap();
        if let Some(auto_scroll) = &scroll.auto_scroll {
            write!(
                code,
                ".auto_scroll({})",
                expr_code(auto_scroll, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(route) = &scroll.route {
            let message_code = ordered_route_code(
                route,
                &[
                    "__absolute.x as f64",
                    "__absolute.y as f64",
                    "__relative.x as f64",
                    "__relative.y as f64",
                ],
                env,
                document,
                message,
            )?;
            write!(
                code,
                ".on_scroll(move |__viewport| {{ let __absolute = __viewport.absolute_offset(); let __relative = __viewport.relative_offset(); {message_code} }})"
            )
            .unwrap();
        } else if let Some(route) = &scroll.viewport_route {
            let message_code = ordered_route_code(
                route,
                &[
                    "__absolute.x as f64",
                    "__absolute.y as f64",
                    "__reversed.x as f64",
                    "__reversed.y as f64",
                    "__relative.x as f64",
                    "__relative.y as f64",
                    "__bounds.x as f64",
                    "__bounds.y as f64",
                    "__bounds.width as f64",
                    "__bounds.height as f64",
                    "__content_bounds.x as f64",
                    "__content_bounds.y as f64",
                    "__content_bounds.width as f64",
                    "__content_bounds.height as f64",
                ],
                env,
                document,
                message,
            )?;
            write!(
                code,
                ".on_scroll(move |__viewport| {{ let __absolute = __viewport.absolute_offset(); let __reversed = __viewport.absolute_offset_reversed(); let __relative = __viewport.relative_offset(); let __bounds = __viewport.bounds(); let __content_bounds = __viewport.content_bounds(); {message_code} }})"
            )
            .unwrap();
        }
        code.push_str(&scroll_style_code(
            &scroll.styles,
            scroll.custom_style.as_ref(),
            env,
            document,
        )?);
        append_size(&mut code, &style);
        if let Some(width) = &scroll.width {
            write!(code, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &scroll.height {
            write!(code, ".height({})", length_code(height, env, document)?).unwrap();
        }
        return Ok(format!(
            "{{ let __a11y_key = {accessibility_key}; let __scroll_content: __IceElement<'_, {message}> = {child}; let __layout = {code}; ::ui_lang_runtime::accessible(__layout, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).into() }}"
        ));
    }

    let mut body = String::from("{ let mut __children: ::std::vec::Vec<__IceElement<'_, ");
    write!(body, "{message}>> = ::std::vec::Vec::new();").unwrap();
    let child_scope = id.as_ref().map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )?;
    render_children(
        &mut body,
        children,
        document,
        message,
        env,
        &child_scope,
        slot,
    )?;
    let constructor = match kind {
        Layout::Column => "column",
        Layout::Row => "row",
        Layout::Grid => "grid",
        Layout::Stack => "stack",
        Layout::Scroll => unreachable!("scroll returned above"),
    };
    if kind == Layout::Stack && options.under > 0 {
        write!(
            body,
            " let __under = ({} as usize).min(__children.len()); let __above = __children.split_off(__under); let __layout = __above.into_iter().fold(::iced::widget::Stack::new(), |__stack, __child| __stack.push(__child)); let __layout = __children.into_iter().rev().fold(__layout, |__stack, __child| __stack.push_under(__child))",
            options.under
        )
        .unwrap();
    } else {
        write!(
            body,
            " let __layout = ::iced::widget::{constructor}(__children)"
        )
        .unwrap();
    }
    if let Some(gap) = style.gap {
        write!(body, ".spacing({gap})").unwrap();
    }
    if matches!(kind, Layout::Column | Layout::Row)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    if style.items_center {
        if kind == Layout::Column {
            body.push_str(".align_x(::iced::Center)");
        } else {
            body.push_str(".align_y(::iced::Center)");
        }
    }
    if kind == Layout::Grid {
        if let Some(spacing) = &options.spacing {
            write!(
                body,
                ".spacing({} as f32)",
                expr_code(spacing, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(width) = &options.width {
            let LengthValue::Fixed(width) = width else {
                unreachable!("grid widths are always fixed")
            };
            write!(
                body,
                ".width({} as f32)",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(height) = &options.grid_height {
            match height {
                GridSizing::AspectRatio { width, height } => write!(
                    body,
                    ".height(::iced::widget::grid::aspect_ratio({} as f32, {} as f32))",
                    expr_code(width, env, document, ValueMode::Owned)?,
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap(),
                GridSizing::EvenlyDistribute(length) => {
                    write!(body, ".height({})", length_code(length, env, document)?).unwrap();
                }
            }
        }
        if let Some(fluid) = &options.fluid {
            write!(
                body,
                ".fluid({} as f32)",
                expr_code(fluid, env, document, ValueMode::Owned)?
            )
            .unwrap();
        } else if let Some(columns) = &options.columns {
            write!(
                body,
                ".columns({} as usize)",
                expr_code(columns, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if matches!(kind, Layout::Column | Layout::Row) {
        if let Some(spacing) = &options.spacing {
            write!(
                body,
                ".spacing({} as f32)",
                expr_code(spacing, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
            write!(body, ".padding({padding})").unwrap();
        }
        if let Some(width) = &options.width {
            write!(body, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &options.height {
            write!(body, ".height({})", length_code(height, env, document)?).unwrap();
        }
        if let Some(max_width) = &options.max_width {
            write!(
                body,
                ".max_width({} as f32)",
                expr_code(max_width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(align) = options.align {
            let alignment = match (kind, align) {
                (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                (Layout::Column, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                (Layout::Column, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Vertical::Center",
                (Layout::Row, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                _ => unreachable!("only row and column reach flex alignment"),
            };
            let method = if kind == Layout::Column {
                "align_x"
            } else {
                "align_y"
            };
            write!(body, ".{method}({alignment})").unwrap();
        }
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if options.wrap {
            body.push_str(".wrap()");
            if let Some(spacing) = &options.wrap_spacing {
                let method = if kind == Layout::Column {
                    "horizontal_spacing"
                } else {
                    "vertical_spacing"
                };
                write!(
                    body,
                    ".{method}({} as f32)",
                    expr_code(spacing, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(align) = options.wrap_align {
                let alignment = match (kind, align) {
                    (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                    (Layout::Column, FlexAlignment::Center) => {
                        "::iced::alignment::Vertical::Center"
                    }
                    (Layout::Column, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                    (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                    (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                    (Layout::Row, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                    _ => unreachable!("only row and column can wrap"),
                };
                write!(body, ".align_x({alignment})").unwrap();
            }
        }
    }
    if kind == Layout::Stack {
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(width) = &options.width {
            write!(body, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &options.height {
            write!(body, ".height({})", length_code(height, env, document)?).unwrap();
        }
        append_size(&mut body, &style);
    }
    body.push(';');
    body.push_str(" let __content = ::iced::widget::container(__layout)");
    if matches!(kind, Layout::Grid | Layout::Stack)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    append_size(&mut body, &style);
    if let Some(max_width) = style.max_width {
        write!(body, ".max_width({max_width})").unwrap();
    }
    body.push_str(&container_style_code(&style, document));
    body.push(';');
    if style.self_center {
        write!(body, " let __layout_content: __IceElement<'_, {message}> = ::iced::widget::container(__content).width(::iced::Fill).center_x(::iced::Fill).into();").unwrap();
    } else {
        write!(
            body,
            " let __layout_content: __IceElement<'_, {message}> = __content.into();"
        )
        .unwrap();
    }
    write!(body, " let __a11y_key = {accessibility_key}; ::ui_lang_runtime::accessible(__layout_content, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::GenericContainer).into() }}").unwrap();
    Ok(body)
}

pub(in crate::codegen) fn scroll_bar_code(
    scroll: &ScrollOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let constructor = if scroll.hidden_bar { "hidden" } else { "new" };
    let mut code = format!("::iced::widget::scrollable::Scrollbar::{constructor}()");
    for (value, method) in [
        (&scroll.bar_width, "width"),
        (&scroll.bar_margin, "margin"),
        (&scroll.scroller_width, "scroller_width"),
        (&scroll.bar_spacing, "spacing"),
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
    Ok(code)
}

pub(in crate::codegen) fn scroll_style_code(
    styles: &[ScrollStatusStyle],
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::ScrollStyle)
                .expect("checker validates scroll style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    if styles.is_empty() {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base = custom
        .unwrap_or_else(|| "::iced::widget::scrollable::default(__theme, __status)".to_owned());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, pattern) in [
        (
            ScrollStatus::Active,
            "Active { is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ScrollStatus::Hovered,
            "Hovered { is_horizontal_scrollbar_hovered: __horizontal_interaction, is_vertical_scrollbar_hovered: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ScrollStatus::Dragged,
            "Dragged { is_horizontal_scrollbar_dragged: __horizontal_interaction, is_vertical_scrollbar_dragged: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
    ] {
        write!(code, " ::iced::widget::scrollable::Status::{pattern} => {{").unwrap();
        for style in styles.iter().filter(|style| style.status == status) {
            write!(code, " if {} {{", scroll_selector_code(style)).unwrap();
            append_scroll_status_style(&mut code, style, env, document)?;
            code.push_str(" }");
        }
        code.push_str(" }");
    }
    code.push_str(" } __style })");
    Ok(code)
}

pub(in crate::codegen) fn scroll_selector_code(style: &ScrollStatusStyle) -> String {
    let mut conditions = Vec::new();
    for (value, binding) in [
        (style.horizontal_disabled, "__horizontal_disabled"),
        (style.vertical_disabled, "__vertical_disabled"),
        (style.horizontal_interaction, "__horizontal_interaction"),
        (style.vertical_interaction, "__vertical_interaction"),
    ] {
        if let Some(value) = value {
            conditions.push(format!("{binding} == {value}"));
        }
    }
    if conditions.is_empty() {
        "true".into()
    } else {
        conditions.join(" && ")
    }
}

pub(in crate::codegen) fn append_scroll_status_style(
    code: &mut String,
    style: &ScrollStatusStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    append_scroll_surface_style(
        code,
        &style.container,
        "__style.container",
        true,
        true,
        env,
        document,
    )?;
    for (rail, target) in [
        (&style.horizontal_rail, "__style.horizontal_rail"),
        (&style.vertical_rail, "__style.vertical_rail"),
    ] {
        append_scroll_surface_style(code, &rail.rail, target, true, false, env, document)?;
        append_scroll_surface_style(
            code,
            &rail.scroller,
            &format!("{target}.scroller"),
            false,
            false,
            env,
            document,
        )?;
    }
    if let Some(gap) = &style.gap {
        write!(
            code,
            " __style.gap = ::std::option::Option::Some({});",
            background_code(gap, env, document)?
        )
        .unwrap();
    }
    append_scroll_surface_style(
        code,
        &style.auto_scroll,
        "__style.auto_scroll",
        false,
        false,
        env,
        document,
    )?;
    if let Some(color) = &style.auto_scroll_icon {
        write!(
            code,
            " __style.auto_scroll.icon = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn append_scroll_surface_style(
    code: &mut String,
    options: &ContainerStyleOptions,
    target: &str,
    optional_background: bool,
    text: bool,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let mut options = options.clone();
    if !optional_background && let Some(background) = options.background.take() {
        write!(
            code,
            " {target}.background = {};",
            background_code(&background, env, document)?
        )
        .unwrap();
    }
    write!(code, " {{ let __style = &mut {target};").unwrap();
    append_surface_style_overrides(code, &options, env, document)?;
    if text && let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, color)
        )
        .unwrap();
    }
    code.push_str(" }");
    Ok(())
}
