use super::*;

#[test]
fn lowers_complex_native_controls() {
    let source = r#"app Controls
extern crate::backend
  SliderNumber()
  sync slider_number(value:f64) -> SliderNumber
  slider-style dynamic_slider(active:bool)
  progress-style dynamic_progress(active:bool)
  radio-style dynamic_radio(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  precise:SliderNumber = slider_number(50.0)
  enabled = false
  choice = "first"
on amount_changed(next)
  amount = next
on precise_changed(next)
  precise = next
on released
on enabled_changed(next)
  enabled = next
on choice_changed(next)
  choice = next
view
  col
    grid columns=2 width=640.0 spacing=12.0 height=aspect(16.0,9.0)
      toggler "Enabled" checked=enabled -> enabled_changed _
      slider amount min=0.0 max=100.0 step=0.5 default=50.0 shift-step=0.1 vertical width=20.0 height=fill(2) style=dynamic_slider(enabled) release=released -> amount_changed _
        active rail-start=linear(0.0, primary@0.0, danger@1.0) rail-end=linear(1.57, background@0.0, primary/25@1.0) rail-width=4.0 rail-border=transparent rail-border-width=1.0 rail-radius=2.0 rail-radius-tl=1.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, foreground@1.0) handle-border=foreground handle-border-width=1.0
        hovered rail-start=foreground rail-end=background handle=rect(12) handle-color=foreground handle-radius=3.0 handle-radius-tl=1.0
        dragged rail-start=danger handle=circle(8.0) handle-color=danger
      slider amount min=0.0 max=100.0 step=1.0 width=fill height=18.0 style=dynamic_slider(enabled) -> amount_changed _
      slider precise min=slider_number(0.0) max=slider_number(100.0) step=slider_number(5.0) default=slider_number(50.0) shift-step=slider_number(1.0) -> precise_changed _
      progress amount vertical length=fill(2) girth=20.0 style=dynamic_progress(enabled) background=linear(1.57, background@0.0, primary/25@1.0) bar=linear(0.0, primary/75@0.0, danger@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0
      progress amount style=success
      progress amount style=warning
      progress amount style=danger
      radio "First" value="first" selected=(choice == "first") style=dynamic_radio(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono -> choice_changed _
        active selected background=linear(1.57, primary@0.0, background@1.0) dot=foreground border=primary border-width=2.0 text=foreground
        active unselected background=background dot=primary border=foreground text=foreground
        hovered selected background=primary dot=foreground border=foreground text=foreground
        hovered unselected background=foreground dot=background border=primary text=primary
      rule horizontal thickness=2.0 style=weak fill=full color=primary/50 radius=4.0 radius-tl=2.0 snap=false
      rule horizontal fill=percent(75.0)
      rule horizontal fill=pad(4)
      rule horizontal fill=pad(4,8)
      space width=fill(2) height=shrink
      stack clip=true width=fill(2) height=120.0 under=1
        text "base"
        text "overlay"
    grid fluid=240.0 height=fill(2)
      text "fluid"
"#;
    let generated = compile(source, "controls.ice").unwrap();
    assert!(
            generated.contains("::iced::widget::grid(__children).spacing(12.0 as f32).width(640.0 as f32).height(::iced::widget::grid::aspect_ratio(16.0 as f32, 9.0 as f32)).columns(2 as usize)")
        );
    assert!(generated.contains(
            "::iced::widget::grid(__children).height(::iced::Length::FillPortion(2)).fluid(240.0 as f32)"
        ));
    assert!(generated.contains("::iced::widget::vertical_slider"));
    assert!(generated.contains(
        ".default(50.0).shift_step(0.1).width(20.0 as f32).height(::iced::Length::FillPortion(2))"
    ));
    assert!(generated.contains("::iced::widget::slider"));
    assert!(generated.contains(".width(::iced::Fill).height(18.0 as f32)"));
    assert!(generated.contains(".style(move |__theme, __status|"));
    assert!(generated.contains("fn __ui_lang_check_slider_style_dynamic_slider"));
    assert_eq!(
        generated
            .matches("crate::backend::dynamic_slider(__theme, __status, self.enabled)")
            .count(),
        2
    );
    assert!(generated.contains(
            "::iced::widget::slider((crate::backend::slider_number(0.0))..=(crate::backend::slider_number(100.0)), self.precise, move |__value| __ControlsMessage::PreciseChanged(__value)).step(crate::backend::slider_number(5.0))"
        ));
    assert!(!generated.contains("self.precise.clone()"));
    assert!(generated.contains("slider::Status::Hovered"));
    assert!(generated.contains("slider::Status::Dragged"));
    assert!(generated.contains("slider::HandleShape::Circle"));
    assert!(generated.contains("slider::HandleShape::Rectangle"));
    assert!(generated.contains("__style.rail.backgrounds.0"));
    assert!(generated.contains("__style.rail.backgrounds.0 = ::iced::Background::from"));
    assert!(generated.contains("__style.rail.backgrounds.1 = ::iced::Background::from"));
    assert!(generated.contains("__style.handle.background = ::iced::Background::from"));
    assert!(generated.contains("::iced::widget::progress_bar"));
    assert!(generated.contains(".vertical()"));
    assert!(generated.contains(".length(::iced::Length::FillPortion(2)).girth(20.0 as f32)"));
    assert!(generated.contains("crate::backend::dynamic_progress(__theme, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_progress_style_dynamic_progress"));
    assert!(generated.contains("progress_bar::success(__theme)"));
    assert!(generated.contains("progress_bar::warning(__theme)"));
    assert!(generated.contains("progress_bar::danger(__theme)"));
    assert!(generated.contains("__style.background = ::iced::Background::from"));
    assert!(generated.contains("__style.bar = ::iced::Background::from"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("::iced::gradient::Linear::new(0.0 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("::iced::widget::radio(\"First\".to_owned(), true"));
    assert!(generated.contains("move |_| __ControlsMessage::ChoiceChanged(\"first\".to_owned())"));
    assert!(generated.contains(".size(20.0 as f32).spacing(8.0 as f32)"));
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains("crate::backend::dynamic_radio(__theme, __status, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_radio_style_dynamic_radio"));
    for (status, selected) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
    ] {
        assert!(generated.contains(&format!(
            "radio::Status::{status} {{ is_selected: {selected} }}"
        )));
    }
    assert!(generated.contains("__style.background = ::iced::Background::from"));
    assert!(generated.contains("__style.dot_color ="));
    assert!(generated.contains("__style.border_width = 2.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    let default_radio = compile(
        &source.replace(" style=dynamic_radio(enabled)", ""),
        "controls.ice",
    )
    .unwrap();
    assert!(default_radio.contains("radio::default(__theme, __status)"));
    assert!(generated.contains("::iced::widget::rule::weak(__theme)"));
    assert!(generated.contains("rule::FillMode::Full"));
    assert!(generated.contains("rule::FillMode::Percent(75.0 as f32)"));
    assert!(generated.contains("rule::FillMode::Padded(4)"));
    assert!(generated.contains("rule::FillMode::AsymmetricPadding(4, 8)"));
    assert!(generated.contains("__style.snap = false"));
    assert!(generated.contains(
        "::iced::widget::space().width(::iced::Length::FillPortion(2)).height(::iced::Shrink)"
    ));
    assert!(generated.contains("__children.split_off(__under)"));
    assert!(generated.contains("::iced::widget::Stack::new()"));
    assert!(generated.contains("__stack.push_under(__child)"));
    assert!(
        generated
            .contains(".clip(true).width(::iced::Length::FillPortion(2)).height(120.0 as f32)")
    );
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
  input "Secret" #secret <-> value hint="Paste token" disabled=disabled secure=secure submit=submitted paste=pasted width=240.0 padding=8.0 text-size=14.0 line-height=1.2 align=center font=mono style=dynamic_input(disabled)
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background border=primary border-width=1.0 radius=10.0 icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary border-width=1.0 radius=10.0
    focused-hovered background=background border=foreground border-width=1.0 radius=10.0
    disabled background=background border=primary border-width=1.0 radius=10.0 value=danger
    icon code="•" font=ui size=12.0 spacing=4.0 side=right
"#;
    let generated = compile(source, "form.ice").unwrap();
    assert!(generated.contains("let __secure = self.secure"));
    assert!(generated.contains(".secure(__secure)"));
    assert!(generated.contains("::ui_lang_runtime::Role::PasswordInput"));
    assert!(generated.contains(".value_maybe((!__secure).then"));
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
    let statuses = custom + generated[custom..].find(" match __status").unwrap();
    assert!(custom < statuses);
    assert!(generated.contains("Status::Focused { is_hovered: true }"));
    assert!(generated.contains("__style.placeholder ="));
    assert!(generated.contains("__style.selection ="));
    assert!(generated.contains(".on_submit_maybe(if __disabled"));
    assert!(generated.contains(".on_paste_maybe(if __disabled"));
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
  button #action label="Save" disabled=disabled width=fill height=48.0 padding=8.0 clip=true style=dynamic_button(disabled) @disabled:opacity-50 -> pressed
    row
      text "Save"
      text "⌘S"
    active background=linear(1.57, primary@0.0, background@1.0) text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
    hovered background=foreground text=background radius=10.0
    pressed background=primary text=white radius=10.0
    disabled background=background text=foreground radius=10.0
"#;
    let generated = compile(source, "actions.ice").unwrap();
    assert!(generated.contains("let __button_content: __IceElement"));
    assert!(generated.contains("::iced::widget::row(__children)"));
    assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
    assert!(generated.contains(".padding(8.0 as f32).clip(true)"));
    assert!(generated.contains(".on_press_maybe(if __disabled"));
    assert!(generated.contains("::ui_lang_runtime::Role::Button"));
    assert!(generated.contains(".label(\"Save\".to_owned())"));
    assert!(generated.contains("crate::backend::dynamic_button(__theme, __status, self.disabled)"));
    assert!(generated.contains("fn __ui_lang_check_button_style_dynamic_button"));
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
fn cascades_active_style_into_interaction_states() {
    let source = r#"app Styles
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on pressed
view
  button "Save" -> pressed
    active background=background text=foreground radius=8.0
    hovered text=primary
"#;
    let generated = compile(source, "styles.ice").unwrap();
    let background = generated.find("__style.background =").unwrap();
    let hovered = generated.find("button::Status::Hovered").unwrap();
    assert!(background < hovered);
    assert!(!generated.contains("button::Status::Active"));
    assert!(generated.contains("__style.text_color ="));
    assert!(generated.contains("__style.border.radius ="));
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

#[test]
fn lowers_builtin_and_opacity_text_color_utilities() {
    let source = r#"app Typography
theme
  background #000000
  foreground #ffffff
  primary #336699
  danger #ff0000
state
view
  col
    text "Invisible" @text-transparent
    text "Muted" @text-primary/50
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".color(::iced::Color::from_rgba8(0, 0, 0, 0.000000))"));
    assert!(generated.contains(".color(::iced::Color::from_rgba8(51, 102, 153, 0.500000))"));
}
