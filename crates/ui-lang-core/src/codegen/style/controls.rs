use super::*;

pub(in crate::codegen) fn append_slider_styles(
    code: &mut String,
    styles: &SliderStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::SliderStyle)
                .expect("checker validates slider style");
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
    if styles.active.is_none() && styles.hovered.is_none() && styles.dragged.is_none() {
        if let Some(custom) = custom {
            write!(code, ".style(move |__theme, __status| {custom})").unwrap();
        }
        return Ok(());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::slider::default(__theme, __status)".to_owned());
    write!(
        code,
        ".style(move |__theme, __status| {{ let mut __style = {base};"
    )
    .unwrap();
    if let Some(active) = &styles.active {
        append_slider_style_fields(code, active, env, document)?;
    }
    if styles.hovered.is_some() || styles.dragged.is_some() {
        code.push_str(" match __status {");
        for (status, style) in [("Hovered", &styles.hovered), ("Dragged", &styles.dragged)] {
            if let Some(style) = style {
                write!(code, " ::iced::widget::slider::Status::{status} => {{").unwrap();
                append_slider_style_fields(code, style, env, document)?;
                code.push_str(" }");
            }
        }
        code.push_str(" _ => {}");
        code.push_str(" }");
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn append_slider_style_fields(
    code: &mut String,
    style: &SliderStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    for (background, field) in [
        (&style.rail_start, "__style.rail.backgrounds.0"),
        (&style.rail_end, "__style.rail.backgrounds.1"),
        (&style.handle_color, "__style.handle.background"),
    ] {
        if let Some(background) = background {
            write!(
                code,
                " {field} = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
    }
    for (color, field) in [
        (&style.rail_border_color, "__style.rail.border.color"),
        (&style.handle_border_color, "__style.handle.border_color"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {}.into();", theme_color(document, color)).unwrap();
        }
    }
    for (value, field) in [
        (&style.rail_width, "__style.rail.width"),
        (&style.rail_border_width, "__style.rail.border.width"),
        (&style.handle_border_width, "__style.handle.border_width"),
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
    if let Some(radius) = radius_code(
        style.rail_radius.as_ref(),
        [
            style.rail_radius_top_left.as_ref(),
            style.rail_radius_top_right.as_ref(),
            style.rail_radius_bottom_right.as_ref(),
            style.rail_radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.rail.border.radius = {radius};").unwrap();
    }
    if let Some(shape) = &style.handle_shape {
        let shape = match shape {
            SliderHandleShape::Circle(radius) => format!(
                "::iced::widget::slider::HandleShape::Circle {{ radius: {} as f32 }}",
                expr_code(radius, env, document, ValueMode::Owned)?
            ),
            SliderHandleShape::Rectangle { width } => {
                let radius = radius_code(
                    style.handle_radius.as_ref(),
                    [
                        style.handle_radius_top_left.as_ref(),
                        style.handle_radius_top_right.as_ref(),
                        style.handle_radius_bottom_right.as_ref(),
                        style.handle_radius_bottom_left.as_ref(),
                    ],
                    env,
                    document,
                )?
                .unwrap_or_else(|| "::iced::border::Radius::default()".to_owned());
                format!(
                    "::iced::widget::slider::HandleShape::Rectangle {{ width: {width}, border_radius: {radius} }}"
                )
            }
        };
        write!(code, " __style.handle.shape = {shape};").unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn append_tooltip_style(
    code: &mut String,
    options: &TooltipOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.custom_style.is_none()
        && options.background.is_none()
        && options.text_color.is_none()
        && options.border_color.is_none()
        && options.border_width.is_none()
        && !has_radius
        && options.shadow_color.is_none()
        && options.shadow_x.is_none()
        && options.shadow_y.is_none()
        && options.shadow_blur.is_none()
        && options.pixel_snap.is_none()
    {
        return Ok(());
    }
    if let Some(style) = &options.custom_style {
        let function = document
            .functions
            .iter()
            .find(|item| item.name == style.function && item.kind == ExternKind::ContainerStyle)
            .expect("checker validates tooltip container style");
        let args = style
            .args
            .iter()
            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?;
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = {}(__theme{});",
            function.rust_path,
            args.iter()
                .map(|arg| format!(", {arg}"))
                .collect::<String>()
        )
        .unwrap();
    } else {
        let preset = match options.style.unwrap_or(TooltipStyle::Transparent) {
            TooltipStyle::Transparent => "transparent",
            TooltipStyle::Rounded => "rounded_box",
            TooltipStyle::Bordered => "bordered_box",
            TooltipStyle::Dark => "dark",
            TooltipStyle::Primary => "primary",
            TooltipStyle::Secondary => "secondary",
            TooltipStyle::Success => "success",
            TooltipStyle::Warning => "warning",
            TooltipStyle::Danger => "danger",
        };
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = ::iced::widget::container::{preset}(__theme);"
        )
        .unwrap();
    }
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = Some({});",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(text) = &options.text_color {
        write!(
            code,
            " __style.text_color = Some({});",
            theme_color(document, text)
        )
        .unwrap();
    }
    if let Some(border) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
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
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("tooltip radius options were present");
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(shadow) = &options.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, shadow)
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
    if let Some(pixel_snap) = &options.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(pixel_snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn append_progress_options(
    code: &mut String,
    options: &ProgressOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.custom_style.is_none()
        && options.background.is_none()
        && options.bar.is_none()
        && options.border_color.is_none()
        && options.border_width.is_none()
        && !has_radius
    {
        return Ok(());
    }
    if let Some(style) = &options.custom_style {
        let function = document
            .functions
            .iter()
            .find(|item| item.name == style.function && item.kind == ExternKind::ProgressStyle)
            .expect("checker validates progress style");
        let args = style
            .args
            .iter()
            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?;
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = {}(__theme{});",
            function.rust_path,
            args.iter()
                .map(|arg| format!(", {arg}"))
                .collect::<String>()
        )
        .unwrap();
    } else {
        let preset = match options.style.unwrap_or(ProgressStyle::Primary) {
            ProgressStyle::Primary => "primary",
            ProgressStyle::Secondary => "secondary",
            ProgressStyle::Success => "success",
            ProgressStyle::Warning => "warning",
            ProgressStyle::Danger => "danger",
        };
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = ::iced::widget::progress_bar::{preset}(__theme);"
        )
        .unwrap();
    }
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(bar) = &options.bar {
        write!(
            code,
            " __style.bar = {};",
            background_code(bar, env, document)?
        )
        .unwrap();
    }
    if let Some(border) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
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
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("progress radius options were present");
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn append_rule_options(
    code: &mut String,
    options: &RuleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.fill.is_none()
        && options.color.is_none()
        && !has_radius
        && options.snap.is_none()
    {
        return Ok(());
    }
    let preset = match options.style.unwrap_or(RuleStyle::Default) {
        RuleStyle::Default => "default",
        RuleStyle::Weak => "weak",
    };
    write!(
        code,
        ".style(move |__theme| {{ let mut __style = ::iced::widget::rule::{preset}(__theme);"
    )
    .unwrap();
    if let Some(fill) = &options.fill {
        let fill = match fill {
            RuleFill::Full => "::iced::widget::rule::FillMode::Full".to_owned(),
            RuleFill::Percent(value) => format!(
                "::iced::widget::rule::FillMode::Percent({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            RuleFill::Padded(value) => {
                format!("::iced::widget::rule::FillMode::Padded({value})")
            }
            RuleFill::AsymmetricPadding(first, second) => {
                format!("::iced::widget::rule::FillMode::AsymmetricPadding({first}, {second})")
            }
        };
        write!(code, " __style.fill_mode = {fill};").unwrap();
    }
    if let Some(color) = &options.color {
        write!(code, " __style.color = {};", theme_color(document, color)).unwrap();
    }
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("rule radius options were present");
        write!(code, " __style.radius = {radius};").unwrap();
    }
    if let Some(snap) = &options.snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}
