use super::*;

pub(in crate::codegen) fn has_animations(document: &Document) -> bool {
    document
        .states
        .iter()
        .any(|state| matches!(state.ty, Type::Animation(_)))
}

pub(in crate::codegen) fn font_assets_code(settings: &AppSettings, source_path: &str) -> String {
    let parent = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    settings
        .fonts
        .iter()
        .map(|font| {
            format!(
                ".font(include_bytes!({}).as_slice())",
                rust_string(&parent.join(&font.path).display().to_string())
            )
        })
        .collect()
}

pub(in crate::codegen) fn app_settings_code(settings: &AppSettings) -> String {
    let mut fields = String::new();
    if let Some(id) = &settings.id {
        write!(
            fields,
            "id: ::std::option::Option::Some({}.to_owned()),",
            rust_string(id)
        )
        .unwrap();
    }
    if let Some(size) = settings.default_text_size {
        write!(fields, "default_text_size: ::iced::Pixels({size} as f32),").unwrap();
    }
    if let Some(value) = settings.antialiasing {
        write!(fields, "antialiasing: {value},").unwrap();
    }
    if let Some(value) = settings.vsync {
        write!(fields, "vsync: {value},").unwrap();
    }
    if fields.is_empty() {
        String::new()
    } else {
        format!(".settings(::iced::Settings {{ {fields} ..::std::default::Default::default() }})")
    }
}

pub(in crate::codegen) fn window_settings_code(
    settings: Option<&WindowSettings>,
    source_path: &str,
) -> String {
    let settings = settings.map_or_else(
        || "::iced::window::Settings::default()".to_owned(),
        |settings| window_settings_value_code(settings, source_path),
    );
    format!(
        ".window({{ let mut __window = {settings}; #[cfg(target_os = \"windows\")] {{ __window.visible = false; __window.maximized = false; __window.fullscreen = false; }} __window }})"
    )
}

pub(in crate::codegen) fn generate_named_windows(
    out: &mut String,
    document: &Document,
    source_path: &str,
) {
    for (index, window) in document.settings.windows.iter().enumerate() {
        writeln!(
            out,
            "fn __window_{index}() -> ::iced::window::Settings {{ {} }}",
            window_settings_value_code(&window.settings, source_path)
        )
        .unwrap();
    }
}

pub(in crate::codegen) fn window_settings_value_code(
    settings: &WindowSettings,
    source_path: &str,
) -> String {
    let mut fields = String::new();
    let size =
        |(width, height): (f64, f64)| format!("::iced::Size::new({width} as f32, {height} as f32)");
    if let Some(value) = settings.size {
        write!(fields, "size: {},", size(value)).unwrap();
    }
    for (name, value) in [
        ("maximized", settings.maximized),
        ("fullscreen", settings.fullscreen),
        ("visible", settings.visible),
        ("resizable", settings.resizable),
        ("closeable", settings.closeable),
        ("minimizable", settings.minimizable),
        ("decorations", settings.decorations),
        ("transparent", settings.transparent),
        ("blur", settings.blur),
        ("exit_on_close_request", settings.exit_on_close_request),
    ] {
        if let Some(value) = value {
            write!(fields, "{name}: {value},").unwrap();
        }
    }
    if let Some(position) = settings.position {
        let position = match position {
            WindowPosition::Default => "::iced::window::Position::Default".into(),
            WindowPosition::Centered => "::iced::window::Position::Centered".into(),
            WindowPosition::Specific(x, y) => format!(
                "::iced::window::Position::Specific(::iced::Point::new({x} as f32, {y} as f32))"
            ),
        };
        write!(fields, "position: {position},").unwrap();
    }
    if let Some(value) = settings.min_size {
        write!(
            fields,
            "min_size: ::std::option::Option::Some({}),",
            size(value)
        )
        .unwrap();
    }
    if let Some(value) = settings.max_size {
        write!(
            fields,
            "max_size: ::std::option::Option::Some({}),",
            size(value)
        )
        .unwrap();
    }
    if let Some(level) = settings.level {
        let level = match level {
            WindowLevel::Normal => "Normal",
            WindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
            WindowLevel::AlwaysOnTop => "AlwaysOnTop",
        };
        write!(fields, "level: ::iced::window::Level::{level},").unwrap();
    }
    if let Some(icon) = &settings.icon {
        let parent = Path::new(source_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let path = parent.join(&icon.path).display().to_string();
        write!(
            fields,
            "icon: ::std::option::Option::Some({{ const __ICE_RGBA: &[u8] = include_bytes!({}); const _: () = ::std::assert!(__ICE_RGBA.len() == {}, \"window icon RGBA byte length does not match width × height × 4\"); ::iced::window::icon::from_rgba(__ICE_RGBA.to_vec(), {}, {}).expect(\"statically checked RGBA window icon\") }}),",
            rust_string(&path),
            icon.byte_len,
            icon.width,
            icon.height
        )
        .unwrap();
    }
    if settings.linux.is_some()
        || settings.windows.is_some()
        || settings.macos.is_some()
        || settings.wasm.is_some()
    {
        write!(
            fields,
            "platform_specific: {},",
            window_platform_code(settings)
        )
        .unwrap();
    }
    format!("::iced::window::Settings {{ {fields} ..::std::default::Default::default() }}")
}

pub(in crate::codegen) fn window_platform_code(settings: &WindowSettings) -> String {
    let mut linux = String::new();
    if let Some(settings) = &settings.linux {
        if let Some(value) = &settings.application_id {
            write!(
                linux,
                "__platform.application_id = {}.to_owned();",
                rust_string(value)
            )
            .unwrap();
        }
        if let Some(value) = settings.override_redirect {
            write!(linux, "__platform.override_redirect = {value};").unwrap();
        }
    }

    let mut windows = String::new();
    if let Some(settings) = &settings.windows {
        for (name, value) in [
            ("drag_and_drop", settings.drag_and_drop),
            ("skip_taskbar", settings.skip_taskbar),
            ("undecorated_shadow", settings.undecorated_shadow),
        ] {
            if let Some(value) = value {
                write!(windows, "__platform.{name} = {value};").unwrap();
            }
        }
        if let Some(value) = settings.corner {
            let value = match value {
                WindowCorner::Default => "Default",
                WindowCorner::DoNotRound => "DoNotRound",
                WindowCorner::Round => "Round",
                WindowCorner::RoundSmall => "RoundSmall",
            };
            write!(
                windows,
                "__platform.corner_preference = ::iced::window::settings::platform::CornerPreference::{value};"
            )
            .unwrap();
        }
    }

    let mut macos = String::new();
    if let Some(settings) = &settings.macos {
        for (name, value) in [
            ("title_hidden", settings.title_hidden),
            ("titlebar_transparent", settings.titlebar_transparent),
            ("fullsize_content_view", settings.fullsize_content_view),
        ] {
            if let Some(value) = value {
                write!(macos, "__platform.{name} = {value};").unwrap();
            }
        }
    }

    let mut wasm = String::new();
    if let Some(Some(target)) = settings
        .wasm
        .as_ref()
        .and_then(|settings| settings.target.as_ref())
    {
        write!(
            wasm,
            "__platform.target = ::std::option::Option::Some({}.to_owned());",
            rust_string(target)
        )
        .unwrap();
    } else if settings
        .wasm
        .as_ref()
        .is_some_and(|settings| settings.target == Some(None))
    {
        wasm.push_str("__platform.target = ::std::option::Option::None;");
    }

    format!(
        "{{ #[cfg(target_os = \"linux\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {linux} __platform }} #[cfg(target_os = \"windows\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {windows} __platform }} #[cfg(target_os = \"macos\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {macos} __platform }} #[cfg(target_arch = \"wasm32\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {wasm} __platform }} #[cfg(not(any(target_os = \"linux\", target_os = \"windows\", target_os = \"macos\", target_arch = \"wasm32\")))] {{ ::std::default::Default::default() }} }}"
    )
}
