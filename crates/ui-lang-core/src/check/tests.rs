use crate::{PaneConfiguration, Type, ViewNode, analyze};

#[test]
fn checks_native_timer_subscription() {
    let source = include_str!("../../../../examples/iced-app/src/ui/timer.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty, Type::Instant);
    assert_eq!(document.handlers[2].params[0].ty, Type::I64);
    assert_eq!(document.handlers[2].params[1].ty, Type::I64);
    assert_eq!(document.handlers[3].params[0].ty, Type::I64);
    assert_eq!(document.handlers[3].params[1].ty, Type::Str);
    assert_eq!(document.handlers[4].params[0].ty, Type::Bool);

    let error = analyze(&source.replace(
        "every 250ms when auto_refresh -> tick _",
        "every 250ms when auto_refresh -> tick",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E133");
    assert!(error.message.contains("expects 1 arguments, got 0"));

    let error = analyze(&source.replace("refresh_time() -> i64", "refresh_time(seed:i64) -> i64"))
        .unwrap_err();
    assert_eq!(error.code, "E142");

    for invalid in ["0ms", "1m", "1.5s"] {
        let error = analyze(&source.replace("250ms", invalid)).unwrap_err();
        assert_eq!(error.code, "E084");
    }

    let error = analyze(&source.replace("when auto_refresh", "when 1")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `bool`"));

    let error = analyze(&source.replace(
        "every 250ms when auto_refresh",
        "every 250ms status=captured when auto_refresh",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(
        error
            .message
            .contains("only available on non-frame runtime events")
    );

    let error = analyze(&source.replace(
        "sync even_refresh(value:i64) -> i64?",
        "sync even_refresh(value:i64, extra:i64) -> i64?",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E142");
    assert!(error.message.contains("expects 2 payloads, got 1"));

    let error = analyze(&source.replace(
        "sync even_refresh(value:i64) -> i64?",
        "sync even_refresh(value:i64) -> i64",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E142");
    assert!(error.message.contains("must return an optional value"));

    let error = analyze(&source.replace("with=generation", "with=1.5")).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("context must be hashable"));
}

#[test]
fn checks_generic_event_values_and_filters() {
    let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
  sync event_label(value:event) -> str?
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  last = "none"
  last_window:window-id? = none
on received(value)
  last = event_name(value)
on labeled(value)
  last = value
on identified(id, value)
  last_window = some(id)
  last = event_name(value)
subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event raw with-id status=captured -> identified _ _
view
  text last
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[0].params[0].ty, Type::Event);
    assert_eq!(document.handlers[1].params[0].ty, Type::Str);
    assert_eq!(document.handlers[2].params[0].ty, Type::WindowId);
    assert_eq!(document.handlers[2].params[1].ty, Type::Event);

    let error = analyze(&source.replace(
        "sync event_label(value:event) -> str?",
        "sync event_label(value:str) -> str?",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`, got `event`"));
}

#[test]
fn checks_all_native_input_method_subscription_payloads() {
    let source = include_str!("../../../../examples/iced-app/src/ui/input_method_events.ice");
    let document = analyze(source).unwrap();
    let preedit = document
        .handlers
        .iter()
        .find(|handler| handler.name == "preedit")
        .unwrap();
    assert_eq!(
        preedit
            .params
            .iter()
            .map(|param| param.ty.display())
            .collect::<Vec<_>>(),
        ["str", "i64?", "i64?"]
    );

    let error = analyze(&source.replace(
        "input-method preedit status=any -> preedit _ _ _",
        "input-method preedit status=any -> preedit _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 3 payloads"));

    let error = analyze(&source.replace(
        "input-method closed -> closed",
        "input-method disabled -> closed",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("input-method event must be"));
}

#[test]
fn checks_all_native_mouse_subscription_payloads() {
    let source = include_str!("../../../../examples/iced-app/src/ui/mouse_events.ice");
    let document = analyze(source).unwrap();
    let handlers = document
        .handlers
        .iter()
        .map(|handler| {
            (
                handler.name.as_str(),
                handler
                    .params
                    .iter()
                    .map(|param| param.ty.display())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(handlers["entered"], Vec::<String>::new());
    assert_eq!(handlers["left"], Vec::<String>::new());
    assert_eq!(handlers["moved"], ["f64", "f64"]);
    assert_eq!(handlers["pressed"], ["mouse-button"]);
    assert_eq!(handlers["released"], ["mouse-button"]);
    assert_eq!(handlers["wheel"], ["f64", "f64", "bool"]);

    let error = analyze(&source.replace(
        "mouse moved status=captured -> moved _ _",
        "mouse moved status=captured -> moved _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 2 payloads"));

    let error = analyze(&source.replace(
        "mouse wheel -> wheel _ _ _",
        "mouse wheel -> wheel 1.0 2.0 true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error =
        analyze(&source.replace("mouse left -> left", "mouse dragged -> left")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("mouse event must be"));

    let error = analyze(&source.replace("status=captured", "status=handled")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("status must be"));
}

#[test]
fn checks_all_native_touch_subscription_payloads() {
    let source = include_str!("../../../../examples/iced-app/src/ui/touch_events.ice");
    let document = analyze(source).unwrap();
    for handler in &document.handlers {
        assert_eq!(
            handler
                .params
                .iter()
                .map(|param| param.ty.display())
                .collect::<Vec<_>>(),
            ["touch-finger", "f64", "f64"]
        );
    }

    let error = analyze(&source.replace("touch moved -> moved _ _ _", "touch moved -> moved _ _"))
        .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 3 payloads"));

    let error = analyze(&source.replace("touch lost -> lost _ _ _", "touch ended -> lost _ _ _"))
        .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("touch event must be"));
}

#[test]
fn checks_typed_pointer_values() {
    let source = include_str!("../../../../examples/iced-app/src/ui/pointer_values.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty, Type::MouseButton);
    assert_eq!(document.handlers[2].params[0].ty, Type::TouchFinger);

    for (from, to, code, message) in [
        (
            "mouse.button(\"left\")",
            "mouse.button(\"side\")",
            "E152",
            "must be left, right, middle, back, or forward",
        ),
        (
            "mouse.other_button(9)",
            "mouse.other_button(65536)",
            "E152",
            "must be in 0..=65535",
        ),
        (
            "mouse.cursor(point(12.0, 24.0))",
            "mouse.cursor(true)",
            "E101",
            "expected `point`, got `bool`",
        ),
        (
            "mouse.button(\"left\"), none",
            "mouse.button(\"left\"), some(mouse.unavailable())",
            "E101",
            "expected `mouse-click?`, got `mouse-cursor?`",
        ),
        (
            "touch.finger(\"18446744073709551615\")",
            "touch.finger(\"18446744073709551616\")",
            "E152",
            "must contain a decimal u64",
        ),
        (
            "touch.finger(\"18446744073709551615\")",
            "touch.finger(\"+42\")",
            "E152",
            "must contain a decimal u64",
        ),
        (
            "cursor.kind",
            "cursor.missing",
            "E151",
            "has no field `missing`",
        ),
        (
            "cursor_levitating = mouse.cursor_is_levitating(mouse.cursor_levitate(cursor))",
            "cursor_levitating = click == click",
            "E153",
            "mouse-click values are opaque",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_native_transformations() {
    let source = include_str!("../../../../examples/iced-app/src/ui/transformation_values.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.states[0].ty, Type::Transformation);
    assert_eq!(document.states[6].ty, Type::Vector);
    assert_eq!(document.states[11].ty, Type::Size);

    for (from, to, code, message) in [
        (
            "transform.identity()",
            "transform.identity(1)",
            "E152",
            "expects 0 argument(s)",
        ),
        (
            "transform.orthographic(640, 480)",
            "transform.orthographic(640.0, 480)",
            "E152",
            "expects two integer literals",
        ),
        (
            "transform.orthographic(640, 480)",
            "transform.orthographic(4294967296, 480)",
            "E152",
            "dimensions must be in 0..=4294967295",
        ),
        (
            "transform.translate(10.0, 20.0)",
            "transform.translate(true, 20.0)",
            "E101",
            "expected `f64`, got `bool`",
        ),
        (
            "transform.scale(2.0))",
            "point(2.0, 2.0))",
            "E101",
            "expected `transformation`, got `point`",
        ),
        (
            "transform.point(point(1.0, 2.0), combined)",
            "transform.point(combined, point(1.0, 2.0))",
            "E101",
            "expected `point`, got `transformation`",
        ),
        (
            "translation = combined.translation",
            "translation = combined.missing",
            "E151",
            "has no field `missing`",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_native_geometry_values() {
    let source = include_str!("../../../../examples/iced-app/src/ui/geometry_values.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.functions[0].output, Type::RectangleU32);
    assert_eq!(document.functions[1].params[1].1, Type::PointU32);
    assert_eq!(
        document.functions[1].params[5].1,
        Type::Option(Box::new(Type::RectangleU32))
    );

    for (from, to, code, message) in [
        (
            "point.origin()",
            "point.origin(1)",
            "E152",
            "expects 0 argument(s)",
        ),
        (
            "size.from_u32(640, 480)",
            "size.from_u32(point_distance, 480)",
            "E152",
            "expects two integer literals",
        ),
        (
            "size.from_u32(640, 480)",
            "size.from_u32(4294967296, 480)",
            "E152",
            "dimensions must be in 0..=4294967295",
        ),
        (
            "\"right\", \"bottom\"",
            "\"around\", \"bottom\"",
            "E152",
            "horizontal alignment must be left, center, or right",
        ),
        (
            "point_value = (point.origin() + vector(3.25, 4.75)) - vector.zero()",
            "point_value = point.origin() + point.origin()",
            "E153",
            "does not accept `point` and `point`",
        ),
        (
            "vector_value = ((-vector(1.0, 2.0)",
            "vector_value = ((-point(1.0, 2.0)",
            "E153",
            "negation expects i64, f64, or vector",
        ),
        (
            "snapped_x = snapped_point.x",
            "snapped_x = snapped_point.missing",
            "E151",
            "has no field `missing`",
        ),
        (
            "scaled_bounds = bounds * 2.0",
            "scaled_bounds = bounds / 2.0",
            "E153",
            "does not accept `rectangle` and `f64`",
        ),
        (
            "contains_point = rectangle.contains(bounds, point(20.0, 30.0))",
            "contains_point = point_value < point.origin()",
            "E153",
            "does not accept `point` and `point`",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_native_padding_and_angles() {
    let source = include_str!("../../../../examples/iced-app/src/ui/padding_angles.ice");
    let document = analyze(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Pixels);
    assert_eq!(document.functions[0].params[1].1, Type::Padding);
    assert_eq!(document.functions[0].params[2].1, Type::Degrees);
    assert_eq!(document.functions[0].params[3].1, Type::Radians);

    for (from, to, code, message) in [
        (
            "pixels.from_u32(4294967295)",
            "pixels.from_u32(4294967296)",
            "E152",
            "value must be in 0..=4294967295",
        ),
        (
            "pixels.from_u32(4294967295)",
            "pixels.from_u32(pixel_value.value)",
            "E152",
            "expects one integer literal",
        ),
        (
            "all_padding = padding.all(5.0)",
            "all_padding = padding.all(true)",
            "E101",
            "expected `f64` or `pixels`, got `bool`",
        ),
        (
            "all_padding = padding.all(5.0)",
            "all_padding = padding.all(5.0, 6.0)",
            "E152",
            "expects one argument",
        ),
        (
            "padding_equal = direct_padding == padding(1.0, 2.0, 3.0, 4.0)",
            "padding_equal = direct_padding < padding(1.0, 2.0, 3.0, 4.0)",
            "E153",
            "does not accept `padding` and `padding`",
        ),
        (
            "pixel_value = ((((pixels(4.0) + pixels(2.0))",
            "pixel_value = ((((pixels(4.0) - pixels(2.0))",
            "E153",
            "does not accept `pixels` and `pixels`",
        ),
        (
            "degree_value = degrees(45.0) * 2.0",
            "degree_value = degrees(45.0) + degrees(2.0)",
            "E153",
            "does not accept `degrees` and `degrees`",
        ),
        (
            "radians_reverse = 2.0 * radians(1.5)",
            "radians_reverse = 2.0 + radians(1.5)",
            "E153",
            "does not accept `f64` and `radians`",
        ),
        (
            "radians(5.0) % radians(2.0)",
            "radians(5.0) % degrees(2.0)",
            "E153",
            "does not accept `radians` and `degrees`",
        ),
        (
            "rotated_size = size.rotate(size(10.0, 20.0), radians_value)",
            "rotated_size = size.rotate(size(10.0, 20.0), degree_value)",
            "E101",
            "expected `f64` or `radians`, got `degrees`",
        ),
        (
            "radians_equal_scalar = radians_value == 1.0",
            "radians_equal_scalar = 1.0 == radians_value",
            "E101",
            "expected `radians`, got `f64`",
        ),
        (
            "radians_display = radians_value.display",
            "radians_display = radians_value.missing",
            "E151",
            "has no field `missing`",
        ),
    ] {
        let error = analyze(&source.replacen(from, to, 1)).unwrap_err();
        assert_eq!(error.code, code, "{from} -> {to}: {error:?}");
        assert!(error.message.contains(message), "{from} -> {to}: {error:?}");
    }
}

#[test]
fn checks_all_native_window_subscription_payloads() {
    let source = include_str!("../../../../examples/iced-app/src/ui/window_events.ice");
    let document = analyze(source).unwrap();
    let opened = document
        .handlers
        .iter()
        .find(|handler| handler.name == "opened")
        .unwrap();
    assert_eq!(
        opened
            .params
            .iter()
            .map(|param| param.ty.display())
            .collect::<Vec<_>>(),
        ["window-id", "f64?", "f64?", "f64", "f64"]
    );

    let error = analyze(&source.replace(
        "window moved with-id status=captured -> moved _ _ _",
        "window moved with-id status=captured -> moved _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("expects 3 payloads"));

    let error = analyze(&source.replace(
        "window resized with-id -> resized _ _ _",
        "window resized with-id -> resized 1.0 2.0 3.0",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error = analyze(&source.replace(
        "window frame when listen_frames -> frame",
        "window frame with-id when listen_frames -> frame _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("does not expose a window ID"));
}

#[test]
fn infers_action_result_handler() {
    let source = r#"app Demo
extern crate::backend
  Item(id:i64)
  load() -> [Item] ! Item
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
on mount
  run load() -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items) @text-sm
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty.display(), "[Item]");
}

#[test]
fn checks_structured_task_groups() {
    let source = r#"app Grouped
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  mode = ""
on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _
on theme_read(next)
  mode = next
on clipboard_read(next)
on info_read(info)
view
  text mode
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty.display(), "str");
    assert_eq!(document.handlers[2].params[0].ty.display(), "str?");
    assert_eq!(document.handlers[3].params[0].ty.display(), "system-info");

    let error = analyze(&source.replace(
        "      task clipboard read -> clipboard_read _",
        "      mode = \"invalid\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E143");
    assert!(error.message.contains("task-producing"));

    let error = analyze(&source.replace(
        "on theme_read(next)",
        "  mode = \"too late\"\non theme_read(next)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E141");
    assert!(error.message.contains("final statement"));
}

#[test]
fn checks_native_task_cancellation() {
    let source = r#"app Cancel
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  request:task-handle? = none
  canceled = false
on start
  abortable request abort-on-drop
    task system theme -> loaded _
on loaded(next)
on cancel
  abort request
  canceled = aborted(request)
view
  col
    if aborted(request)
      text "Canceled"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[0].ty.display(), "task-handle?");

    let error = analyze(&source.replace("request:task-handle?", "request:str?")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("task-handle?"));

    let error = analyze(&source.replace("abort request", "abort missing")).unwrap_err();
    assert_eq!(error.code, "E143");
    assert!(error.message.contains("unknown task handle"));

    let error =
        analyze(&source.replace("    task system theme -> loaded _", "    canceled = false"))
            .unwrap_err();
    assert_eq!(error.code, "E143");
    assert!(error.message.contains("task-producing"));

    let error = analyze(&source.replace(
            "  abortable request abort-on-drop\n    task system theme -> loaded _",
            "  parallel\n    abortable request\n      canceled = false\n    task system theme -> loaded _",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E143");
    assert_eq!(error.line, 13);

    let error = analyze(&source.replace("on loaded(next)", "  canceled = false\non loaded(next)"))
        .unwrap_err();
    assert_eq!(error.code, "E141");
    assert!(error.message.contains("final statement"));

    let error = analyze(&source.replace("aborted(request)", "aborted(true)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("aborted(request)", "request == none")).unwrap_err();
    assert_eq!(error.code, "E153");
    assert!(error.message.contains("opaque"));
}

#[test]
fn checks_typed_task_streams() {
    let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream coordinates(value:f64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(value:i64) -> str
  event-filter raw_event() -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  count = 0
on start
  parallel
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
on number(value)
  count = value
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(count) -> number _
  recipe snapshot(count) -> text _
  events count using=raw_event -> text _
view
  text count
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty.display(), "i64");
    assert_eq!(document.handlers[2].params[0].ty.display(), "str");
    assert_eq!(document.handlers[3].params[0].ty.display(), "AppError");
    assert_eq!(
        document.handlers[4].params[0].ty.display(),
        "result[str,AppError]"
    );

    let error = analyze(&source.replace("numbers(3)", "numbers(true)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
        "stream fallible() -> text _ | failed _",
        "stream fallible() -> text _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E131");

    let error = analyze(&source.replace(
        "stream numbers(3) -> number _",
        "stream numbers(3) -> number count",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");
    assert!(error.message.contains("at most one `_`"));

    let error = analyze(&source.replace(
        "stream numbers(3) -> number _",
        "stream numbers(3) -> number _ _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error = analyze(&source.replace("stream numbers(3)", "stream missing(3)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("extern stream"));

    let error = analyze(&source.replace(
        "run numbers(count) -> number _",
        "run coordinates(1.5) -> number _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("run data must be hashable"));

    let error =
        analyze(&source.replace("recipe snapshot(count)", "recipe missing(count)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("extern recipe"));

    let error =
        analyze(&source.replace("events count using=raw_event", "events 1.5 using=raw_event"))
            .unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("event identity must be hashable"));

    let error = analyze(&source.replace("using=raw_event", "using=missing")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("event filter"));
}

#[test]
fn checks_typed_task_sips() {
    let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip transfer(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    sip transfer(3)
      progress -> advanced _
      done -> downloaded _
    sip fallible()
      progress -> counted _
      done -> finished _
      error -> failed _
on advanced(value)
on downloaded(value)
on counted(value)
on finished(value)
on failed(error)
view
  text "Sips"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[1].params[0].ty, Type::F64);
    assert_eq!(document.handlers[2].params[0].ty, Type::Bytes);
    assert_eq!(document.handlers[3].params[0].ty, Type::I64);
    assert_eq!(document.handlers[4].params[0].ty, Type::Str);
    assert_eq!(
        document.handlers[5].params[0].ty,
        Type::Named("AppError".into())
    );

    let error = analyze(&source.replace("transfer(3)", "transfer(true)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("      error -> failed _\n", "")).unwrap_err();
    assert_eq!(error.code, "E131");

    let error = analyze(&source.replace(
        "      progress -> advanced _",
        "      progress -> advanced 1.0",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E127");

    let error = analyze(&source.replace("sip transfer(3)", "sip missing(3)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("extern sip"));
}

#[test]
fn checks_structured_task_flows() {
    let source = r#"app Flows
extern crate::backend
  AppError(message:str)
  OtherError(message:str)
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
  task optional(value:i64) -> i64?
  task fallible(value:i64) -> i64 ! AppError
  task fallible_double(value:i64) -> i64 ! AppError
  task wrong_error(value:i64) -> i64 ! OtherError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  limit = 3
on start
  parallel
    flow
      from stream numbers(limit)
      then value -> task double(value)
      collect
      done -> collected _
      units -> planned _
    flow
      from task optional(2)
      and-then value -> task double(value)
      done -> finished _
    flow
      from task fallible(2)
      and-then value -> task fallible_double(value)
      done -> finished _
      error -> failed _
on collected(values)
on planned(units)
on finished(value)
on failed(error)
view
  text "Flows"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.handlers[1].params[0].ty,
        Type::List(Box::new(Type::I64))
    );
    assert_eq!(document.handlers[2].params[0].ty, Type::I64);
    assert_eq!(document.handlers[3].params[0].ty, Type::I64);
    assert_eq!(
        document.handlers[4].params[0].ty,
        Type::Named("AppError".into())
    );

    let error = analyze(&source.replace(
        "and-then value -> task fallible_double(value)",
        "then value -> task fallible_double(value)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E144");
    assert!(error.message.contains("use and-then"));

    let error = analyze(&source.replace(
        "and-then value -> task fallible_double(value)",
        "and-then value -> task wrong_error(value)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
        "then value -> task double(value)",
        "then value -> task double(limit)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.hint.unwrap().contains("only read its `value`"));
}

#[test]
fn checks_task_error_mapping_and_native_sources() {
    let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  results:[result[i64,AppError]] = []
on start
  parallel
    flow
      from task request()
      map-error reason -> normalize(reason)
      collect
      done -> collected _
    flow
      from done 1
      then value -> done value + 1
      done -> finished _
    flow
      from none i64
      done -> finished _
on collected(values)
  results = values
on finished(value)
view
  text len(results)
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.handlers[1].params[0].ty,
        Type::List(Box::new(Type::Result(
            Box::new(Type::I64),
            Box::new(Type::Named("AppError".into()))
        )))
    );
    assert_eq!(document.handlers[2].params[0].ty, Type::I64);

    let error = analyze(&source.replace(
        "map-error reason -> normalize(reason)",
        "map-error reason -> normalize(1)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace(
        "from task request()\n      map-error reason -> normalize(reason)",
        "from done 1\n      map-error reason -> normalize(reason)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E144");
    assert!(error.message.contains("fallible"));

    let error = analyze(&source.replace("from none i64", "from none Missing")).unwrap_err();
    assert_eq!(error.code, "E103");
}

#[test]
fn checks_optional_selection_values() {
    let source = r#"app Demo
extern crate::backend
  pick-list-style dynamic_pick(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on opened
view
  pick choices selected placeholder="Choose" line-height=1.2 shaping=advanced font=ui open=opened style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
    active text=foreground placeholder=danger handle=primary background=background border=foreground border-width=1.0 radius=4.0
    hovered text=foreground
    opened text=foreground
    opened-hovered text=foreground
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground shadow=danger shadow-y=2.0
    handle dynamic
      closed code="⌄" font=ui size=12.0 line-height=1.0 shaping=basic
      open code="⌃" font=ui size=12.0 line-height=1.0 shaping=advanced
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[1].ty.display(), "[str]");
    assert_eq!(document.states[2].ty.display(), "str?");
    assert_eq!(document.handlers[0].params[0].ty.display(), "str");

    let error = analyze(&source.replace("size=12.0", "size=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("icon size"));

    let error = analyze(&source.replace("dynamic_pick(busy)", "missing(busy)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("pick-list style"));

    let error = analyze(&source.replace("dynamic_menu(busy)", "missing(busy)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("menu style"));

    let error = analyze(&source.replace("dynamic_pick(busy)", "dynamic_pick(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("style=dynamic_pick(busy)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E087");
    assert!(error.message.contains("declared style call"));
}

#[test]
fn rejects_a_non_optional_pick_selection() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  choices = ["List", "Board"]
  selected = "List"
on selected(next)
  selected = next
view
  pick choices selected -> selected _
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("optional"));
}

#[test]
fn checks_qr_declarations_and_references() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
qr code "hello" version=micro(0)
view
  qr code
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E136");
    assert!(error.message.contains("micro(1..4)"));

    let source = source.replace(
        "qr code \"hello\" version=micro(0)",
        "qr saved \"hello\" version=micro(4)",
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E136");
    assert!(error.message.contains("unknown qr data `code`"));
}

#[test]
fn rejects_unknown_nested_theme_colors() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  theme dark text=missing
    text "Hello"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E137");
    assert!(error.message.contains("missing"));

    let source = source.replace(
        "theme dark text=missing",
        "theme dark background=linear(1.57, background@0.0, missing@1.0)",
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E137");
    assert!(error.message.contains("missing"));
}

#[test]
fn checks_component_slot_contracts() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
component Card(title:str, padded:bool)
  col
    text title
    slot
view
  Card padded=true title="Editor"
    input "Name" <-> draft
"#;
    analyze(source).unwrap();
    analyze(&source.replace(
        "Card padded=true title=\"Editor\"",
        "Card(\"Editor\", true)",
    ))
    .unwrap();

    let error = analyze(&source.replace(
        "  Card padded=true title=\"Editor\"\n    input \"Name\" <-> draft",
        "  Card padded=true title=\"Editor\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("requires slot `children`"));

    let error = analyze(&source.replace("    text title\n    slot", "    text title")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `children`"));

    let error = analyze(&source.replace("padded=true ", "")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("missing prop `padded`"));

    let error = analyze(&source.replace("padded=true", "raised=true")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("no prop `raised`"));

    let error = analyze(&source.replace("padded=true", "title=\"Again\"")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("prop `title` more than once"));

    let error = analyze(&source.replace("title=\"Editor\"", "title=true")).unwrap_err();
    assert!(error.message.contains("expected `str`, got `bool`"));

    let error = analyze(&source.replace("padded:bool", "title:bool")).unwrap_err();
    assert_eq!(error.code, "E100");
    assert!(error.message.contains("duplicate component prop `title`"));
}

#[test]
fn checks_named_component_slots() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog(title:str)
  col
    slot header
    text title
    slot body
    slot actions
on cancel
on delete
view
  Dialog title="Delete task?"
    header:
      text "Danger zone"
    body:
      col
        text "This cannot be undone."
    actions:
      row
        button "Cancel" -> cancel
        button "Delete" -> delete
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace(
            "    actions:\n      row\n        button \"Cancel\" -> cancel\n        button \"Delete\" -> delete\n",
            "",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("requires slot `actions`"));

    let error = analyze(&source.replace("    actions:", "    footer:")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `footer`"));

    let error = analyze(&source.replace(
        "    body:\n      col\n        text \"This cannot be undone.\"",
        "    body:\n      text \"First\"\n      text \"Second\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("slot `body` needs exactly one root"));

    let error = analyze(&source.replace("    slot actions", "    slot body")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(
        error
            .message
            .contains("declares slot `body` more than once")
    );
}

#[test]
fn checks_compound_component_slots() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog()
  col
    slot Header
    slot Body
    slot Actions
component Dialog.Header(title:str)
  col
    text title
    slot
component Dialog.Body()
  container
    slot
component Dialog.Actions()
  row
    slot
on close
view
  Dialog
    Dialog.Header title="About"
      text "Compound title"
    Dialog.Body
      text "Structured body"
    Dialog.Actions
      button "Close" -> close
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("    slot Actions\n", "")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `Actions`"));

    let error = analyze(&source.replace(
        "    Dialog.Actions\n      button \"Close\" -> close",
        "    text \"not compound\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("cannot mix compound components"));

    let error = analyze(&source.replace("Dialog.Header", "Dialog..Header")).unwrap_err();
    assert_eq!(error.code, "E072");
    assert!(error.message.contains("invalid component name"));
}

#[test]
fn checks_keyed_columns_and_copyable_keys() {
    let source = r#"app Demo
extern crate::backend
  Item(id:i64, name:str)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
view
  keyed item in items by=item.id width=fill height=shrink spacing=8.0 padding=4.0 max-width=640.0 align=center
    text item.name
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("by=item.id", "by=item.name")).unwrap_err();
    assert_eq!(error.code, "E138");
    assert!(error.message.contains("bool, i64, or f64"));

    let error = analyze(&source.replace("spacing=8.0", "spacing=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));
}

#[test]
fn checks_lazy_static_boundaries() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  title = "Hello"
  other = "Outside"
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("text len(cached)", "text other")).unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.message.contains("unknown value `other`"));

    let error = analyze(&source.replace("title = \"Hello\"", "title = 1.0")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("stable hashing"));

    let error =
        analyze(&source.replace("text len(cached)", "input \"Edit\" <-> cached")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("borrows app state"));

    let component_source = source.replace(
            "view\n  lazy title as cached\n    col\n      text cached\n      text len(cached)",
            "component Editor(value:str)\n  input \"Edit\" <-> value\nview\n  lazy title as cached\n    Editor(cached)",
        );
    let error = analyze(&component_source).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("borrows app state"));
}

#[test]
fn checks_markdown_content_settings_and_links() {
    let source = r##"app Docs
font ui family=sans
extern crate::backend
  markdown-viewer docs_viewer(prefix:str) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  docs:markdown = "# Hello [world](https://example.com)"
  images:[str] = []
on open(url)
on reset
  docs = markdown("# Reset")
on extend
  markdown docs append "\n![Ice](asset://ice)"
  images = markdown_images(docs)
view
  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 spacing=12.0 viewer=docs_viewer("docs") -> open _
    style font=ui inline-code-background=linear(1.57, background@0.0, primary@1.0) inline-code-color=foreground inline-code-font=mono code-block-font=mono link=primary inline-code-padding=2.0 inline-code-padding-x=3.0 inline-code-padding-y=4.0 inline-code-padding-top=5.0 inline-code-padding-right=6.0 inline-code-padding-bottom=7.0 inline-code-padding-left=8.0 inline-code-border=primary inline-code-border-width=1.0 inline-code-radius=4.0 inline-code-radius-tl=1.0 inline-code-radius-tr=2.0 inline-code-radius-br=3.0 inline-code-radius-bl=4.0
"##;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[0].ty.display(), "markdown");
    assert_eq!(document.handlers[0].params[0].ty.display(), "str");

    let error = analyze(&source.replace("spacing=12.0", "spacing=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));

    let error = analyze(&source.replace("markdown docs", "markdown missing")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("unknown markdown state"));

    let error =
        analyze(&source.replace("markdown docs append", "markdown missing append")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("unknown markdown state"));

    let error = analyze(&source.replace(
        "markdown docs append \"\\n![Ice](asset://ice)\"",
        "markdown docs append true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("viewer=docs_viewer", "viewer=missing")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("markdown viewer"));

    let error = analyze(&source.replace("link=primary", "link=missing")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("markdown link"));

    let error =
        analyze(&source.replace("markdown_images(docs)", "markdown_images(true)")).unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn checks_structured_tables_and_metrics() {
    let source = r#"app Rows
extern crate::backend
  Item(name:str, done:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  rows:[Item] = []
view
  table row in rows width=fill padding=4.0 padding-x=8.0 padding-y=6.0 separator=1.0 separator-x=2.0 separator-y=3.0
    column width=fill(2) align-x=left align-y=center
      header
        text "Name"
      cell
        text row.name
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("padding=4.0", "padding=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));

    let error = analyze(&source.replace("table row in rows", "table row in true")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("list of rows"));
}

#[test]
fn checks_bound_text_editors_and_highlighting() {
    let source = r#"app Notes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = "fn main() {}"
  locked = false
view
  editor #body <-> body placeholder="Write" width=640.0 height=fill min-height=80.0 max-height=240.0 size=14.0 line-height=1.3 padding=8.0 wrapping=word-or-glyph font=mono highlight="rs" highlight-theme=solarized-dark disabled=locked
    active background=background border=foreground border-width=1.0 radius=4.0 placeholder=danger value=foreground selection=primary
    hovered background=background border=primary placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[0].ty.display(), "editor");

    let error = analyze(&source.replace("min-height=80.0", "min-height=300.0")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("cannot exceed"));

    let error = analyze(&source.replace("placeholder=danger", "icon=danger")).unwrap_err();
    assert_eq!(error.code, "E099");
    assert!(error.message.contains("unknown editor style property"));
}

#[test]
fn checks_component_controlled_state_origins() {
    let source = r#"app Notes
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-style editor_surface(readonly:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = ""
  title = "Notes"
  locked = false
  language = "rs"
component EditorPanel(content:editor, heading:str, readonly:bool, syntax:str)
  col
    input "Title" <-> heading
    editor <-> content highlighter=editor_highlight(syntax) key-binding=editor_keys(readonly) style=editor_surface(readonly) -> command _
on command(value)
view
  EditorPanel(body, title, locked, language)
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[0].params[0].ty.display(), "EditorCommand");

    let error = analyze(&source.replace(
        "EditorPanel(body, title, locked, language)",
        "EditorPanel(editor(\"scratch\"), title, locked, language)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(
        error
            .message
            .contains("editor binding must resolve to an app state")
    );

    let error = analyze(&source.replace("editor_keys(readonly)", "missing(readonly)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor binding"));

    let error =
        analyze(&source.replace("editor_highlight(syntax)", "missing(syntax)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor highlighter"));

    let error =
        analyze(&source.replace("editor_surface(readonly)", "missing(readonly)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor style"));
}

#[test]
fn rejects_slots_outside_components_and_duplicate_slots() {
    let outside = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  slot
"#;
    let error = analyze(outside).unwrap_err();
    assert_eq!(error.code, "E124");
    assert_eq!(error.line, 8);

    let duplicate = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Card()
  col
    slot
    slot
view
  text "Hello"
"#;
    let error = analyze(duplicate).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(
        error
            .message
            .contains("declares slot `children` more than once")
    );
}

#[test]
fn checks_combo_search_state_and_routes() {
    let source = r#"app Demo
extern crate::backend
  input-style dynamic_input(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
  query = ""
on selected(next)
  selected = some(next)
on searched(next)
  query = next
on hovered(next)
on opened
on closed
on add
  combo modes push "Timeline"
view
  combo modes selected "Search modes" line-height=1.2 shaping=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground shadow=danger shadow-y=2.0
    icon code="⌕" font=ui size=12.0 spacing=6.0 side=right
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[1].ty.display(), "combo[str]");
    assert_eq!(document.handlers[0].params[0].ty.display(), "str");
    assert_eq!(document.handlers[1].params[0].ty.display(), "str");
    assert_eq!(document.handlers[2].params[0].ty.display(), "str");

    let error = analyze(&source.replace("spacing=6.0", "spacing=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("icon spacing"));

    let error = analyze(&source.replace("combo modes push", "combo missing push")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("unknown combo state"));

    let error = analyze(&source.replace("combo modes push", "combo selected push")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("not combo state"));

    let error = analyze(&source.replace("push \"Timeline\"", "push 1")).unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn replaces_combo_search_options_with_a_typed_list() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
on reset
  modes = ["Timeline"]
on selected(next)
  selected = some(next)
view
  combo modes selected "Search modes" -> selected _
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("[\"Timeline\"]", "[1]")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `[str]`, got `[i64]`"));
}

#[test]
fn checks_structural_widget_routes_and_ranges() {
    let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  sensor_key = 0
  width = 0.0
  height = 0.0
on shown(w, h)
  width = w
  height = h
on resized(w, h)
  width = w
  height = h
on hidden
view
  col
    float scale=1.1 x=(viewport_x + viewport_width - original_x - original_width) y=(viewport_y + viewport_height - original_y - original_height) shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 radius=8.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
      text "Floating"
    pin width=fill height=80.0 x=12.0 y=8.0
      text "Pinned"
    sensor show=shown resize=resized hide=hidden key=sensor_key anticipate=32.0 delay=10
      text "Observed"
    responsive at=600.0 width=fill height=40.0
      text "Narrow"
      text "Wide"
    responsive size=(available_width, available_height) width=fill height=fill
      col
        if available_width < available_height
          text "Portrait"
        if available_width >= available_height
          text "Landscape"
    stack width=fill(2) height=120.0 clip=true under=1
      text "Base"
      text "Overlay"
    rule horizontal thickness=2.0 style=weak fill=percent(75.0) color=primary/50 radius=4.0 radius-tl=2.0 snap=false
    space width=fill(2) height=shrink
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[0].params[0].ty.display(), "f64");
    assert_eq!(document.handlers[0].params[1].ty.display(), "f64");
    assert_eq!(document.handlers[1].params[0].ty.display(), "f64");

    let bad_float_translation = source.replace(
        "x=(viewport_x + viewport_width - original_x - original_width)",
        "x=true",
    );
    let error = analyze(&bad_float_translation).unwrap_err();
    assert!(error.message.contains("expected `f64`, got `bool`"));

    let bad_float_blur = source.replace("shadow-blur=4.0", "shadow-blur=-1.0");
    let error = analyze(&bad_float_blur).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("float style metric"));

    let bad_float_color = source.replace("shadow=black/50", "shadow=missing");
    let error = analyze(&bad_float_color).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("unknown float shadow color"));

    let bad_stack = source.replace("height=120.0 clip=true", "height=-1.0 clip=true");
    let error = analyze(&bad_stack).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("stack size"));

    let bad_under = source.replace("under=1", "under=70000");
    let error = analyze(&bad_under).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("stack under"));

    let duplicate_size_name = source.replace(
        "size=(available_width, available_height)",
        "size=(available_width, available_width)",
    );
    let error = analyze(&duplicate_size_name).unwrap_err();
    assert_eq!(error.code, "E092");
    assert!(error.message.contains("different names"));

    let conflicting_responsive = source.replace(
        "responsive size=(available_width, available_height)",
        "responsive at=600.0 size=(available_width, available_height)",
    );
    let error = analyze(&conflicting_responsive).unwrap_err();
    assert_eq!(error.code, "E092");
    assert!(error.message.contains("either `at=` or `size=`"));
}

#[path = "tests/platform.rs"]
mod platform;
#[path = "tests/widgets.rs"]
mod widgets;
