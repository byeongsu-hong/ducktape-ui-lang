use super::*;

pub(in crate::codegen) fn append_text_options(
    code: &mut String,
    options: &TextOptions,
    style: &Style,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(size) = &options.size {
        write!(
            code,
            ".size({} as f32)",
            expr_code(size, env, document, ValueMode::Owned)?
        )
        .unwrap();
    } else if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    if let Some(line_height) = &options.line_height {
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
    if let Some(alignment) = options.align_x {
        write!(
            code,
            ".align_x(::iced::widget::text::Alignment::{})",
            text_alignment_code(alignment)
        )
        .unwrap();
    }
    if let Some(alignment) = options.align_y {
        let alignment = match alignment {
            VerticalAlignment::Top => "Top",
            VerticalAlignment::Center => "Center",
            VerticalAlignment::Bottom => "Bottom",
        };
        write!(code, ".align_y(::iced::alignment::Vertical::{alignment})").unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".shaping(::iced::widget::text::Shaping::{})",
            text_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".wrapping(::iced::widget::text::Wrapping::{})",
            text_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = &options.font {
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
    if let Some(style) = &options.custom_style {
        let function = document
            .functions
            .iter()
            .find(|item| item.name == style.function && item.kind == ExternKind::TextStyle)
            .expect("checker validates text style");
        let args = style
            .args
            .iter()
            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?;
        write!(
            code,
            ".style(move |__theme| {}(__theme{}))",
            function.rust_path,
            args.iter()
                .map(|arg| format!(", {arg}"))
                .collect::<String>()
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn append_bool_control_options(
    code: &mut String,
    options: &BoolControlOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
    toggler: bool,
) -> Result<(), Error> {
    for (value, method) in [
        (&options.size, "size"),
        (&options.spacing, "spacing"),
        (&options.text_size, "text_size"),
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
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.line_height {
        write!(
            code,
            ".text_line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
            expr_code(height, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".text_shaping(::iced::widget::text::Shaping::{})",
            text_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".text_wrapping(::iced::widget::text::Wrapping::{})",
            text_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = &options.font {
        write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
    }
    if toggler {
        if let Some(alignment) = options.alignment {
            write!(
                code,
                ".text_alignment(::iced::widget::text::Alignment::{})",
                text_alignment_code(alignment)
            )
            .unwrap();
        }
    } else if let Some(icon) = options.icon {
        let size = options.icon_size.as_ref().map_or_else(
            || Ok("None".to_owned()),
            |value| {
                Ok::<_, Error>(format!(
                    "Some(({} as f32).into())",
                    expr_code(value, env, document, ValueMode::Owned)?
                ))
            },
        )?;
        let line_height = if let Some(value) = &options.icon_line_height {
            format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
        } else {
            "::iced::widget::text::LineHeight::default()".to_owned()
        };
        let shaping = options.icon_shaping.map_or("Auto", text_shaping_code);
        write!(
            code,
            ".icon(::iced::widget::checkbox::Icon {{ font: ::iced::Font::DEFAULT, code_point: {icon:?}, size: {size}, line_height: {line_height}, shaping: ::iced::widget::text::Shaping::{shaping} }})"
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn checkbox_style_code(
    styles: &CheckboxStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::CheckboxStyle)
                .expect("checker validates checkbox style");
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
    let preset = match styles.preset {
        CheckboxStylePreset::Primary => "primary",
        CheckboxStylePreset::Secondary => "secondary",
        CheckboxStylePreset::Success => "success",
        CheckboxStylePreset::Danger => "danger",
    };
    let overrides = [
        ("Active", true, &styles.active_checked),
        ("Active", false, &styles.active_unchecked),
        ("Hovered", true, &styles.hovered_checked),
        ("Hovered", false, &styles.hovered_unchecked),
        ("Disabled", true, &styles.disabled_checked),
        ("Disabled", false, &styles.disabled_unchecked),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(if let Some(custom) = custom {
            format!(".style(move |__theme, __status| {custom})")
        } else if styles.preset == CheckboxStylePreset::Primary {
            String::new()
        } else {
            format!(".style(::iced::widget::checkbox::{preset})")
        });
    }

    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::checkbox::{preset}(__theme, __status)"));
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, checked, style) in overrides {
        let Some(style) = style else { continue };
        write!(
            code,
            " ::iced::widget::checkbox::Status::{status} {{ is_checked: {checked} }} => {{"
        )
        .unwrap();
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.icon_color {
            write!(
                code,
                " __style.icon_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(color) = &style.text_color {
            write!(
                code,
                " __style.text_color = ::std::option::Option::Some({});",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(color) = &style.border_color {
            write!(
                code,
                " __style.border.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.border_width {
            write!(
                code,
                " __style.border.width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(radius) = radius_code(
            style.radius.as_ref(),
            [
                style.radius_top_left.as_ref(),
                style.radius_top_right.as_ref(),
                style.radius_bottom_right.as_ref(),
                style.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )? {
            write!(code, " __style.border.radius = {radius};").unwrap();
        }
        code.push_str(" }");
    }
    code.push_str(" _ => {} } __style })");
    Ok(code)
}

pub(in crate::codegen) fn toggler_style_code(
    styles: &TogglerStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::TogglerStyle)
                .expect("checker validates toggler style");
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
        ("Active", true, &styles.active_checked),
        ("Active", false, &styles.active_unchecked),
        ("Hovered", true, &styles.hovered_checked),
        ("Hovered", false, &styles.hovered_unchecked),
        ("Disabled", true, &styles.disabled_checked),
        ("Disabled", false, &styles.disabled_unchecked),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }

    let base =
        custom.unwrap_or_else(|| "::iced::widget::toggler::default(__theme, __status)".to_owned());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, checked, style) in overrides {
        let Some(style) = style else { continue };
        write!(
            code,
            " ::iced::widget::toggler::Status::{status} {{ is_toggled: {checked} }} => {{"
        )
        .unwrap();
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.background_border_color {
            write!(
                code,
                " __style.background_border_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.background_border_width {
            write!(
                code,
                " __style.background_border_width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(foreground) = &style.foreground {
            write!(
                code,
                " __style.foreground = {};",
                background_code(foreground, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.foreground_border_color {
            write!(
                code,
                " __style.foreground_border_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.foreground_border_width {
            write!(
                code,
                " __style.foreground_border_width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(color) = &style.text_color {
            write!(
                code,
                " __style.text_color = ::std::option::Option::Some({});",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(radius) = radius_code(
            style.radius.as_ref(),
            [
                style.radius_top_left.as_ref(),
                style.radius_top_right.as_ref(),
                style.radius_bottom_right.as_ref(),
                style.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )? {
            write!(
                code,
                " __style.border_radius = ::std::option::Option::Some({radius});"
            )
            .unwrap();
        }
        if let Some(ratio) = &style.padding_ratio {
            write!(
                code,
                " __style.padding_ratio = {} as f32;",
                expr_code(ratio, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        code.push_str(" }");
    }
    code.push_str(" _ => {} } __style })");
    Ok(code)
}

pub(in crate::codegen) fn radio_style_code(
    styles: &RadioStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::RadioStyle)
                .expect("checker validates radio style");
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
        ("Active", true, &styles.active_selected),
        ("Active", false, &styles.active_unselected),
        ("Hovered", true, &styles.hovered_selected),
        ("Hovered", false, &styles.hovered_unselected),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }

    let base =
        custom.unwrap_or_else(|| "::iced::widget::radio::default(__theme, __status)".to_owned());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, selected, style) in overrides {
        let Some(style) = style else { continue };
        write!(
            code,
            " ::iced::widget::radio::Status::{status} {{ is_selected: {selected} }} => {{"
        )
        .unwrap();
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.dot_color {
            write!(
                code,
                " __style.dot_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(color) = &style.border_color {
            write!(
                code,
                " __style.border_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.border_width {
            write!(
                code,
                " __style.border_width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(color) = &style.text_color {
            write!(
                code,
                " __style.text_color = ::std::option::Option::Some({});",
                theme_color(document, color)
            )
            .unwrap();
        }
        code.push_str(" }");
    }
    code.push_str(" _ => {} } __style })");
    Ok(code)
}
