use super::*;

#[test]
fn lowers_configured_scrollables_and_viewport_events() {
    let source = r#"app Scrolling
extern crate::backend
  scroll-style dynamic_scroll(busy:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  absolute_x = 0.0
  absolute_y = 0.0
  relative_x = 0.0
  relative_y = 0.0
on scrolled(ax, ay, rx, ry)
  absolute_x = ax
  absolute_y = ay
  relative_x = rx
  relative_y = ry
on viewport(ax, ay, reversed_x, reversed_y, rx, ry, bx, by, bw, bh, cx, cy, cw, ch)
view
  col
    scroll #feed direction=both width=fill height=200.0 bar=hidden bar-width=8.0 bar-margin=2.0 scroller-width=6.0 bar-spacing=4.0 anchor-x=end anchor-y=start auto=true scroll=scrolled style=dynamic_scroll(busy)
      text "Legacy offsets"
    scroll direction=both width=fill height=200.0 viewport=viewport style=dynamic_scroll(busy)
      col
        text "Complete viewport"
      active horizontal-disabled=false vertical-disabled=false
        container background=background text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
        horizontal-rail background=background border=primary border-width=1.0 radius=2.0
        horizontal-scroller background=primary border=foreground border-width=1.0 radius=2.0
        vertical-rail background=background border=primary border-width=1.0 radius=2.0
        vertical-scroller background=primary border=foreground border-width=1.0 radius=2.0
        gap background=background
        auto background=background border=primary border-width=1.0 radius=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 icon=foreground
      hovered horizontal-hovered=true vertical-hovered=false horizontal-disabled=false vertical-disabled=false
        horizontal-scroller background=foreground
      dragged horizontal-dragged=false vertical-dragged=true horizontal-disabled=false vertical-disabled=false
        vertical-scroller background=danger
"#;
    let generated = compile(source, "scrolling.ice").unwrap();
    assert!(generated.contains("scrollable::Direction::Both"));
    assert!(generated.contains("scrollable::Scrollbar::hidden().width(8.0 as f32)"));
    assert!(generated.contains(".anchor_x(::iced::widget::scrollable::Anchor::End)"));
    assert!(generated.contains(".auto_scroll(true)"));
    assert!(generated.contains("crate::backend::dynamic_scroll(__theme, __status, self.busy)"));
    assert!(generated.contains(
            ".style(move |__theme, __status| crate::backend::dynamic_scroll(__theme, __status, self.busy))"
        ));
    assert!(generated.contains(
            "let mut __style = crate::backend::dynamic_scroll(__theme, __status, self.busy); match __status"
        ));
    assert!(generated.contains("fn __ui_lang_check_scroll_style_dynamic_scroll"));
    assert!(generated.contains("let __absolute = __viewport.absolute_offset()"));
    assert!(generated.contains(
            "__ScrollingMessage::Scrolled(__absolute.x as f64, __absolute.y as f64, __relative.x as f64, __relative.y as f64)"
        ));
    assert!(generated.contains("absolute_offset_reversed()"));
    assert!(generated.contains("let __bounds = __viewport.bounds()"));
    assert!(generated.contains("let __content_bounds = __viewport.content_bounds()"));
    assert!(generated.contains("scrollable::Status::Hovered"));
    assert!(generated.contains("__horizontal_interaction == true"));
    assert!(generated.contains("let __style = &mut __style.container"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.horizontal_rail.scroller.background"));
    assert!(generated.contains("__style.vertical_rail.scroller.background"));
    assert!(generated.contains("__style.gap = ::std::option::Option::Some"));
    assert!(generated.contains("let __style = &mut __style.auto_scroll"));
    assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
    assert!(generated.contains("__style.auto_scroll.icon"));
    let default_scroll = compile(
        &source.replace(" style=dynamic_scroll(busy)", ""),
        "scrolling.ice",
    )
    .unwrap();
    assert!(default_scroll.contains("scrollable::default(__theme, __status)"));
}

#[test]
fn lowers_extended_text_input_behavior() {
    let source = r#"app Form
extern crate::backend
  input-style dynamic_input(disabled:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
  disabled = false
  secure = true
on submitted
on pasted(next)
  value = next
view
  input "Secret" #secret <-> value hint="Paste token" disabled=disabled secure=secure submit=submitted paste=pasted width=240.0 padding=8.0 text-size=14.0 line-height=1.2 align=center font=mono style=dynamic_input(disabled) @bg-background border border-primary rounded-lg focus:border-danger
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    icon code="•" font=ui size=12.0 spacing=4.0 side=right
"#;
    let generated = compile(source, "form.ice").unwrap();
    assert!(generated.contains(".secure(self.secure)"));
    assert!(generated.contains(".width(240.0 as f32).padding(8.0 as f32).size(14.0 as f32)"));
    assert!(generated.contains("LineHeight::Relative(1.2 as f32)"));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains("code_point: '•'"));
    assert!(generated.contains("family: ::iced::font::Family::SansSerif"));
    assert!(generated.contains("Side::Right"));
    assert!(generated.contains(".style(move |__theme, __status|"));
    assert!(generated.contains("crate::backend::dynamic_input(__theme, __status, self.disabled)"));
    assert!(generated.contains("fn __ui_lang_check_input_style_dynamic_input"));
    let custom = generated
        .find("crate::backend::dynamic_input(__theme, __status, self.disabled)")
        .unwrap();
    let utility = custom + generated[custom..].find(" __style.background =").unwrap();
    let statuses = utility + generated[utility..].find(" match __status").unwrap();
    assert!(custom < utility && utility < statuses);
    assert!(generated.contains("Status::Focused { is_hovered: true }"));
    assert!(generated.contains("__style.placeholder ="));
    assert!(generated.contains("__style.selection ="));
    assert!(generated.contains(".on_submit_maybe(if self.disabled"));
    assert!(generated.contains(".on_paste_maybe(if self.disabled"));
    let default_input = compile(
        &source.replace(" style=dynamic_input(disabled)", ""),
        "form.ice",
    )
    .unwrap();
    assert!(default_input.contains("text_input::default(__theme, __status)"));
}

#[test]
fn lowers_button_children_and_typed_properties() {
    let source = r#"app Actions
extern crate::backend
  button-style dynamic_button(disabled:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  disabled = false
on pressed
view
  button #action disabled=disabled width=fill height=48.0 padding=8.0 clip=true style=dynamic_button(disabled) @bg-primary text-white rounded-lg disabled:opacity-50 -> pressed
    row
      text "Save"
      text "⌘S"
    active background=linear(1.57, primary@0.0, background@1.0) text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
    hovered background=foreground text=background
    pressed background=primary
    disabled background=background text=foreground
"#;
    let generated = compile(source, "actions.ice").unwrap();
    assert!(generated.contains("let __button_content: ::iced::Element"));
    assert!(generated.contains("::iced::widget::row(__children)"));
    assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
    assert!(generated.contains(".padding(8.0 as f32).clip(true)"));
    assert!(generated.contains(".on_press_maybe(if self.disabled"));
    assert!(generated.contains("crate::backend::dynamic_button(__theme, __status, self.disabled)"));
    assert!(generated.contains("fn __ui_lang_check_button_style_dynamic_button"));
    assert!(generated.contains("button::Status::Active =>"));
    assert!(generated.contains("button::Status::Hovered =>"));
    assert!(generated.contains("button::Status::Pressed =>"));
    assert!(generated.contains("button::Status::Disabled =>"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.snap = true"));
    for preset in [
        "primary",
        "secondary",
        "success",
        "warning",
        "danger",
        "text",
        "background",
        "subtle",
    ] {
        let generated = compile(
            &source.replace("style=dynamic_button(disabled)", &format!("style={preset}")),
            "actions.ice",
        )
        .unwrap();
        assert!(generated.contains(&format!("button::{preset}(__theme, __status)")));
    }
}

#[test]
fn lowers_complete_boolean_control_styles_and_typography() {
    let source = r#"app Preferences
extern crate::backend
  checkbox-style dynamic_checkbox(disabled:bool)
  toggler-style dynamic_toggler(disabled:bool)
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
  col
    checkbox "Checkbox" checked=enabled style=dynamic_checkbox(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono icon="✓" icon-size=12.0 icon-line-height=1.0 icon-shaping=basic -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) icon=foreground text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0
      active unchecked background=background icon=primary text=foreground border=foreground
      hovered checked background=primary icon=foreground text=foreground border=primary
      hovered unchecked background=foreground icon=background text=primary border=primary
      disabled checked background=background icon=foreground text=foreground border=foreground
      disabled unchecked background=background icon=primary text=foreground border=primary
    toggler "Toggler" checked=enabled style=dynamic_toggler(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=auto wrapping=glyph font=default align=right -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) background-border=primary background-border-width=1.0 foreground=linear(0.0, foreground@0.0, primary@1.0) foreground-border=foreground foreground-border-width=2.0 text=foreground radius=7.0 radius-tl=6.0 radius-tr=7.0 radius-br=8.0 radius-bl=9.0 padding-ratio=0.125
      active unchecked background=background foreground=foreground text=primary
      hovered checked background=primary foreground=foreground text=foreground
      hovered unchecked background=foreground foreground=background text=primary
      disabled checked background=background foreground=foreground text=foreground
      disabled unchecked background=background foreground=primary text=foreground
"#;
    let generated = compile(source, "preferences.ice").unwrap();
    assert!(generated.contains(".size(20.0 as f32).spacing(8.0 as f32)"));
    assert!(generated.contains(".width(::iced::Fill)"));
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
    assert!(generated.contains("checkbox::Icon"));
    assert!(generated.contains("code_point: '✓'"));
    assert!(generated.contains(".text_alignment(::iced::widget::text::Alignment::Right)"));
    assert!(
        generated.contains("crate::backend::dynamic_checkbox(__theme, __status, self.enabled)")
    );
    assert!(generated.contains("fn __ui_lang_check_checkbox_style_dynamic_checkbox"));
    for (status, checked) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
        ("Disabled", true),
        ("Disabled", false),
    ] {
        assert!(generated.contains(&format!(
            "checkbox::Status::{status} {{ is_checked: {checked} }}"
        )));
    }
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.icon_color ="));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.border.width = 1.0 as f32"));
    assert!(generated.contains("top_left: 2.0 as f32"));
    for preset in ["primary", "secondary", "success", "danger"] {
        let generated = compile(
            &source.replace(
                "style=dynamic_checkbox(enabled)",
                &format!("style={preset}"),
            ),
            "preferences.ice",
        )
        .unwrap();
        assert!(generated.contains(&format!("checkbox::{preset}(__theme, __status)")));
    }
    assert!(generated.contains("crate::backend::dynamic_toggler(__theme, __status, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_toggler_style_dynamic_toggler"));
    for (status, checked) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
        ("Disabled", true),
        ("Disabled", false),
    ] {
        assert!(generated.contains(&format!(
            "toggler::Status::{status} {{ is_toggled: {checked} }}"
        )));
    }
    assert!(generated.contains("__style.background_border_width = 1.0 as f32"));
    assert!(generated.contains("__style.foreground = ::iced::Background"));
    assert!(generated.contains("__style.foreground_border_width = 2.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.border_radius = ::std::option::Option::Some"));
    assert!(generated.contains("top_left: 6.0 as f32"));
    assert!(generated.contains("__style.padding_ratio = 0.125 as f32"));
    let generated = compile(
        &source.replace(" style=dynamic_toggler(enabled)", ""),
        "preferences.ice",
    )
    .unwrap();
    assert!(generated.contains("toggler::default(__theme, __status)"));
}

#[test]
fn lowers_full_text_format() {
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
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".width(::iced::Fill).height(40.0 as f32)"));
    assert!(generated.contains("LineHeight::Absolute((20.0 as f32).into())"));
    assert!(generated.contains("text::Alignment::Justified"));
    assert!(generated.contains("alignment::Vertical::Center"));
    assert!(generated.contains("text::Shaping::Advanced"));
    assert!(generated.contains("text::Wrapping::WordOrGlyph"));
    assert!(generated.contains("..::iced::Font::MONOSPACE"));
}

#[test]
fn lowers_native_text_style_callbacks() {
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
    rich-text style=dynamic_text(active) color=foreground
      span "Rich"
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(
        generated.contains(
            "fn __ui_lang_check_text_style_dynamic_text(theme: &::iced::Theme, arg0: bool)"
        )
    );
    assert_eq!(
        generated
            .matches(".style(move |__theme| crate::backend::dynamic_text(__theme, self.active))")
            .count(),
        2
    );
    assert!(generated.contains(
        ".style(move |__theme| crate::backend::dynamic_text(__theme, self.active)).color("
    ));
}

#[test]
fn lowers_structured_rich_text_spans() {
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
    span "language" link="https://example.com" background=background @text-lg font-bold text-primary
"#;
    let generated = compile(source, "rich.ice").unwrap();
    assert!(generated.contains("::iced::widget::rich_text(__rich_spans)"));
    assert!(generated.contains("::iced::widget::span(\"Ice \".to_owned())"));
    assert!(generated.contains(".size(18.0 as f32)"));
    assert!(generated.contains("LineHeight::Absolute((22.0 as f32).into())"));
    assert!(generated.contains(".background(::iced::Background::Color("));
    assert!(generated.contains(".background(::iced::Background::from(::iced::gradient::Linear::new(1.57 as f32).add_stop(0.0 as f32"));
    assert!(generated.contains(".border(::iced::Border"));
    assert!(generated.contains(".padding(::iced::Padding"));
    assert!(generated.contains(".underline(true).strikethrough(false)"));
    assert!(generated.contains(".link(\"https://example.com\".to_owned())"));
    assert!(generated.contains(".on_link_click(move |__link| __TypographyMessage::Link(__link))"));
    assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
    assert!(generated.contains("text::Wrapping::Word"));
}

#[test]
fn lowers_declared_font_descriptors_and_app_default() {
    let source = r#"app Typography
font brand family="Inter" weight=semibold stretch=semi-expanded style=italic default=true
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Brand" font=brand @font-bold
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".default_font(::iced::Font"));
    assert!(generated.contains("Family::Name(\"Inter\")"));
    assert!(generated.contains("Weight::Semibold"));
    assert!(generated.contains("Stretch::SemiExpanded"));
    assert!(generated.contains("Style::Italic"));
    assert!(generated.contains("weight: ::iced::font::Weight::Bold, ..::iced::Font"));
}
