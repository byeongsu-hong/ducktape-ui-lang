use super::*;

#[test]
fn lowers_native_canvas_geometry_cache_and_events() {
    let source = r#"app Drawing
theme
  background #0f172a
  foreground #f8fafc
  primary #7c3aed
  danger #dc2626
state
  cached = true
  picture = rgba(1, 1, bytes(ff 00 ff ff))
on pressed(x, y)
on released(x, y)
on moved(x, y)
on scrolled(x, y, pixels)
on entered
on exited
view
  canvas width=fill height=240.0 cache=cached cache-group=drawings capture=true cursor=crosshair press=pressed release=released move=moved scroll=scrolled enter=entered exit=exited
    rect x=0.0 y=0.0 width=canvas_width height=canvas_height fill=linear(1.57, background@0.0, primary@1.0) stroke=foreground
    rect x=8.0 y=8.0 width=72.0 height=40.0 radius=8.0 radius-tl=4.0 stroke=foreground stroke-width=2.0 dash=(4.0, 2.0) dash-offset=1 cap=round join=bevel
    circle x=120.0 y=60.0 radius=24.0 fill=primary fill-rule=even-odd stroke=foreground
    line x1=16.0 y1=120.0 x2=200.0 y2=120.0 stroke=foreground stroke-width=3.0 cap=square
    text "Canvas" x=16.0 y=150.0 max-width=180.0 color=foreground size=18.0 line-height=1.2 font=default align-x=left align-y=top shaping=advanced
    image picture x=8.0 y=160.0 width=32.0 height=24.0 filter=nearest rotation=0.2 opacity=0.8 snap=true radius=4.0 radius-tl=2.0
    svg "<svg/>" memory x=48.0 y=160.0 width=24.0 height=24.0 color=foreground rotation=0.1 opacity=0.9
    path fill=primary stroke=foreground stroke-width=1.0
      move x=220.0 y=20.0
      line x=260.0 y=20.0
      arc x=260.0 y=40.0 radius=20.0 start=0.0 end=3.14
      arc-to ax=280.0 ay=60.0 bx=300.0 by=40.0 radius=8.0
      ellipse x=320.0 y=40.0 radius-x=20.0 radius-y=10.0 rotation=0.2 start=0.0 end=6.28
      bezier ax=340.0 ay=10.0 bx=360.0 by=70.0 x=380.0 y=40.0
      quadratic cx=400.0 cy=10.0 x=420.0 y=40.0
      rect x=220.0 y=80.0 width=30.0 height=20.0
      rounded x=260.0 y=80.0 width=30.0 height=20.0 radius=4.0
      circle x=320.0 y=90.0 radius=10.0
      close
    group x=10.0 y=10.0 rotate=0.1 scale=1.1 scale-x=1.0 scale-y=0.9 clip=(0.0, 0.0, 100.0, 100.0)
      circle x=20.0 y=20.0 radius=10.0 fill=foreground
    if cached
      circle x=360.0 y=180.0 radius=12.0 fill=primary
    for value in [12.0, 24.0]
      circle x=value y=210.0 radius=4.0 fill=foreground
"#;
    let generated = compile(source, "drawing.ice").unwrap();
    for expected in [
        "impl<State, Message, Draw, Update, Interaction> ::iced::widget::canvas::Program<Message>",
        "__state.cache.get_or_init",
        "Cache::with_group",
        "__ICE_CANVAS_GROUP_DRAWINGS",
        "::std::hash::Hash::hash",
        "::iced::widget::canvas::Path::rounded_rectangle",
        "__frame.fill_rectangle",
        "__frame.stroke_rectangle",
        "__frame.fill_text",
        "__frame.draw_image",
        "__frame.draw_svg",
        "::iced::advanced::svg::Svg",
        "__path.arc(",
        "__path.arc_to(",
        "__path.ellipse(",
        "__path.bezier_curve_to(",
        "__path.quadratic_curve_to(",
        "__frame.with_save",
        "__frame.with_clip",
        "__frame.scale_nonuniform",
        "::iced::mouse::Interaction::Crosshair",
        "::iced::widget::canvas::Action::publish",
        ".and_capture()",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_canvas_event_and_redraw_action() {
    let source = example!("canvas_events.ice");
    let generated = compile(source, "canvas_events.ice").unwrap();
    for expected in [
        "Event::InputMethod",
        "Event::Keyboard",
        "Event::Mouse",
        "Event::Touch",
        "Event::Window",
        "struct __IceKeyPress",
        "::iced::mouse::Button",
        "KeyPressed",
        "KeyReleased",
        "ModifiersChanged",
        "CursorEntered",
        "CursorLeft",
        "CursorMoved",
        "ButtonPressed",
        "ButtonReleased",
        "WheelScrolled",
        "FingerPressed",
        "FingerMoved",
        "FingerLifted",
        "FingerLost",
        "RedrawRequested",
        "CloseRequested",
        "FileHovered",
        "FileDropped",
        "FilesHoveredLeft",
        "Action::publish",
        "Action::capture",
        "Action::request_redraw()",
        "Action::request_redraw_at",
        "Duration::from_millis(16)",
        ".and_capture()",
        "move_count: i64",
        "__state.move_count =",
        "fn __ice_canvas_interaction",
        "__ice_canvas_interaction(__interaction.as_str())",
        "__cursor.is_over(__bounds)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_media_tooltip_and_pointer_events() {
    let source = r#"app Media
extern crate::backend
  svg-style dynamic_svg(active:bool)
  container-style dynamic_tooltip(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
  encoded_image = encoded(bytes(50 36 0a))
  rgba_image = rgba(1, 1, bytes(ff 00 00 ff))
on entered
on exited
on pressed
on moved(x, y)
on scrolled(x, y, pixels)
view
  col
    image "photo.ppm" width=fill height=64.0 fit=cover filter=nearest rotation=solid(0.5) opacity=0.8 scale=1.2 expand=true radius=4.0 radius-tl=1.0 radius-br=2.0 crop=(1, 2, 30, 40)
    image encoded_image
    image rgba_image
    viewer encoded_image width=fill(2) height=120.0 fit=contain filter=linear padding=8.0 min-scale=0.5 max-scale=4.0 scale-step=0.25
    viewer "photo.ppm" width=64.0 height=64.0
    svg "icon.svg" width=48.0 height=shrink fit=scale-down rotation=0.1 opacity=0.9 color=foreground hover=primary style=dynamic_svg(active)
    svg "<svg/>" memory width=16.0 color=foreground hover=none
    svg bytes(3c 73 76 67 2f 3e) memory width=16.0
    tooltip position=cursor gap=2.0 padding=5.0 delay=100 snap=false style=dynamic_tooltip(active) background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary/75 border-width=1.0 radius=5.0 radius-tl=2.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true
      mouse enter=entered exit=exited press=pressed move=moved scroll=scrolled cursor=pointer
        text "Hover"
      text "Tip"
"#;
    let generated = compile(source, "media.ice").unwrap();
    assert!(generated.contains("::iced::widget::image(\"photo.ppm\".to_owned())"));
    assert!(generated.contains(".rotation(::iced::Rotation::Solid(::iced::Radians(0.5 as f32)))"));
    assert!(generated.contains(".border_radius(::iced::border::Radius { top_left: 1.0 as f32, top_right: 4.0 as f32, bottom_right: 2.0 as f32, bottom_left: 4.0 as f32 })"));
    assert!(generated.contains("image::Handle::from_bytes(::std::vec![0x50u8, 0x36u8, 0x0au8])"));
    assert!(generated.contains("image::Handle::from_rgba((1).clamp(0, u32::MAX as i64) as u32, (1).clamp(0, u32::MAX as i64) as u32, ::std::vec![0xffu8, 0x00u8, 0x00u8, 0xffu8])"));
    assert!(generated.contains("::iced::widget::image::viewer(self.encoded_image.clone()).width(::iced::Length::FillPortion(2)).height(120.0 as f32).content_fit(::iced::ContentFit::Contain).filter_method(::iced::widget::image::FilterMethod::Linear).padding(8.0 as f32).min_scale(0.5 as f32).max_scale(4.0 as f32).scale_step(0.25 as f32)"));
    assert!(generated.contains("::iced::widget::image::viewer(::iced::widget::image::Handle::from_path(\"photo.ppm\".to_owned()))"));
    assert!(generated.contains(".crop(::iced::Rectangle { x: (1).clamp(0, u32::MAX as i64) as u32, y: (2).clamp(0, u32::MAX as i64) as u32, width: (30).clamp(0, u32::MAX as i64) as u32, height: (40).clamp(0, u32::MAX as i64) as u32 })"));
    assert!(generated.contains(".filter_method(::iced::widget::image::FilterMethod::Nearest)"));
    assert!(generated.contains("::iced::widget::svg(\"icon.svg\".to_owned())"));
    assert!(generated.contains("svg::Handle::from_memory((\"<svg/>\".to_owned()).into_bytes())"));
    assert!(generated.contains(
        "svg::Handle::from_memory(::std::vec![0x3cu8, 0x73u8, 0x76u8, 0x67u8, 0x2fu8, 0x3eu8])"
    ));
    assert!(generated.contains("crate::backend::dynamic_svg(__theme, __status, self.active)"));
    assert!(generated.contains("fn __ui_lang_check_svg_style_dynamic_svg"));
    assert!(generated.contains("svg::Status::Idle => __style.color = Some(::iced::Color"));
    assert!(generated.contains("svg::Status::Hovered => __style.color = Some(::iced::Color"));
    assert!(generated.contains("svg::Status::Hovered => __style.color = None"));
    let default_svg = compile(
        &source.replace(" style=dynamic_svg(active)", ""),
        "media.ice",
    )
    .unwrap();
    assert!(default_svg.contains("let mut __style = ::iced::widget::svg::Style::default()"));
    assert!(generated.contains("tooltip::Position::FollowCursor"));
    assert!(generated.contains(".delay(::std::time::Duration::from_millis(100 as u64))"));
    assert!(generated.contains("crate::backend::dynamic_tooltip(__theme, self.active)"));
    let preset_tooltip = compile(
        &source.replace("style=dynamic_tooltip(active)", "style=success"),
        "media.ice",
    )
    .unwrap();
    assert!(preset_tooltip.contains("container::success(__theme)"));
    assert!(generated.contains("__style.background = Some("));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.shadow.blur_radius = 8.0 as f32"));
    assert!(generated.contains("__style.snap = true"));
    assert!(generated.contains(".on_enter(__MediaMessage::Entered)"));
    assert!(generated.contains(
        ".on_move(move |__point| __MediaMessage::Moved(__point.x as f64, __point.y as f64))"
    ));
    assert!(generated.contains("::iced::mouse::ScrollDelta::Lines"));
    assert!(generated.contains("__MediaMessage::Scrolled(__x as f64, __y as f64, true)"));
    assert!(generated.contains(".interaction(::iced::mouse::Interaction::Pointer)"));
}
