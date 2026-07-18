use super::*;

pub(in crate::codegen) fn pick_list_handle_code(
    handle: &PickListHandle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match handle {
        PickListHandle::Arrow { size } => {
            let size = size.as_ref().map_or_else(
                || Ok("::std::option::Option::None".to_owned()),
                |value| {
                    Ok::<_, Error>(format!(
                        "::std::option::Option::Some(({} as f32).into())",
                        expr_code(value, env, document, ValueMode::Owned)?
                    ))
                },
            )?;
            format!("::iced::widget::pick_list::Handle::Arrow {{ size: {size} }}")
        }
        PickListHandle::Static(icon) => format!(
            "::iced::widget::pick_list::Handle::Static({})",
            pick_list_icon_code(icon, env, document)?
        ),
        PickListHandle::Dynamic { closed, open } => format!(
            "::iced::widget::pick_list::Handle::Dynamic {{ closed: {}, open: {} }}",
            pick_list_icon_code(closed, env, document)?,
            pick_list_icon_code(open, env, document)?
        ),
        PickListHandle::None => "::iced::widget::pick_list::Handle::None".to_owned(),
    })
}

pub(in crate::codegen) fn pick_list_icon_code(
    icon: &PickListIcon,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let font = icon.font.as_ref().map_or_else(
        || Ok("::iced::Font::DEFAULT".to_owned()),
        |font| font_preset_code(font, document),
    )?;
    let size = icon.size.as_ref().map_or_else(
        || Ok("::std::option::Option::None".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ))
        },
    )?;
    let line_height = icon.line_height.as_ref().map_or_else(
        || Ok("::iced::widget::text::LineHeight::default()".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ))
        },
    )?;
    let shaping = icon.shaping.map_or_else(
        || "::iced::widget::text::Shaping::default()".to_owned(),
        |shaping| {
            format!(
                "::iced::widget::text::Shaping::{}",
                text_shaping_code(shaping)
            )
        },
    );
    Ok(format!(
        "::iced::widget::pick_list::Icon {{ font: {font}, code_point: {:?}, size: {size}, line_height: {line_height}, shaping: {shaping} }}",
        icon.code_point
    ))
}

pub(in crate::codegen) fn pick_list_style_code(
    options: &PickListOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = options
        .custom_style
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::PickListStyle)
                .expect("checker validates pick-list style");
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
    let overrides = [
        ("Active", &options.style.active),
        ("Hovered", &options.style.hovered),
        ("Opened { is_hovered: false }", &options.style.opened),
        ("Opened { is_hovered: true }", &options.style.opened_hovered),
    ];
    let mut code = String::new();
    if overrides.iter().any(|(_, style)| style.is_some()) {
        let base = custom
            .unwrap_or_else(|| "::iced::widget::pick_list::default(__theme, __status)".to_owned());
        write!(
            code,
            ".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{"
        )
        .unwrap();
        for (status, style) in overrides {
            let Some(style) = style else { continue };
            write!(code, " ::iced::widget::pick_list::Status::{status} => {{").unwrap();
            append_select_surface_overrides(&mut code, &style.options, env, document, false)?;
            if let Some(color) = &style.placeholder_color {
                write!(
                    code,
                    " __style.placeholder_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(color) = &style.handle_color {
                write!(
                    code,
                    " __style.handle_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            code.push_str(" }");
        }
        code.push_str(" _ => {} } __style })");
    } else if let Some(custom) = custom {
        write!(code, ".style(move |__theme, __status| {custom})").unwrap();
    }
    code.push_str(&menu_style_code(
        options.menu_style.as_deref(),
        options.custom_menu_style.as_ref(),
        env,
        document,
    )?);
    Ok(code)
}

pub(in crate::codegen) fn menu_style_code(
    style: Option<&MenuStyleOptions>,
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::MenuStyle)
                .expect("checker validates menu style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let Some(style) = style else {
        return Ok(custom
            .map(|custom| format!(".menu_style(move |__theme| {custom})"))
            .unwrap_or_default());
    };
    let base = custom.unwrap_or_else(|| "::iced::overlay::menu::default(__theme)".to_owned());
    let mut code = String::new();
    write!(
        code,
        ".menu_style(move |__theme| {{ let mut __style = {base};"
    )
    .unwrap();
    append_select_surface_overrides(&mut code, &style.options, env, document, true)?;
    if let Some(color) = &style.selected_text_color {
        write!(
            code,
            " __style.selected_text_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(background) = &style.selected_background {
        write!(
            code,
            " __style.selected_background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(code)
}

pub(in crate::codegen) fn append_select_surface_overrides(
    code: &mut String,
    options: &ContainerStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
    shadow: bool,
) -> Result<(), Error> {
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(color) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        options.radius.as_ref(),
        [
            options.radius_top_left.as_ref(),
            options.radius_top_right.as_ref(),
            options.radius_bottom_right.as_ref(),
            options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if shadow {
        if let Some(color) = &options.shadow_color {
            write!(
                code,
                " __style.shadow.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        for (value, field) in [
            (&options.shadow_x, "__style.shadow.offset.x"),
            (&options.shadow_y, "__style.shadow.offset.y"),
            (&options.shadow_blur, "__style.shadow.blur_radius"),
        ] {
            if let Some(value) = value {
                write!(
                    code,
                    " {field} = {} as f32;",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
        }
    }
    Ok(())
}

pub(in crate::codegen) fn text_input_icon_code(
    icon: &TextInputIcon,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let font = icon.font.as_ref().map_or_else(
        || Ok("::iced::Font::DEFAULT".to_owned()),
        |font| font_preset_code(font, document),
    )?;
    let size = icon.size.as_ref().map_or_else(
        || Ok("::std::option::Option::None".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ))
        },
    )?;
    let spacing = icon.spacing.as_ref().map_or_else(
        || Ok("0.0".to_owned()),
        |value| expr_code(value, env, document, ValueMode::Owned),
    )?;
    let side = match icon.side {
        IconSide::Left => "Left",
        IconSide::Right => "Right",
    };
    Ok(format!(
        "::iced::widget::text_input::Icon {{ font: {font}, code_point: {:?}, size: {size}, spacing: {spacing} as f32, side: ::iced::widget::text_input::Side::{side} }}",
        icon.code_point
    ))
}

pub(in crate::codegen) fn text_input_style_code(
    styles: &TextInputStyleSet,
    custom: Option<&ExternCall>,
    utilities: Option<&Style>,
    env: &HashMap<String, Binding>,
    document: &Document,
    method: &str,
    widget: &str,
) -> Result<String, Error> {
    let custom_kind = if widget == "text_editor" {
        ExternKind::EditorStyle
    } else {
        ExternKind::InputStyle
    };
    let custom = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == custom_kind)
                .expect("checker validates input style");
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
    let has_utilities = utilities.is_some_and(|style| {
        style.background.is_some()
            || style.border_color.is_some()
            || style.border_width != 0
            || style.radius != 0
            || style.focus_border_color.is_some()
    });
    let overrides = [
        ("Active", &styles.active),
        ("Hovered", &styles.hovered),
        ("Focused { is_hovered: false }", &styles.focused),
        ("Focused { is_hovered: true }", &styles.focused_hovered),
        ("Disabled", &styles.disabled),
    ];
    let has_overrides = overrides.iter().any(|(_, style)| style.is_some());
    if !has_overrides && !has_utilities {
        return Ok(custom
            .map(|custom| format!(".{method}(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::{widget}::default(__theme, __status)"));
    let mut code = format!(".{method}(move |__theme, __status| {{ let mut __style = {base};");
    if let Some(style) = utilities.filter(|_| has_utilities) {
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {}.into();",
                theme_color(document, background)
            )
            .unwrap();
        }
        if let Some(border) = &style.border_color {
            write!(
                code,
                " __style.border.color = {};",
                theme_color(document, border)
            )
            .unwrap();
        }
        if style.border_width != 0 {
            write!(code, " __style.border.width = {}.0;", style.border_width).unwrap();
        }
        if style.radius != 0 {
            write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
        }
        if let Some(focus) = &style.focus_border_color {
            write!(
                code,
                " if matches!(__status, ::iced::widget::text_input::Status::Focused {{ .. }}) {{ __style.border.color = {}; }}",
                theme_color(document, focus)
            )
            .unwrap();
        }
    }
    if has_overrides {
        code.push_str(" match __status {");
        for (status, style) in overrides {
            let Some(style) = style else { continue };
            write!(code, " ::iced::widget::{widget}::Status::{status} => {{").unwrap();
            append_text_input_style_overrides(&mut code, style, env, document)?;
            code.push_str(" }");
        }
        code.push_str(" _ => {} }");
    }
    code.push_str(" __style })");
    Ok(code)
}

pub(in crate::codegen) fn append_text_input_style_overrides(
    code: &mut String,
    style: &TextInputStatusStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(background) = &style.options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &style.options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &style.options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        style.options.radius.as_ref(),
        [
            style.options.radius_top_left.as_ref(),
            style.options.radius_top_right.as_ref(),
            style.options.radius_bottom_right.as_ref(),
            style.options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    for (color, field) in [
        (&style.icon_color, "__style.icon"),
        (&style.placeholder_color, "__style.placeholder"),
        (&style.value_color, "__style.value"),
        (&style.selection_color, "__style.selection"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {};", theme_color(document, color)).unwrap();
        }
    }
    Ok(())
}
