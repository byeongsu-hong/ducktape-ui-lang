use super::*;

#[test]
fn lowers_complete_common_application_and_window_settings() {
    let source = r#"app Configured
  title "Configured app"
  theme "dark"
  background "123456"
  text-color "abcdef"
  id "dev.example.configured"
  executor iced::executor::Default
  font "fonts/Brand.ttf"
  font "fonts/Icons.otf"
  default-text-size 15
  antialiasing false
  vsync false
  scale-factor 1.25
  window
    icon-rgba "assets/app.rgba" 2 1
    size 960 720
    maximized true
    fullscreen false
    position 10 -20
    min-size 480 360
    max-size 1920 1080
    visible true
    resizable false
    closeable false
    minimizable false
    decorations false
    transparent true
    blur true
    level always-on-top
    exit-on-close-request false
    platform linux
      application-id "dev.example.configured"
      override-redirect true
    platform windows
      drag-and-drop false
      skip-taskbar true
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target none
state
  ready = false
extern crate::backend
  task seed() -> bool
preset ready
  state
    ready = true
  boot
    task seed() -> seeded _
on seeded(value)
  ready = value
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  text "Configured"
"#;
    let generated = compile(source, "configured.ice").unwrap();
    for expected in [
        ".title(Self::__title)",
        ".theme(Self::__theme).style(Self::__style)",
        "fn __title(&self) -> ::std::string::String",
        "\"dark\" => ::iced::Theme::Dark",
        "fn __style(&self, __theme: &::iced::Theme)",
        "parse::<::iced::Color>()",
        ".executor::<iced::executor::Default>()",
        ".presets([::iced::Preset::new(\"ready\", Self::__preset_0)])",
        "fn __preset_0()",
        "state.ready = true",
        "crate::backend::seed().map(|value| __ConfiguredMessage::Seeded(value))",
        "id: ::std::option::Option::Some(\"dev.example.configured\".to_owned())",
        ".font(include_bytes!(\"fonts/Brand.ttf\").as_slice())",
        ".font(include_bytes!(\"fonts/Icons.otf\").as_slice())",
        "default_text_size: ::iced::Pixels(15 as f32)",
        "antialiasing: false",
        "vsync: false",
        "size: ::iced::Size::new(960 as f32, 720 as f32)",
        "maximized: true",
        "fullscreen: false",
        "Position::Specific(::iced::Point::new(10 as f32, -20 as f32))",
        "min_size: ::std::option::Option::Some(::iced::Size::new(480 as f32, 360 as f32))",
        "max_size: ::std::option::Option::Some(::iced::Size::new(1920 as f32, 1080 as f32))",
        "visible: true",
        "resizable: false",
        "closeable: false",
        "minimizable: false",
        "decorations: false",
        "transparent: true",
        "blur: true",
        "level: ::iced::window::Level::AlwaysOnTop",
        "const __ICE_RGBA: &[u8] = include_bytes!(\"assets/app.rgba\")",
        "__ICE_RGBA.len() == 8",
        "window::icon::from_rgba(__ICE_RGBA.to_vec(), 2, 1)",
        "exit_on_close_request: false",
        "__platform.application_id = \"dev.example.configured\".to_owned()",
        "__platform.override_redirect = true",
        "__platform.drag_and_drop = false",
        "__platform.skip_taskbar = true",
        "__platform.undecorated_shadow = true",
        "CornerPreference::RoundSmall",
        "__platform.title_hidden = true",
        "__platform.titlebar_transparent = true",
        "__platform.fullsize_content_view = true",
        "__platform.target = ::std::option::Option::None",
        "#[cfg(target_os = \"linux\")]",
        "#[cfg(target_os = \"windows\")]",
        "#[cfg(target_os = \"macos\")]",
        "#[cfg(target_arch = \"wasm32\")]",
        ".scale_factor(Self::__scale_factor)",
        "fn __scale_factor(&self) -> f32",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }

    let error = compile(
        &source.replace("ready = true", "ready = 1"),
        "configured.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");

    for (from, to, expected) in [
        ("title \"Configured app\"", "title ready", "expected `str`"),
        ("theme \"dark\"", "theme \"unknown\"", "unknown iced theme"),
        (
            "background \"123456\"",
            "background \"not-a-color\"",
            "hexadecimal",
        ),
        ("scale-factor 1.25", "scale-factor 0", "greater than zero"),
    ] {
        let error = compile(&source.replace(from, to), "configured.ice").unwrap_err();
        assert!(error.message.contains(expected), "{error:?}");
    }
}
