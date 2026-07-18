use super::*;

pub(super) fn length_code(
    length: &LengthValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match length {
        LengthValue::Fill => "::iced::Fill".into(),
        LengthValue::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        LengthValue::Shrink => "::iced::Shrink".into(),
        LengthValue::Fixed(value) => format!(
            "{} as f32",
            expr_code(value, env, document, ValueMode::Owned)?
        ),
    })
}

pub(super) fn typed_padding_code(
    padding: &PaddingOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let code = |value: Option<&Expr>| {
        value
            .map(|value| expr_code(value, env, document, ValueMode::Owned))
            .transpose()
    };
    let all = code(padding.all.as_ref())?.unwrap_or_else(|| "0.0".into());
    let x = code(padding.x.as_ref())?.unwrap_or_else(|| all.clone());
    let y = code(padding.y.as_ref())?.unwrap_or_else(|| all.clone());
    let top = code(padding.top.as_ref())?.unwrap_or_else(|| y.clone());
    let right = code(padding.right.as_ref())?.unwrap_or_else(|| x.clone());
    let bottom = code(padding.bottom.as_ref())?.unwrap_or(y);
    let left = code(padding.left.as_ref())?.unwrap_or(x);
    Ok(Some(format!(
        "::iced::Padding {{ top: {top} as f32, right: {right} as f32, bottom: {bottom} as f32, left: {left} as f32 }}"
    )))
}

pub(super) fn radius_code(
    uniform: Option<&Expr>,
    corners: [Option<&Expr>; 4],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if uniform.is_none() && corners.iter().all(Option::is_none) {
        return Ok(None);
    }
    let base = uniform
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "0.0".to_owned());
    let mut values = Vec::with_capacity(4);
    for corner in corners {
        values.push(
            corner
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| base.clone()),
        );
    }
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {} as f32, top_right: {} as f32, bottom_right: {} as f32, bottom_left: {} as f32 }}",
        values[0], values[1], values[2], values[3]
    )))
}

pub(super) fn append_float_style(
    code: &mut String,
    style: &FloatStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let radius = radius_code(
        style.radius.as_ref(),
        [
            style.radius_top_left.as_ref(),
            style.radius_top_right.as_ref(),
            style.radius_bottom_right.as_ref(),
            style.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )?;
    if style.shadow_color.is_none()
        && style.shadow_x.is_none()
        && style.shadow_y.is_none()
        && style.shadow_blur.is_none()
        && radius.is_none()
    {
        return Ok(());
    }
    code.push_str(".style(move |_| { let mut __style = ::iced::widget::float::Style::default();");
    if let Some(color) = &style.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    for (value, field) in [
        (&style.shadow_x, "__style.shadow.offset.x"),
        (&style.shadow_y, "__style.shadow.offset.y"),
        (&style.shadow_blur, "__style.shadow.blur_radius"),
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
    if let Some(radius) = radius {
        write!(code, " __style.shadow_border_radius = {radius};").unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

pub(super) fn background_code(
    background: &BackgroundValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    match background {
        BackgroundValue::Color(color) => Ok(format!(
            "::iced::Background::Color({})",
            theme_color(document, color)
        )),
        BackgroundValue::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                expr_code(angle, env, document, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    expr_code(&stop.offset, env, document, ValueMode::Owned)?,
                    theme_color(document, &stop.color)
                )
                .unwrap();
            }
            code.push(')');
            Ok(code)
        }
    }
}

pub(super) fn container_surface_style_value(
    utilities: &Style,
    options: &ContainerStyleOptions,
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    let has_typed_style = options.background.is_some()
        || options.text_color.is_some()
        || options.border_color.is_some()
        || options.border_width.is_some()
        || options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some()
        || options.shadow_color.is_some()
        || options.shadow_x.is_some()
        || options.shadow_y.is_some()
        || options.shadow_blur.is_some()
        || options.pixel_snap.is_some();
    let utility_style = container_style_value(utilities, document);
    let custom_style = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::ContainerStyle)
                .expect("checker validates container style");
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
    if !has_typed_style && custom_style.is_none() {
        return Ok(utility_style);
    }
    if !has_typed_style && utility_style.is_none() {
        return Ok(custom_style);
    }

    let has_custom_style = custom_style.is_some();
    let base = custom_style
        .or_else(|| utility_style.clone())
        .unwrap_or_else(|| "::iced::widget::container::Style::default()".into());
    let mut code = format!("{{ let mut __style = {base};");
    if has_custom_style {
        append_container_utility_overrides(&mut code, utilities, document);
    }
    append_surface_style_overrides(&mut code, options, env, document)?;
    if let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, color)
        )
        .unwrap();
    }
    code.push_str(" __style }");
    Ok(Some(code))
}

pub(super) fn append_container_utility_overrides(
    code: &mut String,
    style: &Style,
    document: &Document,
) {
    if let Some(background) = &style.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({}.into());",
            theme_color(document, background)
        )
        .unwrap();
    }
    if let Some(text) = &style.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, text)
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
}

pub(super) fn append_surface_style_overrides(
    code: &mut String,
    options: &ContainerStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({});",
            background_code(background, env, document)?
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
    if let Some(color) = &options.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(x) = &options.shadow_x {
        write!(
            code,
            " __style.shadow.offset.x = {} as f32;",
            expr_code(x, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(y) = &options.shadow_y {
        write!(
            code,
            " __style.shadow.offset.y = {} as f32;",
            expr_code(y, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(blur) = &options.shadow_blur {
        write!(
            code,
            " __style.shadow.blur_radius = {} as f32;",
            expr_code(blur, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(snap) = &options.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(())
}

pub(super) fn append_slider_styles(
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
    let complete = styles.active.is_some() && styles.hovered.is_some() && styles.dragged.is_some();
    let base =
        custom.unwrap_or_else(|| "::iced::widget::slider::default(__theme, __status)".to_owned());
    write!(
        code,
        ".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{"
    )
    .unwrap();
    for (status, style) in [
        ("Active", &styles.active),
        ("Hovered", &styles.hovered),
        ("Dragged", &styles.dragged),
    ] {
        if let Some(style) = style {
            write!(code, " ::iced::widget::slider::Status::{status} => {{").unwrap();
            append_slider_style_fields(code, style, env, document)?;
            code.push_str(" }");
        }
    }
    if !complete {
        code.push_str(" _ => {}");
    }
    code.push_str(" } __style })");
    Ok(())
}

pub(super) fn append_slider_style_fields(
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

pub(super) fn append_tooltip_style(
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

pub(super) fn append_progress_options(
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

pub(super) fn append_rule_options(
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

pub(super) fn append_text_options(
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

pub(super) fn append_bool_control_options(
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

pub(super) fn checkbox_style_code(
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

pub(super) fn toggler_style_code(
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

pub(super) fn radio_style_code(
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

pub(super) fn pick_list_handle_code(
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

pub(super) fn pick_list_icon_code(
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

pub(super) fn pick_list_style_code(
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

pub(super) fn menu_style_code(
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

pub(super) fn append_select_surface_overrides(
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

pub(super) fn text_input_icon_code(
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

pub(super) fn text_input_style_code(
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

pub(super) fn append_text_input_style_overrides(
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

pub(super) fn text_shaping_code(shaping: TextShaping) -> &'static str {
    match shaping {
        TextShaping::Auto => "Auto",
        TextShaping::Basic => "Basic",
        TextShaping::Advanced => "Advanced",
    }
}

pub(super) fn text_wrapping_code(wrapping: TextWrapping) -> &'static str {
    match wrapping {
        TextWrapping::None => "None",
        TextWrapping::Word => "Word",
        TextWrapping::Glyph => "Glyph",
        TextWrapping::WordOrGlyph => "WordOrGlyph",
    }
}

pub(super) fn font_preset_code(font: &FontPreset, document: &Document) -> Result<String, Error> {
    match font {
        FontPreset::Default => Ok("::iced::Font::DEFAULT".into()),
        FontPreset::Monospace => Ok("::iced::Font::MONOSPACE".into()),
        FontPreset::Named(name) => document
            .fonts
            .iter()
            .find(|font| font.name == *name)
            .map(font_decl_code)
            .ok_or_else(|| Error::new("E171", &Span::line(1), format!("unknown font `{name}`"))),
    }
}

pub(super) fn font_decl_code(font: &FontDecl) -> String {
    let family = match &font.family {
        FontFamily::Named(name) => format!("::iced::font::Family::Name({})", rust_string(name)),
        FontFamily::Serif => "::iced::font::Family::Serif".into(),
        FontFamily::SansSerif => "::iced::font::Family::SansSerif".into(),
        FontFamily::Cursive => "::iced::font::Family::Cursive".into(),
        FontFamily::Fantasy => "::iced::font::Family::Fantasy".into(),
        FontFamily::Monospace => "::iced::font::Family::Monospace".into(),
    };
    let weight = match font.weight {
        FontWeight::Thin => "Thin",
        FontWeight::ExtraLight => "ExtraLight",
        FontWeight::Light => "Light",
        FontWeight::Normal => "Normal",
        FontWeight::Medium => "Medium",
        FontWeight::Semibold => "Semibold",
        FontWeight::Bold => "Bold",
        FontWeight::ExtraBold => "ExtraBold",
        FontWeight::Black => "Black",
    };
    let stretch = match font.stretch {
        FontStretch::UltraCondensed => "UltraCondensed",
        FontStretch::ExtraCondensed => "ExtraCondensed",
        FontStretch::Condensed => "Condensed",
        FontStretch::SemiCondensed => "SemiCondensed",
        FontStretch::Normal => "Normal",
        FontStretch::SemiExpanded => "SemiExpanded",
        FontStretch::Expanded => "Expanded",
        FontStretch::ExtraExpanded => "ExtraExpanded",
        FontStretch::UltraExpanded => "UltraExpanded",
    };
    let style = match font.style {
        FontStyle::Normal => "Normal",
        FontStyle::Italic => "Italic",
        FontStyle::Oblique => "Oblique",
    };
    format!(
        "::iced::Font {{ family: {family}, weight: ::iced::font::Weight::{weight}, stretch: ::iced::font::Stretch::{stretch}, style: ::iced::font::Style::{style} }}"
    )
}

pub(super) fn text_alignment_code(alignment: TextAlignment) -> &'static str {
    match alignment {
        TextAlignment::Default => "Default",
        TextAlignment::Left => "Left",
        TextAlignment::Center => "Center",
        TextAlignment::Right => "Right",
        TextAlignment::Justified => "Justified",
    }
}

pub(super) fn mouse_interaction_code(interaction: MouseInteraction) -> &'static str {
    match interaction {
        MouseInteraction::None => "None",
        MouseInteraction::Hidden => "Hidden",
        MouseInteraction::Idle => "Idle",
        MouseInteraction::ContextMenu => "ContextMenu",
        MouseInteraction::Help => "Help",
        MouseInteraction::Pointer => "Pointer",
        MouseInteraction::Progress => "Progress",
        MouseInteraction::Wait => "Wait",
        MouseInteraction::Cell => "Cell",
        MouseInteraction::Crosshair => "Crosshair",
        MouseInteraction::Text => "Text",
        MouseInteraction::Alias => "Alias",
        MouseInteraction::Copy => "Copy",
        MouseInteraction::Move => "Move",
        MouseInteraction::NoDrop => "NoDrop",
        MouseInteraction::NotAllowed => "NotAllowed",
        MouseInteraction::Grab => "Grab",
        MouseInteraction::Grabbing => "Grabbing",
        MouseInteraction::ResizingHorizontally => "ResizingHorizontally",
        MouseInteraction::ResizingVertically => "ResizingVertically",
        MouseInteraction::ResizingDiagonallyUp => "ResizingDiagonallyUp",
        MouseInteraction::ResizingDiagonallyDown => "ResizingDiagonallyDown",
        MouseInteraction::ResizingColumn => "ResizingColumn",
        MouseInteraction::ResizingRow => "ResizingRow",
        MouseInteraction::AllScroll => "AllScroll",
        MouseInteraction::ZoomIn => "ZoomIn",
        MouseInteraction::ZoomOut => "ZoomOut",
    }
}

pub(super) fn binding_variant(binding: &str) -> String {
    format!("__Bind{}", pascal(binding))
}

pub(super) fn editor_variant(binding: &str) -> String {
    format!("__Edit{}", pascal(binding))
}

pub(super) fn controlled_state_name(
    code: &str,
    widget: &str,
    span: &Span,
) -> Result<String, Error> {
    let Some(name) = code.strip_prefix("self.") else {
        return Err(Error::new(
            "E139",
            span,
            format!("{widget} binding must resolve to an app state"),
        ));
    };
    if name.contains('.') {
        return Err(Error::new(
            "E139",
            span,
            format!("{widget} binding must resolve to one app state"),
        ));
    }
    Ok(name.to_owned())
}

pub(super) fn id_code(
    id: &Id,
    scope: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    if let Some(key) = &id.key {
        Ok(format!(
            "format!(\"{{}}/{}({{}})\", {scope}, {})",
            id.name,
            expr_code(key, env, document, ValueMode::Borrowed)?
        ))
    } else {
        Ok(format!("format!(\"{{}}/{}\", {scope})", id.name))
    }
}

pub(super) fn widget_target_code(
    target: &WidgetTarget,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    if target.segments.iter().all(|segment| segment.key.is_none()) {
        return Ok(format!(
            "::iced::widget::Id::new({})",
            rust_string(&format!(
                "{}/{}",
                document.app,
                target
                    .segments
                    .iter()
                    .map(|segment| segment.name.as_str())
                    .collect::<Vec<_>>()
                    .join("/")
            ))
        ));
    }
    let mut scope = rust_string(&document.app);
    for segment in &target.segments {
        scope = id_code(segment, &scope, env, document)?;
    }
    Ok(format!("::iced::widget::Id::from({scope})"))
}

pub(super) fn widget_selector_code(
    selector: &WidgetSelector,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(String, Option<&'static str>), Error> {
    match selector {
        WidgetSelector::Id(target) => Ok((
            format!(
                "::iced::widget::selector::id({})",
                widget_target_code(target, env, document)?
            ),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Text(value) => Ok((
            expr_code(value, env, document, ValueMode::Owned)?,
            Some("__ice_widget_target_from_text"),
        )),
        WidgetSelector::Point { x, y } => Ok((
            format!(
                "::iced::Point::new(({}) as f32, ({}) as f32)",
                expr_code(x, env, document, ValueMode::Owned)?,
                expr_code(y, env, document, ValueMode::Owned)?
            ),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Focused => Ok((
            "::iced::widget::selector::is_focused()".into(),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Extern { function, args } => {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Selector)
                .expect("checker validates selectors");
            Ok((
                format!(
                    "{}({})",
                    function.rust_path,
                    args.iter()
                        .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ")
                ),
                None,
            ))
        }
    }
}

#[derive(Default)]
pub(super) struct Style {
    pub(super) width_fill: bool,
    pub(super) height_fill: bool,
    pub(super) max_width: Option<u16>,
    pub(super) padding: [u16; 4],
    pub(super) gap: Option<u16>,
    pub(super) items_center: bool,
    pub(super) self_center: bool,
    pub(super) text_size: Option<u16>,
    pub(super) bold: bool,
    pub(super) text_color: Option<String>,
    pub(super) background: Option<String>,
    pub(super) hover_background: Option<String>,
    pub(super) pressed_background: Option<String>,
    pub(super) border_color: Option<String>,
    pub(super) focus_border_color: Option<String>,
    pub(super) border_width: u16,
    pub(super) radius: u16,
    pub(super) disabled_opacity: Option<f32>,
}

impl Style {
    pub(super) fn parse(tokens: &[String], document: &Document) -> Self {
        let mut style = Self::default();
        for token in tokens {
            let (variant, utility) = token
                .split_once(':')
                .map_or((None, token.as_str()), |(a, b)| (Some(a), b));
            if variant == Some("hover") && utility.starts_with("bg-") {
                style.hover_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("pressed") && utility.starts_with("bg-") {
                style.pressed_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("focus") && utility.starts_with("border-") {
                style.focus_border_color = Some(utility[7..].into());
                continue;
            }
            if variant == Some("disabled") && utility.starts_with("opacity-") {
                style.disabled_opacity =
                    utility[8..].parse::<f32>().ok().map(|value| value / 100.0);
                continue;
            }
            if variant.is_some() {
                continue;
            }
            match utility {
                "w-full" => style.width_fill = true,
                "h-full" => style.height_fill = true,
                "max-w-sm" => style.max_width = Some(384),
                "max-w-md" => style.max_width = Some(448),
                "max-w-lg" => style.max_width = Some(512),
                "max-w-xl" => style.max_width = Some(576),
                "max-w-2xl" => style.max_width = Some(672),
                "items-center" => style.items_center = true,
                "self-center" => style.self_center = true,
                "text-xs" => style.text_size = Some(12),
                "text-sm" => style.text_size = Some(14),
                "text-base" => style.text_size = Some(16),
                "text-lg" => style.text_size = Some(18),
                "text-xl" => style.text_size = Some(20),
                "text-2xl" => style.text_size = Some(24),
                "font-bold" => style.bold = true,
                "border" => style.border_width = 1,
                "border-2" => style.border_width = 2,
                "rounded-sm" => style.radius = 2,
                "rounded" | "rounded-md" => style.radius = 6,
                "rounded-lg" => style.radius = 10,
                "rounded-full" => style.radius = 999,
                _ if utility.starts_with("gap-") => style.gap = spacing(&utility[4..]),
                _ if utility.starts_with("p-") => {
                    if let Some(value) = spacing(&utility[2..]) {
                        style.padding = [value; 4];
                    }
                }
                _ if utility.starts_with("px-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[1] = value;
                        style.padding[3] = value;
                    }
                }
                _ if utility.starts_with("py-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[0] = value;
                        style.padding[2] = value;
                    }
                }
                _ if utility.starts_with("bg-") => style.background = Some(utility[3..].into()),
                _ if utility.starts_with("text-") && document.theme.contains_key(&utility[5..])
                    || matches!(utility, "text-white" | "text-black") =>
                {
                    style.text_color = Some(utility[5..].into())
                }
                _ if utility.starts_with("border-") => {
                    style.border_color = Some(utility[7..].into())
                }
                _ => {}
            }
        }
        style
    }

    pub(super) fn padding_code(&self) -> Option<String> {
        (self.padding != [0; 4]).then(|| {
            format!(
                "::iced::Padding {{ top: {}.0, right: {}.0, bottom: {}.0, left: {}.0 }}",
                self.padding[0], self.padding[1], self.padding[2], self.padding[3]
            )
        })
    }
}

pub(super) fn append_size(code: &mut String, style: &Style) {
    if style.width_fill {
        code.push_str(".width(::iced::Fill)");
    }
    if style.height_fill {
        code.push_str(".height(::iced::Fill)");
    }
}

pub(super) fn container_style_code(style: &Style, document: &Document) -> String {
    container_style_value(style, document)
        .map(|style| format!(".style(|_| {style})"))
        .unwrap_or_default()
}

pub(super) fn container_style_value(style: &Style, document: &Document) -> Option<String> {
    if style.background.is_none() && style.border_width == 0 && style.text_color.is_none() {
        return None;
    }
    let background = style
        .background
        .as_ref()
        .map(|color| format!("Some({}.into())", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let text = style
        .text_color
        .as_ref()
        .map(|color| format!("Some({})", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let border = style
        .border_color
        .as_ref()
        .map(|color| theme_color(document, color))
        .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
    Some(format!(
        "::iced::widget::container::Style {{ background: {background}, text_color: {text}, border: ::iced::Border {{ color: {border}, width: {}.0, radius: {}.0.into() }}, ..::iced::widget::container::Style::default() }}",
        style.border_width, style.radius
    ))
}

pub(super) fn button_style_code(
    style: &Style,
    typed: &ButtonStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let has_utilities = style.background.is_some()
        || style.hover_background.is_some()
        || style.pressed_background.is_some()
        || style.text_color.is_some()
        || style.radius != 0
        || style.disabled_opacity.is_some();
    let has_typed = typed.active.is_some()
        || typed.hovered.is_some()
        || typed.pressed.is_some()
        || typed.disabled.is_some();
    let custom = typed
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::ButtonStyle)
                .expect("checker validates button style");
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
    let preset = match typed.preset {
        ButtonStylePreset::Primary => "primary",
        ButtonStylePreset::Secondary => "secondary",
        ButtonStylePreset::Success => "success",
        ButtonStylePreset::Warning => "warning",
        ButtonStylePreset::Danger => "danger",
        ButtonStylePreset::Text => "text",
        ButtonStylePreset::Background => "background",
        ButtonStylePreset::Subtle => "subtle",
    };
    if !has_utilities && !has_typed {
        return Ok(if let Some(custom) = custom {
            format!(".style(move |__theme, __status| {custom})")
        } else if typed.preset == ButtonStylePreset::Primary {
            String::new()
        } else {
            format!(".style(::iced::widget::button::{preset})")
        });
    }

    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::button::{preset}(__theme, __status)"));
    let mut code = format!(".style(move |__theme, __status| {{ let mut __style = {base};");
    if has_utilities {
        let normal = style
            .background
            .as_ref()
            .map(|color| theme_color(document, color));
        let hover = style
            .hover_background
            .as_ref()
            .map(|color| theme_color(document, color))
            .or_else(|| normal.clone());
        let pressed = style
            .pressed_background
            .as_ref()
            .map(|color| theme_color(document, color))
            .or_else(|| hover.clone())
            .or_else(|| normal.clone());
        let option = |color: Option<String>| {
            color.map_or_else(|| "None".into(), |color| format!("Some({color})"))
        };
        write!(
            code,
            " let __background: Option<::iced::Color> = match __status {{ ::iced::widget::button::Status::Hovered => {}, ::iced::widget::button::Status::Pressed => {}, ::iced::widget::button::Status::Disabled => {}, _ => {} }}; if let Some(__background) = __background {{ __style.background = Some(::iced::Background::Color(__background)); }}",
            option(hover),
            option(pressed),
            option(normal.clone()),
            option(normal),
        )
        .unwrap();
        if let Some(text) = &style.text_color {
            write!(
                code,
                " __style.text_color = {};",
                theme_color(document, text)
            )
            .unwrap();
        }
        if style.radius > 0 {
            write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
        }
        if style.background.is_some()
            || style.text_color.is_some()
            || style.disabled_opacity.is_some()
        {
            let disabled = style.disabled_opacity.unwrap_or(0.5);
            write!(code, " if matches!(__status, ::iced::widget::button::Status::Disabled) {{ __style.text_color.a *= {disabled}; if let Some(::iced::Background::Color(mut __color)) = __style.background {{ __color.a *= {disabled}; __style.background = Some(::iced::Background::Color(__color)); }} }}").unwrap();
        }
    }
    if has_typed {
        code.push_str(" match __status {");
        for (variant, status) in [
            ("Active", &typed.active),
            ("Hovered", &typed.hovered),
            ("Pressed", &typed.pressed),
            ("Disabled", &typed.disabled),
        ] {
            write!(code, " ::iced::widget::button::Status::{variant} => {{").unwrap();
            if let Some(status) = status {
                append_surface_style_overrides(&mut code, &status.options, env, document)?;
                if let Some(color) = &status.options.text_color {
                    write!(
                        code,
                        " __style.text_color = {};",
                        theme_color(document, color)
                    )
                    .unwrap();
                }
            }
            code.push_str(" }");
        }
        code.push_str(" }");
    }
    code.push_str(" __style })");
    Ok(code)
}

pub(super) fn theme_color(document: &Document, token: &str) -> String {
    let (name, opacity) = token
        .split_once('/')
        .map_or((token, None), |(name, opacity)| {
            (name, opacity.parse::<u8>().ok())
        });
    let value = match name {
        "white" => "#ffffff",
        "black" => "#000000",
        "transparent" => "#00000000",
        name => document
            .theme
            .get(name)
            .map(String::as_str)
            .unwrap_or("#000000"),
    };
    color_code(value, opacity)
}

pub(super) fn theme_preset_code(
    preset: &ThemePreset,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match preset {
        ThemePreset::Default => "::std::option::Option::None".into(),
        ThemePreset::App => "::std::option::Option::Some(Self::__app_theme())".into(),
        ThemePreset::BuiltIn(name) => format!(
            "::std::option::Option::Some(::iced::Theme::{})",
            pascal(name)
        ),
        ThemePreset::Factory(factory) => format!(
            "::std::option::Option::Some({})",
            theme_factory_code(&factory.function, &factory.args, env, document)?
        ),
    })
}

pub(super) fn theme_factory_code(
    name: &str,
    args: &[Expr],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let function = document
        .functions
        .iter()
        .find(|function| function.name == name && function.kind == ExternKind::Theme)
        .expect("checker validates theme factories");
    let args = args
        .iter()
        .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!("{}({args})", function.rust_path))
}

pub(super) fn qr_data_code(qr: &QrData) -> String {
    let module = "::iced::widget::qr_code";
    let data = match &qr.data {
        QrPayload::Text(value) => rust_string(value),
        QrPayload::Bytes(values) => format!(
            "&[{}][..]",
            values
                .iter()
                .map(|value| format!("0x{value:02x}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let correction = |value| match value {
        QrCorrection::Low => format!("{module}::ErrorCorrection::Low"),
        QrCorrection::Medium => format!("{module}::ErrorCorrection::Medium"),
        QrCorrection::Quartile => format!("{module}::ErrorCorrection::Quartile"),
        QrCorrection::High => format!("{module}::ErrorCorrection::High"),
    };
    let constructor = if let Some(version) = qr.version {
        let version = match version {
            QrVersion::Normal(value) => format!("{module}::Version::Normal({value})"),
            QrVersion::Micro(value) => format!("{module}::Version::Micro({value})"),
        };
        let correction = correction(qr.correction.unwrap_or(QrCorrection::Medium));
        format!("{module}::Data::with_version({data}, {version}, {correction})")
    } else if let Some(value) = qr.correction {
        format!(
            "{module}::Data::with_error_correction({data}, {})",
            correction(value)
        )
    } else {
        format!("{module}::Data::new({data})")
    };
    format!("{constructor}.expect(\"invalid qr data `{}`\")", qr.name)
}

pub(super) fn color_code(value: &str, opacity: Option<u8>) -> String {
    let hex = value.trim_start_matches('#');
    let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).unwrap_or(0);
    let alpha = opacity
        .map(|value| value as f32 / 100.0)
        .or_else(|| (hex.len() == 8).then(|| byte(6..8) as f32 / 255.0))
        .unwrap_or(1.0);
    format!(
        "::iced::Color::from_rgba8({}, {}, {}, {alpha:.6})",
        byte(0..2),
        byte(2..4),
        byte(4..6)
    )
}

pub(super) fn spacing(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().map(|value| value * 4)
}

pub(super) fn rust_string(value: &str) -> String {
    format!("{value:?}")
}

pub(super) fn rust_f64(value: f64) -> String {
    format!("{value:?}")
}

pub(super) fn pascal(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect()
}
