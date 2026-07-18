use super::*;

#[test]
fn checks_text_format_options_and_rejects_zero_line_height() {
    let source = r#"app Typography
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Long text" width=fill height=40.0 size=16.0 line-height-px=20.0 font=mono align-x=justified align-y=center shaping=advanced wrapping=word-or-glyph @font-bold
"#;
    analyze(source).unwrap();

    let invalid = source.replace("line-height-px=20.0", "line-height=0.0");
    let error = analyze(&invalid).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("text line height"));
}

#[test]
fn checks_native_text_style_callbacks() {
    let source = r#"app Typography
extern crate::backend
  text-style dynamic_text(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  col
    text "Styled" style=dynamic_text(active)
    rich-text style=dynamic_text(active)
      span "Rich"
"#;
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("dynamic_text(active)", "missing_text(active)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("text style"));

    let error = analyze(&source.replace("dynamic_text(active)", "dynamic_text(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error =
        analyze(&source.replacen("style=dynamic_text(active)", "style=primary", 1)).unwrap_err();
    assert_eq!(error.code, "E063");

    let rich_only = source.replacen("style=dynamic_text(active)", "", 1);
    let error =
        analyze(&rich_only.replace("style=dynamic_text(active)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E186");
}

#[test]
fn checks_structured_rich_text_spans() {
    let source = r#"app Typography
font ui family=sans weight=medium stretch=normal style=normal
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
on link(url)
view
  rich-text width=fill height=48.0 size=16.0 line-height=1.2 font=ui align-x=justified align-y=center wrapping=word color=foreground @font-bold -> link _
    span "Ice " size=18.0 line-height-px=22.0 font=ui color=primary background=linear(1.57, background@0.0, primary@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 padding=2.0 padding-left=4.0 underline strike=false
    span "language" link="https://example.com" @text-lg font-bold text-primary
"#;
    analyze(source).unwrap();

    let bad_text = source.replace("span \"Ice \"", "span [\"bad\"]");
    let error = analyze(&bad_text).unwrap_err();
    assert_eq!(error.code, "E186");
    assert!(error.message.contains("span text"));

    let bad_link = source.replace("link=\"https://example.com\"", "link=1");
    let error = analyze(&bad_link).unwrap_err();
    assert_eq!(error.code, "E101");

    let missing_route = source.replace(" @font-bold -> link _", " @font-bold");
    let error = analyze(&missing_route).unwrap_err();
    assert_eq!(error.code, "E186");
    assert!(error.message.contains("require `-> handler _`"));

    let bad_padding = source.replace("padding-left=4.0", "padding-left=-1.0");
    let error = analyze(&bad_padding).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("span padding"));

    let bad_background = source.replace("primary@1.0", "missing@1.0");
    let error = analyze(&bad_background).unwrap_err();
    assert_eq!(error.code, "E186");
    assert!(error.message.contains("missing"));
}

#[test]
fn checks_complete_font_descriptors_and_references() {
    let source = r#"app Typography
font thin family="Inter" weight=thin stretch=ultra-condensed style=normal default=true
font extra_light family=serif weight=extra-light stretch=extra-condensed style=italic
font light family=sans weight=light stretch=condensed style=oblique
font normal family=cursive weight=normal stretch=semi-condensed style=normal
font medium family=fantasy weight=medium stretch=normal style=normal
font semibold family=mono weight=semibold stretch=semi-expanded style=normal
font bold weight=bold stretch=expanded style=normal
font extra_bold weight=extra-bold stretch=extra-expanded style=normal
font black weight=black stretch=ultra-expanded style=normal
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Fonts" font=black
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.fonts.len(), 9);

    let error = analyze(&source.replace("font=black", "font=missing")).unwrap_err();
    assert_eq!(error.code, "E114");
    assert!(error.message.contains("missing"));

    let error = analyze(&source.replace(
        "font extra_light family=serif",
        "font extra_light family=serif default=true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E114");
    assert!(error.message.contains("only one"));
}

#[test]
fn rejects_checkbox_icon_options_without_icon() {
    let source = r#"app Preferences
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  enabled = false
on changed(next)
  enabled = next
view
  checkbox "Checkbox" checked=enabled icon-size=12.0 -> changed _
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("checkbox icon properties"));
}

#[test]
fn names_an_undeclared_extern_type() {
    let source = r#"app Demo
extern crate::backend
  load() -> [Missing]
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  text "hello"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("`Missing`"));
}

#[test]
fn requires_a_route_for_an_emitting_extern_component() {
    let source = r#"app Demo
extern crate::backend
  component native_control() -> bool
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  extern native_control()
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E126");
    assert!(error.message.contains("requires a route"));
}

#[test]
fn checks_native_shader_programs() {
    let source = r#"app Demo
extern crate::backend
  shader native_shader(value:f64) -> bool
  shader passive_shader() -> unit
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 1.0
on shaded(active)
view
  col
    shader native_shader(amount) width=fill height=64.0 -> shaded _
    shader passive_shader()
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace(" -> shaded _", "")).unwrap_err();
    assert_eq!(error.code, "E191");
    assert!(error.message.contains("requires a route"));

    let error = analyze(&source.replace("height=64.0", "height=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("shader size"));

    let error =
        analyze(&source.replace("native_shader(amount)", "native_shader(true)")).unwrap_err();
    assert!(error.message.contains("expected `f64`"));

    let error = analyze(&source.replace("height=64.0", "depth=64.0")).unwrap_err();
    assert_eq!(error.code, "E191");
    assert!(error.message.contains("unknown shader property"));

    let error = analyze(&source.replace(
        "shader native_shader(value:f64) -> bool",
        "shader native_shader(value:f64) -> bool ! bool",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E023");
}

#[test]
fn rejects_state_capture_in_subscription_routes() {
    let source = r#"app Demo
extern crate::backend
  subscription events() -> bool
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  count = 1
on event(count, next)
subscribe
  events() -> event(count, _)
view
  text count
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E127");
}
