use super::*;

#[test]
fn rejects_invalid_rule_style_values() {
    let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  rule horizontal fill=percent(101.0)
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("rule percent"));

    let unknown_color = source.replace("fill=percent(101.0)", "color=missing");
    let error = analyze(&unknown_color).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("unknown rule color"));
}

#[test]
fn checks_slider_options_and_rejects_invalid_ranges() {
    let source = r#"app Controls
extern crate::backend
  SliderNumber()
  sync slider_number(value:f64) -> SliderNumber
  slider-style dynamic_slider(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  precise:SliderNumber = slider_number(50.0)
  active = true
on changed(next)
  amount = next
on precise_changed(next)
  precise = next
view
  col
    slider amount min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 width=fill(2) height=20.0 style=dynamic_slider(active) -> changed _
      active rail-start=linear(0.0, primary@0.0, danger@1.0) rail-end=linear(1.57, background@0.0, primary/25@1.0) rail-width=4.0 rail-border=transparent rail-border-width=1.0 rail-radius=2.0 rail-radius-tl=1.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, foreground@1.0) handle-border=foreground handle-border-width=1.0
      hovered rail-start=foreground rail-end=background rail-radius-tr=3.0 rail-radius-br=3.0 rail-radius-bl=2.0 handle=rect(12) handle-color=foreground handle-radius=3.0 handle-radius-tl=1.0 handle-radius-tr=2.0 handle-radius-br=3.0 handle-radius-bl=4.0
      dragged rail-start=danger handle=circle(8.0) handle-color=danger
    slider amount min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 vertical width=20.0 height=fill -> changed _
    slider precise min=slider_number(0.0) max=slider_number(100.0) step=slider_number(5.0) default=slider_number(50.0) shift-step=slider_number(1.0) -> precise_changed _
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty.display(), "SliderNumber");

    let bad_step = source.replace("step=5.0", "step=0.0");
    let error = analyze(&bad_step).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("slider step"));

    let bad_axis = source.replace("vertical width=20.0", "vertical width=fill");
    let error = analyze(&bad_axis).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("slider width must be fixed"));

    let bad_range = source.replace("min=0.0 max=100.0", "min=101.0 max=100.0");
    let error = analyze(&bad_range).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("min cannot exceed max"));

    let bad_color = source.replace("danger@1.0", "missing@1.0");
    let error = analyze(&bad_color).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("unknown slider rail start color"));

    let bad_metric = source.replace("rail-width=4.0", "rail-width=-1.0");
    let error = analyze(&bad_metric).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("slider rail width"));

    let bad_handle = source.replace("handle=rect(12)", "handle=circle(7.0)");
    let error = analyze(&bad_handle).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("requires `handle=rect"));

    let error =
        analyze(&source.replace("dynamic_slider(active)", "missing_slider(active)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("slider style"));

    let error =
        analyze(&source.replace("dynamic_slider(active)", "dynamic_slider(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error =
        analyze(&source.replace("style=dynamic_slider(active)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E076");

    let error = analyze(&source.replace("step=slider_number(5.0)", "step=5.0")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("amount = 50.0", "amount = 50")).unwrap_err();
    assert_eq!(error.code, "E125");
    assert!(error.message.contains("extern numeric type"));
}

#[test]
fn checks_progress_options_and_rejects_invalid_style() {
    let source = r#"app Controls
extern crate::backend
  progress-style dynamic_progress(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  active = true
view
  col
    progress amount min=0.0 max=100.0 length=fill(2) girth=20.0 style=dynamic_progress(active) background=linear(1.57, background@0.0, primary/25@1.0) bar=linear(0.0, primary/75@0.0, danger@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=4.0 radius-bl=5.0
    progress amount vertical length=120.0 girth=fill style=warning
"#;
    analyze(source).unwrap();

    let bad_range = source.replace("min=0.0 max=100.0", "min=101.0 max=100.0");
    let error = analyze(&bad_range).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("progress min cannot exceed max"));

    let bad_color = source.replace("danger@1.0", "missing@1.0");
    let error = analyze(&bad_color).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("unknown progress bar color"));

    let bad_radius = source.replace("radius=4.0", "radius=-1.0");
    let error = analyze(&bad_radius).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("progress radius"));

    let unknown = source.replace("dynamic_progress(active)", "missing(active)");
    let error = analyze(&unknown).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("progress style"));

    let wrong_arg = source.replace("dynamic_progress(active)", "dynamic_progress(amount)");
    let error = analyze(&wrong_arg).unwrap_err();
    assert_eq!(error.code, "E101");

    let malformed = source.replace("dynamic_progress(active)", "unknown");
    let error = analyze(&malformed).unwrap_err();
    assert_eq!(error.code, "E077");
}

#[test]
fn checks_tooltip_style_and_rejects_invalid_values() {
    let source = r#"app Hints
extern crate::backend
  container-style tooltip_surface(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  tooltip position=bottom style=tooltip_surface(active) background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary/75 border-width=1.0 radius=5.0 radius-tl=2.0 radius-tr=3.0 radius-br=4.0 radius-bl=5.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true
    text "Hover"
    text "Tip"
"#;
    analyze(source).unwrap();

    let bad_color = source.replace("shadow=black/50", "shadow=missing");
    let error = analyze(&bad_color).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("unknown tooltip color"));

    let bad_background = source.replace("primary/25@1.0", "missing@1.0");
    let error = analyze(&bad_background).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("unknown tooltip background color"));

    let bad_blur = source.replace("shadow-blur=8.0", "shadow-blur=-1.0");
    let error = analyze(&bad_blur).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("tooltip shadow blur"));

    analyze(&source.replace("style=tooltip_surface(active)", "style=rounded")).unwrap();

    let unknown = source.replace("tooltip_surface(active)", "missing(active)");
    let error = analyze(&unknown).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("container style"));

    let wrong_arg = source.replace("tooltip_surface(active)", "tooltip_surface(1)");
    let error = analyze(&wrong_arg).unwrap_err();
    assert_eq!(error.code, "E101");

    let bad_style = source.replace("style=tooltip_surface(active)", "style=unknown");
    let error = analyze(&bad_style).unwrap_err();
    assert_eq!(error.code, "E086");
    assert!(error.message.contains("declared container style call"));
}

#[test]
fn rejects_a_negative_space_length() {
    let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  space width=-1.0
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("space length"));

    let invalid_portion = source.replace("width=-1.0", "width=fill(65536)");
    let error = analyze(&invalid_portion).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("fill portion"));
}

#[test]
fn rejects_a_non_positive_responsive_breakpoint() {
    let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  responsive at=0.0
    text "Narrow"
    text "Wide"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("responsive breakpoint"));
}

#[test]
fn infers_mouse_move_and_scroll_payloads() {
    let source = r#"app Pointer
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  x = 0.0
  y = 0.0
  pixels = false
on moved(next_x, next_y)
  x = next_x
  y = next_y
on scrolled(delta_x, delta_y, pixel_units)
  x = delta_x
  y = delta_y
  pixels = pixel_units
view
  mouse move=moved scroll=scrolled cursor=crosshair
    text "Track me"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[0].params[0].ty.display(), "f64");
    assert_eq!(document.handlers[0].params[1].ty.display(), "f64");
    assert_eq!(document.handlers[1].params[0].ty.display(), "f64");
    assert_eq!(document.handlers[1].params[2].ty.display(), "bool");
}

#[test]
fn rejects_wrong_mouse_move_arity() {
    let source = r#"app Pointer
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on moved(x)
view
  mouse move=moved(_)
    text "Track me"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("mouse move"));
}

#[test]
fn checks_scrollable_configuration_and_offsets() {
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
    let document = analyze(source).unwrap();
    for param in &document.handlers[0].params {
        assert_eq!(param.ty.display(), "f64");
    }
    assert_eq!(document.handlers[1].params.len(), 14);
    for param in &document.handlers[1].params {
        assert_eq!(param.ty.display(), "f64");
    }

    let error = analyze(&source.replace("horizontal-hovered=true", "horizontal-hovered=maybe"))
        .unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("true or false"));

    let error = analyze(&source.replace(
        "auto=true scroll=scrolled",
        "auto=true scroll=scrolled viewport=viewport",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("either scroll= or viewport="));

    let error = analyze(&source.replace("dynamic_scroll(busy)", "missing(busy)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("scroll style"));

    let error =
        analyze(&source.replace("dynamic_scroll(busy)", "dynamic_scroll(absolute_x)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error =
        analyze(&source.replace("style=dynamic_scroll(busy)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("declared style call"));
}

#[test]
fn rejects_negative_scrollbar_size() {
    let source = r#"app Scrolling
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  scroll bar-width=-1.0
    text "Scrollable"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("scroll bar width"));
}

#[test]
fn checks_extended_text_input_routes_and_properties() {
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
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    icon code="•" font=ui size=12.0 spacing=4.0 side=right
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty.display(), "str");

    let error =
        analyze(&source.replace("dynamic_input(disabled)", "missing(disabled)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("input style"));

    let error =
        analyze(&source.replace("dynamic_input(disabled)", "dynamic_input(value)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error =
        analyze(&source.replace("style=dynamic_input(disabled)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E065");
    assert!(error.message.contains("declared style call"));
}

#[test]
fn rejects_input_icon_options_without_an_icon() {
    let source = r#"app Form
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
view
  input "Value" <-> value icon-size=12.0
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("require `icon="));
}

#[test]
fn rejects_negative_input_icon_spacing() {
    let source = r#"app Form
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
view
  input "Value" <-> value
    icon code="+" spacing=-1.0
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("input icon spacing"));
}

#[test]
fn checks_button_child_and_typed_properties() {
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
  button #action disabled=disabled width=fill height=48.0 padding=8.0 clip=true style=dynamic_button(disabled) -> pressed
    row
      text "Save"
      text "⌘S"
    active background=linear(1.57, primary@0.0, background@1.0) text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
    hovered background=foreground text=background
    pressed background=primary
    disabled background=background text=foreground
"#;
    analyze(source).unwrap();

    let bad_color = source.replace("border=primary", "border=missing");
    let error = analyze(&bad_color).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("missing"));

    let bad_preset = source.replace("style=dynamic_button(disabled)", "style=tertiary");
    let error = analyze(&bad_preset).unwrap_err();
    assert_eq!(error.code, "E066");
    assert!(error.message.contains("button style must be"));

    let unknown = source.replace("dynamic_button(disabled)", "missing(disabled)");
    let error = analyze(&unknown).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("button style"));

    let wrong_arg = source.replace("dynamic_button(disabled)", "dynamic_button(1.0)");
    let error = analyze(&wrong_arg).unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn rejects_button_label_and_child_together() {
    let source = r#"app Actions
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on pressed
view
  button "Save" -> pressed
    text "Duplicate"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E066");
    assert!(error.message.contains("not both"));
}

#[test]
fn checks_complete_boolean_control_styles_and_typography() {
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
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("border=primary border-width", "border=missing border-width"))
            .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("checkbox border color `missing`"));

    let error = analyze(&source.replace("border-width=1.0", "border-width=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("checkbox style metric"));

    let error =
        analyze(&source.replace("style=dynamic_checkbox(enabled)", "style=warning")).unwrap_err();
    assert_eq!(error.code, "E067");
    assert!(error.message.contains("checkbox style must be"));

    let error = analyze(&source.replace("dynamic_checkbox(enabled)", "missing_checkbox(enabled)"))
        .unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("checkbox style"));

    let error =
        analyze(&source.replace("dynamic_checkbox(enabled)", "dynamic_checkbox(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error =
        analyze(&source.replace("style=dynamic_toggler(enabled)", "style=default")).unwrap_err();
    assert_eq!(error.code, "E075");
    assert!(error.message.contains("toggler style must be"));

    let error = analyze(&source.replace("dynamic_toggler(enabled)", "missing_toggler(enabled)"))
        .unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("toggler style"));

    let error =
        analyze(&source.replace("dynamic_toggler(enabled)", "dynamic_toggler(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
        "      active unchecked background=background",
        "      active checked background=background\n      active unchecked background=background",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E067");
    assert!(error.message.contains("duplicate checkbox active checked"));

    let error = analyze(&source.replace(
        "background-border=primary background-border-width",
        "background-border=missing background-border-width",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(
        error
            .message
            .contains("toggler background border color `missing`")
    );

    let error = analyze(&source.replace("padding-ratio=0.125", "padding-ratio=0.6")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("toggler padding ratio"));

    let error = analyze(&source.replace(
            "      active unchecked background=background foreground=foreground",
            "      active checked background=background\n      active unchecked background=background foreground=foreground",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E075");
    assert!(error.message.contains("duplicate toggler active checked"));
}

#[test]
fn checks_complete_radio_api_and_generic_values() {
    let source = r#"app Choices
extern crate::backend
  Item(id:i64)
  radio-style dynamic_radio(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  choice = "list"
  items:[Item] = []
  highlight = false
on changed(next)
  choice = next
on float_changed(next)
on item_changed(next)
view
  col
    radio "List" value="list" selected=(choice == "list") style=dynamic_radio(highlight) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono -> changed _
      active selected background=linear(1.57, primary@0.0, background@1.0) dot=foreground border=primary border-width=2.0 text=foreground
      active unselected background=background dot=primary border=foreground text=foreground
      hovered selected background=primary dot=foreground border=foreground text=foreground
      hovered unselected background=foreground dot=background border=primary text=primary
    radio "Float" value=1.5 selected=false -> float_changed _
    for item in items
      radio "Item" value=item selected=false -> item_changed _
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[0].params[0].ty.display(), "str");
    assert_eq!(document.handlers[1].params[0].ty.display(), "f64");
    assert_eq!(document.handlers[2].params[0].ty.display(), "Item");

    let error =
        analyze(&source.replace("border=primary border-width", "border=missing border-width"))
            .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("radio border color `missing`"));

    let error = analyze(&source.replace("border-width=2.0", "border-width=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("radio border width"));

    let error = analyze(&source.replace("value=\"list\"", "value=[\"list\"]")).unwrap_err();
    assert_eq!(error.code, "E125");
    assert!(error.message.contains("radio values must be"));

    let error =
        analyze(&source.replace("style=dynamic_radio(highlight)", "style=default")).unwrap_err();
    assert_eq!(error.code, "E078");
    assert!(error.message.contains("radio style must be"));

    let error = analyze(&source.replace("dynamic_radio(highlight)", "missing_radio(highlight)"))
        .unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("radio style"));

    let error =
        analyze(&source.replace("dynamic_radio(highlight)", "dynamic_radio(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
            "      active unselected background=background",
            "      active selected background=background\n      active unselected background=background",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E078");
    assert!(error.message.contains("duplicate radio active selected"));
}

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
