use super::*;

#[derive(Default)]
pub(in crate::codegen) struct Style {
    pub(in crate::codegen) width_fill: bool,
    pub(in crate::codegen) height_fill: bool,
    pub(in crate::codegen) max_width: Option<u16>,
    pub(in crate::codegen) padding: [u16; 4],
    pub(in crate::codegen) gap: Option<u16>,
    pub(in crate::codegen) items_center: bool,
    pub(in crate::codegen) self_center: bool,
    pub(in crate::codegen) text_size: Option<u16>,
    pub(in crate::codegen) bold: bool,
    pub(in crate::codegen) text_color: Option<String>,
    pub(in crate::codegen) background: Option<String>,
    pub(in crate::codegen) hover_background: Option<String>,
    pub(in crate::codegen) pressed_background: Option<String>,
    pub(in crate::codegen) border_color: Option<String>,
    pub(in crate::codegen) focus_border_color: Option<String>,
    pub(in crate::codegen) border_width: u16,
    pub(in crate::codegen) radius: u16,
    pub(in crate::codegen) disabled_opacity: Option<f32>,
}

impl Style {
    pub(in crate::codegen) fn parse(tokens: &[String], document: &Document) -> Self {
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

    pub(in crate::codegen) fn padding_code(&self) -> Option<String> {
        (self.padding != [0; 4]).then(|| {
            format!(
                "::iced::Padding {{ top: {}.0, right: {}.0, bottom: {}.0, left: {}.0 }}",
                self.padding[0], self.padding[1], self.padding[2], self.padding[3]
            )
        })
    }
}

pub(in crate::codegen) fn append_size(code: &mut String, style: &Style) {
    if style.width_fill {
        code.push_str(".width(::iced::Fill)");
    }
    if style.height_fill {
        code.push_str(".height(::iced::Fill)");
    }
}

pub(in crate::codegen) fn container_style_code(style: &Style, document: &Document) -> String {
    container_style_value(style, document)
        .map(|style| format!(".style(|_| {style})"))
        .unwrap_or_default()
}

pub(in crate::codegen) fn container_style_value(
    style: &Style,
    document: &Document,
) -> Option<String> {
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

pub(in crate::codegen) fn button_style_code(
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

pub(in crate::codegen) fn theme_color(document: &Document, token: &str) -> String {
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

pub(in crate::codegen) fn theme_preset_code(
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

pub(in crate::codegen) fn theme_factory_code(
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

pub(in crate::codegen) fn qr_data_code(qr: &QrData) -> String {
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

pub(in crate::codegen) fn color_code(value: &str, opacity: Option<u8>) -> String {
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

pub(in crate::codegen) fn spacing(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().map(|value| value * 4)
}

pub(in crate::codegen) fn rust_string(value: &str) -> String {
    format!("{value:?}")
}

pub(in crate::codegen) fn rust_f64(value: f64) -> String {
    format!("{value:?}")
}

pub(in crate::codegen) fn pascal(value: &str) -> String {
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
