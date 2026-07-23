use super::*;

#[test]
fn parses_checked_application_and_window_settings() {
    let source = SOURCE.replace(
        "app Demo",
        r##"app Demo
  title "Configured"
  theme "dark"
  bg "#123456"
  fg "#abcdef"
  id "dev.example.demo"
  executor iced::executor::Default
  renderer crate::backend::Renderer
  font "assets/Brand.ttf"
  font "assets/Icons.otf"
  text-size 15
  antialiasing false
  vsync false
  scale 1.25
  window
    icon-rgba "assets/app.rgba" 2 1
    size 960 720
    min-size 480 360
    max-size 1920 1080
    position centered
    level always-on-top
    visible true
    platform linux
      app-id "dev.example.demo"
      override-redirect false
    platform windows
      drag-and-drop true
      skip-taskbar false
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target none
  window child
    size 640 480
    position centered"##,
    );
    let document = parse(&source).unwrap();
    assert!(matches!(
        document.settings.title.as_ref().map(|setting| &setting.value),
        Some(Expr::Str(value)) if value == "Configured"
    ));
    assert_eq!(
        document.settings.executor.as_deref(),
        Some("iced::executor::Default")
    );
    assert_eq!(
        document.settings.renderer.as_deref(),
        Some("crate::backend::Renderer")
    );
    assert!(matches!(
        document
            .settings
            .scale_factor
            .as_ref()
            .map(|setting| &setting.value),
        Some(Expr::F64(value)) if *value == 1.25
    ));
    assert!(matches!(
        document.settings.theme.as_ref().map(|setting| &setting.value),
        Some(Expr::Str(value)) if value == "dark"
    ));
    assert_eq!(document.settings.fonts.len(), 2);
    assert_eq!(document.settings.fonts[0].path, "assets/Brand.ttf");
    let window = document.settings.window.unwrap();
    assert_eq!(window.size, Some((960.0, 720.0)));
    assert!(matches!(window.position, Some(WindowPosition::Centered)));
    assert!(matches!(window.level, Some(WindowLevel::AlwaysOnTop)));
    assert_eq!(
        window
            .linux
            .as_ref()
            .and_then(|settings| settings.application_id.as_deref()),
        Some("dev.example.demo")
    );
    assert!(matches!(
        window.windows.as_ref().and_then(|settings| settings.corner),
        Some(WindowCorner::RoundSmall)
    ));
    assert_eq!(
        window
            .macos
            .as_ref()
            .and_then(|settings| settings.fullsize_content_view),
        Some(true)
    );
    assert_eq!(
        window
            .wasm
            .as_ref()
            .and_then(|settings| settings.target.clone()),
        Some(None)
    );
    let icon = window.icon.unwrap();
    assert_eq!(
        (icon.path.as_str(), icon.width, icon.height, icon.byte_len),
        ("assets/app.rgba", 2, 1, 8)
    );
    assert_eq!(document.settings.windows.len(), 1);
    assert_eq!(document.settings.windows[0].name, "child");
    assert_eq!(
        document.settings.windows[0].settings.size,
        Some((640.0, 480.0))
    );

    let duplicate_window = source.replace(
        "  window child\n    size 640 480\n    position centered",
        "  window child\n    size 640 480\n    position centered\n  window child\n    size 320 240",
    );
    let error = parse(&duplicate_window).unwrap_err();
    assert_eq!(error.code, "E014");
    assert!(error.message.contains("duplicate app window"));

    let error = parse(&source.replace("min-size 480 360", "min-size 2000 360")).unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("min-size cannot exceed max-size"));

    let error = parse(&source.replace("size 960 720", "size 0 720")).unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("greater than zero"));

    let error = parse(&source.replace(
        "  antialiasing false",
        "  antialiasing false\n  antialiasing true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E014");
    assert!(error.message.contains("duplicate"));

    let duplicate_font =
        source.replace("  font \"assets/Icons.otf\"", "  font \"assets/Brand.ttf\"");
    let error = parse(&duplicate_font).unwrap_err();
    assert_eq!(error.code, "E014");
    assert!(error.message.contains("duplicate app font"));

    let error = parse(&source.replace("  font \"assets/Brand.ttf\"", "  font \"\"")).unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("relative `/` paths"));

    let error = parse(&source.replace("  font \"assets/Brand.ttf\"", "  font \"/tmp/Brand.ttf\""))
        .unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("relative `/` paths"));

    let error = parse(&source.replace(
        "icon-rgba \"assets/app.rgba\" 2 1",
        "icon-rgba \"assets/app.rgba\" 2 0",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("positive integers"));

    let error = parse(&source.replace(
        "icon-rgba \"assets/app.rgba\" 2 1",
        "icon-rgba \"assets/app.rgba\" 4294967295 2",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("dimensions are too large"));

    let error = parse(&source.replace(
        "executor iced::executor::Default",
        "executor iced::bad-path",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E073");

    let error = parse(&source.replace(
        "    platform linux\n      app-id \"dev.example.demo\"\n      override-redirect false",
        "    platform plan9\n      app-id \"dev.example.demo\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("linux, windows, macos, or wasm"));

    let error = parse(&source.replace("corner round-small", "corner softly-rounded")).unwrap_err();
    assert_eq!(error.code, "E015");
    assert!(error.message.contains("window corner"));

    let error = parse(&source.replace(
        "    platform wasm\n      target none",
        "    platform wasm\n      target none\n    platform wasm\n      target \"app\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E014");
    assert!(error.message.contains("duplicate setting `platform wasm`"));

    let error = parse(&source.replace(
        "      skip-taskbar false",
        "      skip-taskbar false\n      skip-taskbar true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E014");
    assert!(error.message.contains("duplicate setting `skip-taskbar`"));
}

#[test]
fn rejects_host_independent_absolute_asset_paths() {
    for setting in [
        "  font \"C:/tmp/brand.ttf\"",
        "  window\n    icon-rgba \"C:/tmp/app.rgba\" 1 1",
    ] {
        let source = format!("app Demo\n{setting}\nview\n  text \"ok\"\n");
        let error = parse(&source).unwrap_err();

        assert_eq!(error.code, "E015", "{setting}");
        assert!(error.message.contains("relative"), "{setting}");
    }
}

#[test]
fn rejects_redeclared_builtin_theme_colors() {
    for name in ["white", "black", "transparent"] {
        let source = format!("app Demo\ntheme\n  {name} #123456\nview\n  text \"ok\"\n");
        let error = parse(&source).unwrap_err();

        assert_eq!(error.code, "E012", "{name}");
        assert!(error.message.contains("built in"), "{name}");
    }
}

#[test]
fn rejects_redeclared_builtin_font_presets() {
    for name in ["default", "mono"] {
        let source = format!("app Demo\nfont {name}\nview\n  text \"ok\"\n");
        let error = parse(&source).unwrap_err();

        assert_eq!(error.code, "E013", "{name}");
        assert!(error.message.contains("built in"), "{name}");
    }
}

#[test]
fn parses_native_theme_factories() {
    let source = r#"extern crate::backend
  theme native_theme(dark:bool)
app Themes
  theme native_theme(dark)
theme
  bg #000000
state
  dark = true
view
  theme native_theme(!dark)
    text "Nested"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Theme);
    assert!(matches!(
        document.settings.theme.as_ref().map(|setting| &setting.value),
        Some(Expr::Call { name, .. }) if name == "native_theme"
    ));
    assert!(matches!(
        document.view,
        ViewNode::Theme {
            preset: ThemePreset::Factory(ExternCall { ref function, .. }),
            ..
        } if function == "native_theme"
    ));
}

#[test]
fn parses_alternate_theme_subtrees() {
    let source = r#"extern crate::backend
  themer alternate_panel(active:bool) -> bool
app Themes
state
  active = true
on changed(value)
  active = value
view
  themer alternate_panel(active) -> changed _
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Themer);
    assert!(matches!(
        document.view,
        ViewNode::Themer {
            ref function,
            route: Some(_),
            ..
        } if function == "alternate_panel"
    ));
}

#[test]
fn rejects_non_assignment_preset_state() {
    let source = SOURCE.replace(
        "view\n",
        "preset seeded\n  state\n    return if true\nview\n",
    );
    let error = parse(&source).unwrap_err();
    assert_eq!(error.code, "E016");
    assert!(error.message.contains("only accepts"));
}

#[test]
fn accepts_an_input_without_an_id() {
    let source = SOURCE.replace(
        "input \"Query\" #query <-> query",
        "input \"Query\" <-> query",
    );
    parse(&source).unwrap();
}

#[test]
fn parses_every_pick_list_handle() {
    for handle in [
        "handle arrow size=12.0",
        "handle static code=\"⌄\" font=default size=12.0 line-h=1.0 shape=basic",
        "handle dynamic\n      closed code=\"⌄\"\n      open code=\"⌃\"",
        "handle none",
    ] {
        let source = format!(
            r#"app Selection
state
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
view
  pick choices selected -> selected _
    active text=fg placeholder=muted handle=primary bg=surface border=border border-w=1.0 r=4.0
    hovered text=fg
    opened text=fg
    opened-hovered text=fg
    menu text=fg selected-text=fg selected-bg=primary bg=surface shadow=black shadow-y=2.0
    {handle}
"#
        );
        parse(&source).unwrap_or_else(|error| panic!("{handle}: {error:?}"));
    }
}

#[test]
fn names_missing_qr_data() {
    let source = SOURCE.replace(
        "qr docs \"https://example.com/ice docs\" correction=high version=normal(4)",
        "qr",
    );
    let error = parse(&source).unwrap_err();
    assert_eq!(error.code, "E093");
    assert!(error.message.contains("needs a name"));
}

#[test]
fn parses_editor_extension_boundaries() {
    let source = r#"app Notes
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-style editor_surface(readonly:bool)
state
  body:editor = ""
  readonly = false
  language = "rs"
on command(value)
view
  editor <-> body highlighter=editor_highlight(language) key-binding=editor_keys(readonly) style=editor_surface(readonly) -> command _
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::EditorBinding);
    assert_eq!(document.functions[1].kind, ExternKind::EditorHighlighter);
    assert_eq!(document.functions[2].kind, ExternKind::EditorStyle);
    let ViewNode::TextEditor { options, .. } = &document.view else {
        panic!("expected editor");
    };
    assert_eq!(
        options.highlighter.as_ref().unwrap().function,
        "editor_highlight"
    );
    assert_eq!(
        options.key_binding.as_ref().unwrap().function,
        "editor_keys"
    );
    assert_eq!(
        options.custom_style.as_ref().unwrap().function,
        "editor_surface"
    );
    assert!(options.key_binding_route.is_some());

    let error = parse(&source.replace(" key-binding=editor_keys(readonly)", "")).unwrap_err();
    assert!(error.message.contains("route requires key-binding"));

    let error = parse(&source.replace(" -> command _", "")).unwrap_err();
    assert!(error.message.contains("key-binding requires"));

    let error =
        parse(&source.replace(" highlighter=", " highlight=\"rs\" highlighter=")).unwrap_err();
    assert!(error.message.contains("either highlight or highlighter"));
}

#[test]
fn accepts_every_built_in_nested_theme() {
    for preset in [
        "light",
        "dark",
        "dracula",
        "nord",
        "solarized-light",
        "solarized-dark",
        "gruvbox-light",
        "gruvbox-dark",
        "catppuccin-latte",
        "catppuccin-frappe",
        "catppuccin-macchiato",
        "catppuccin-mocha",
        "tokyo-night",
        "tokyo-night-storm",
        "tokyo-night-light",
        "kanagawa-wave",
        "kanagawa-dragon",
        "kanagawa-lotus",
        "moonfly",
        "nightfly",
        "oxocarbon",
        "ferra",
    ] {
        let source = SOURCE.replace(
            "view\n  input",
            &format!("view\n  theme {preset}\n    input"),
        );
        parse(&source).unwrap_or_else(|error| panic!("{preset}: {error:?}"));
    }
}

#[test]
fn parses_first_class_accessibility_metadata() {
    let document = parse(
        r#"app Accessible
state
  name = ""
  checked = false
on press
on toggle(value)
view
  col
    input "Name" #name label="Full name" description="Profile name" <-> name
    button "Save" #save description="Save changes" -> press
    checkbox "Ready" #ready label="Ready state" description="Current readiness" checked=checked -> toggle _
    image "photo.ppm" label="Portrait" description="Profile portrait"
"#,
    )
    .unwrap();
    let ViewNode::Layout { children, .. } = &document.view else {
        panic!("expected column");
    };
    let ViewNode::Input { options, .. } = &children[0] else {
        panic!("expected input");
    };
    assert!(options.accessibility.label.is_some());
    assert!(options.accessibility.description.is_some());
    let ViewNode::Button { options, .. } = &children[1] else {
        panic!("expected button");
    };
    assert!(options.accessibility.description.is_some());
    let ViewNode::Checkbox { options, .. } = &children[2] else {
        panic!("expected checkbox");
    };
    assert!(options.accessibility.label.is_some());
    let ViewNode::Media { options, .. } = &children[3] else {
        panic!("expected image");
    };
    assert!(options.accessibility.label.is_some());
}
