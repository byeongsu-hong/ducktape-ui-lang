use super::*;

const SOURCE: &str = r#"app Demo

extern crate::backend
  Item(id:i64, name:str)
  load() -> [Item] ! Item

theme
  background #000000

qr docs "https://example.com/ice docs" correction=high version=normal(4)

state
  items:[Item] = []
  query = ""

on mount
  run load() -> loaded _ | failed _

on loaded(next)
  items = next

on failed(error)
  query = error.name

view
  input "Query" #query <-> query @w-full
"#;

#[test]
fn parses_compact_app() {
    let document = parse(SOURCE).unwrap();
    assert_eq!(document.app, "Demo");
    assert_eq!(document.structs.len(), 1);
    assert_eq!(document.handlers.len(), 3);
    assert_eq!(document.qr_codes.len(), 1);
    assert_eq!(
        document.qr_codes[0].data,
        QrPayload::Text("https://example.com/ice docs".into())
    );
}

#[test]
fn parses_all_native_time_operations() {
    let source = include_str!("../../../../examples/iced-app/src/ui/timer.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.states[1].ty, Type::Option(Box::new(Type::Instant)));
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Run { function, .. } if function == "__ice_time_now"
    ));
    assert!(matches!(
        document.subscriptions[0].source,
        SubscriptionSource::Every { milliseconds: 250 }
    ));
    assert!(matches!(
        &document.subscriptions[1].source,
        SubscriptionSource::Repeat {
            function,
            milliseconds: 1000
        } if function == "refresh_time"
    ));
    assert_eq!(
        document.subscriptions[1].filter.as_deref(),
        Some("even_refresh")
    );
    assert!(matches!(
        document.subscriptions[1].context,
        Some(Expr::Path(ref path)) if path == &["generation"]
    ));
    assert_eq!(
        document.subscriptions[2].filter.as_deref(),
        Some("visible_pointer")
    );
    assert!(document.subscriptions[3].context.is_none());

    let error =
        parse(&source.replace("refresh_time() every", "refresh_time(1) every")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("cannot take arguments"));
}

#[test]
fn parses_structured_task_groups() {
    let source = SOURCE.replace(
            "  run load() -> loaded _ | failed _",
            "  parallel\n    run load() -> loaded _ | failed _\n    sequential\n      task clipboard read -> clipboard_read _\n      task system theme -> theme_read _",
        );
    let document = parse(&source).unwrap();
    let Statement::TaskGroup {
        kind, statements, ..
    } = &document.handlers[0].statements[0]
    else {
        panic!("expected task group");
    };
    assert_eq!(*kind, TaskGroupKind::Parallel);
    assert_eq!(statements.len(), 2);
    assert!(matches!(
        &statements[1],
        Statement::TaskGroup {
            kind: TaskGroupKind::Sequential,
            statements,
            ..
        } if statements.len() == 2
    ));

    let error =
        parse(&SOURCE.replace("  run load() -> loaded _ | failed _", "  parallel")).unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("at least one"));
}

#[test]
fn parses_abortable_tasks_and_handles() {
    let source = SOURCE
        .replace(
            "  query = \"\"",
            "  query = \"\"\n  request:task-handle? = none",
        )
        .replace(
            "  run load() -> loaded _ | failed _",
            "  abortable request abort-on-drop\n    run load() -> loaded _ | failed _",
        );
    let document = parse(&source).unwrap();
    assert_eq!(
        document.states[2].ty,
        Type::Option(Box::new(Type::TaskHandle))
    );
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Abortable {
            handle,
            abort_on_drop: true,
            task,
            ..
        } if handle == "request" && matches!(task.as_ref(), Statement::Run { .. })
    ));

    let error = parse(&SOURCE.replace(
        "  run load() -> loaded _ | failed _",
        "  abortable request later\n    run load() -> loaded _ | failed _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("abort-on-drop"));

    let error = parse(&SOURCE.replace(
            "  run load() -> loaded _ | failed _",
            "  abortable request\n    run load() -> loaded _ | failed _\n    run load() -> loaded _ | failed _",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("exactly one"));
}

#[test]
fn parses_dynamic_widget_operation_ids() {
    let source = r#"app Operations
theme
  background #000000
state
  selected = 1
  value = ""
on focus
  task widget focus #outer(selected)/inner/field
view
  input "Value" #field(selected) <-> value
"#;
    let document = parse(source).unwrap();
    let Statement::WidgetOperation {
        operation: WidgetOperation::Focus { target },
        ..
    } = &document.handlers[0].statements[0]
    else {
        panic!("expected dynamic focus operation");
    };
    assert_eq!(target.segments.len(), 3);
    let id = &target.segments[0];
    assert_eq!(id.name, "outer");
    assert!(matches!(
        id.key.as_ref(),
        Some(Expr::Path(path)) if path == &["selected"]
    ));
    assert_eq!(target.segments[1].name, "inner");
    assert!(target.segments[1].key.is_none());
    assert_eq!(target.segments[2].name, "field");
    assert!(target.segments[2].key.is_none());

    let error = parse(&source.replace(
        "focus #outer(selected)/inner/field",
        "focus outer(selected)/inner/field",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E052");
    assert!(error.message.contains("#id(key)"));
}

#[test]
fn parses_widget_selectors() {
    let source = include_str!("../../../../examples/iced-app/src/ui/widget_selectors.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Selector);
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::WidgetOperation {
            operation: WidgetOperation::Find {
                selector: WidgetSelector::Id(_),
                all: false,
            },
            ..
        }
    ));
    assert!(matches!(
        &document.handlers[4].statements[0],
        Statement::WidgetOperation {
            operation: WidgetOperation::Find {
                selector: WidgetSelector::Text(_),
                all: true,
            },
            ..
        }
    ));
    assert!(matches!(
        &document.handlers[5].statements[0],
        Statement::WidgetOperation {
            operation: WidgetOperation::Find {
                selector: WidgetSelector::Extern { function, args },
                all: true,
            },
            ..
        } if function == "by_kind" && args.len() == 1
    ));

    let error = parse(&source.replace("point 12.0 24.0", "point 12.0")).unwrap_err();
    assert_eq!(error.code, "E052");
    assert!(error.message.contains("requires x and y"));
}

#[test]
fn parses_typed_keyboard_values() {
    let source = include_str!("../../../../examples/iced-app/src/ui/keyboard_values.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.states[0].ty, Type::Key);
    assert_eq!(document.states[1].ty, Type::PhysicalKey);
    assert_eq!(
        document.states[3].ty,
        Type::Option(Box::new(Type::PhysicalKey))
    );
    assert_eq!(document.states[4].ty, Type::KeyLocation);
    assert_eq!(document.states[5].ty, Type::KeyModifiers);
    assert!(matches!(
        &document.states[0].initial,
        Expr::Call { name, args } if name == "key.unidentified" && args.is_empty()
    ));
    assert!(matches!(
        &document.handlers[0].statements[4],
        Statement::Assign {
            value: Expr::Call { name, args },
            ..
        } if name == "key.latin" && args.len() == 2
    ));
}

#[test]
fn parses_typed_pointer_values() {
    let source = include_str!("../../../../examples/iced-app/src/ui/pointer_values.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.states[0].ty, Type::Point);
    assert_eq!(document.states[1].ty, Type::Rectangle);
    assert_eq!(document.states[2].ty, Type::MouseButton);
    assert_eq!(document.states[5].ty, Type::MouseCursor);
    assert_eq!(document.states[7].ty, Type::MouseClick);
    assert_eq!(document.states[8].ty, Type::TouchFinger);
    assert!(matches!(
        &document.states[7].initial,
        Expr::Call { name, args } if name == "mouse.click" && args.len() == 3
    ));
    assert!(matches!(
        &document.handlers[0].statements[0],
        Statement::Assign {
            value: Expr::Call { name, args },
            ..
        } if name == "mouse.cursor_position" && args.len() == 1
    ));
}

#[test]
fn parses_native_transformations() {
    let source = include_str!("../../../../examples/iced-app/src/ui/transformation_values.ice");
    let document = parse(source).unwrap();
    assert_eq!(document.states[0].ty, Type::Transformation);
    assert_eq!(document.states[6].ty, Type::Vector);
    assert_eq!(document.states[11].ty, Type::Size);
    assert!(matches!(
        &document.states[4].initial,
        Expr::Call { name, args } if name == "transform.compose" && args.len() == 2
    ));
    assert!(matches!(
        &document.handlers[0].statements[4],
        Statement::Assign {
            value: Expr::Call { name, args },
            ..
        } if name == "transform.point" && args.len() == 2
    ));
}

#[test]
fn parses_native_geometry_values() {
    let source = include_str!("../../../../examples/iced-app/src/ui/geometry_values.ice");
    let document = parse(source).unwrap();
    let state_type = |name: &str| {
        document
            .states
            .iter()
            .find(|state| state.name == name)
            .map(|state| state.ty.clone())
            .unwrap()
    };
    assert_eq!(state_type("origin"), Type::Point);
    assert_eq!(state_type("snapped_point"), Type::PointU32);
    assert_eq!(state_type("exact_bounds"), Type::RectangleU32);
    assert_eq!(
        state_type("snapped_bounds"),
        Type::Option(Box::new(Type::RectangleU32))
    );
    assert_eq!(state_type("bounds_size"), Type::Size);
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assign {
                target,
                value: Expr::Binary { op: BinaryOp::Mul, .. },
                ..
            } if target == "scaled_bounds"
        )
    }));
}

#[test]
fn parses_native_padding_and_angles() {
    fn contains_remainder(expr: &Expr) -> bool {
        match expr {
            Expr::Binary {
                op: BinaryOp::Rem, ..
            } => true,
            Expr::Binary { left, right, .. } => {
                contains_remainder(left) || contains_remainder(right)
            }
            Expr::Unary { value, .. } => contains_remainder(value),
            Expr::Call { args, .. } | Expr::List(args) => args.iter().any(contains_remainder),
            _ => false,
        }
    }

    let source = include_str!("../../../../examples/iced-app/src/ui/padding_angles.ice");
    let document = parse(source).unwrap();
    let state_type = |name: &str| {
        document
            .states
            .iter()
            .find(|state| state.name == name)
            .map(|state| state.ty.clone())
            .unwrap()
    };
    assert_eq!(state_type("pixel_value"), Type::Pixels);
    assert_eq!(state_type("direct_padding"), Type::Padding);
    assert_eq!(state_type("degree_value"), Type::Degrees);
    assert_eq!(state_type("radians_value"), Type::Radians);
    assert!(document.handlers[0].statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::Assign { target, value, .. }
                if target == "radians_math" && contains_remainder(value)
        )
    }));
}

#[test]
fn parses_typed_task_streams() {
    let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream range(start:i64, limit:i64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(id:i64) -> str
  event-filter raw_event() -> str
theme
  background #000000
on start
  parallel
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
on number(value)
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(3) -> number _
  run range(1, 3) -> number _
  recipe snapshot(3) -> text _
  events 3 using=raw_event -> text _
view
  text "Streams"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Stream);
    assert_eq!(
        document.functions[2].error,
        Some(Type::Named("AppError".into()))
    );
    let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
        panic!("expected task group");
    };
    assert!(statements.iter().all(|statement| matches!(
        statement,
        Statement::Run {
            kind: EffectKind::Stream,
            ..
        }
    )));
    assert!(matches!(
        &document.subscriptions[0].source,
        SubscriptionSource::Run { function, args }
            if function == "fallible" && args.is_empty()
    ));
    assert!(matches!(
        &document.subscriptions[2].source,
        SubscriptionSource::Run { function, args }
            if function == "range" && args.len() == 2
    ));
    assert!(matches!(
        &document.subscriptions[3].source,
        SubscriptionSource::Recipe { function, args }
            if function == "snapshot" && args.len() == 1
    ));
    assert!(matches!(
        &document.subscriptions[4].source,
        SubscriptionSource::Events { id: Expr::I64(3), filter }
            if filter == "raw_event"
    ));

    let error = parse(&source.replace(
        "recipe snapshot(id:i64) -> str",
        "recipe snapshot(id:i64) -> str ! AppError",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E023");

    let error = parse(&source.replace(
        "event-filter raw_event() -> str",
        "event-filter raw_event(value:i64) -> str",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E022");

    let error =
        parse(&source.replace("stream numbers(3) -> number _", "stream numbers(3)")).unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("stream requires"));
}

#[test]
fn parses_generic_event_subscriptions() {
    let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on received(value)
on identified(id, value)
subscribe
  event -> received _
  event status=any -> received _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id -> identified _ _
view
  text "Events"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].params[0].1, Type::Event);
    assert!(matches!(
        document.subscriptions[0].source,
        SubscriptionSource::Event { raw: false }
    ));
    assert!(!document.subscriptions[0].window_id);
    assert_eq!(document.subscriptions[2].status, Some(EventStatus::Ignored));
    assert!(document.subscriptions[2].window_id);
    assert!(matches!(
        document.subscriptions[3].source,
        SubscriptionSource::Event { raw: true }
    ));
    assert!(document.subscriptions[4].window_id);

    let error =
        parse(&source.replace("event -> received _", "event redraw -> received _")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("event [raw] [with-id]"));
}

#[test]
fn parses_typed_task_sips() {
    let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip download(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
theme
  background #000000
on start
  parallel
    sip download(3)
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
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Sip);
    assert_eq!(document.functions[0].progress, Some(Type::F64));
    assert_eq!(
        document.functions[1].error,
        Some(Type::Named("AppError".into()))
    );
    let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
        panic!("expected task group");
    };
    assert!(
        statements
            .iter()
            .all(|statement| matches!(statement, Statement::Sip { .. }))
    );

    let error = parse(&source.replace("      progress -> advanced _\n", "")).unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("progress"));
}

#[test]
fn parses_structured_task_flows() {
    let source = r#"app Flows
extern crate::backend
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
theme
  background #000000
on start
  flow
    from stream numbers(3)
    then value -> task double(value)
    collect
    done -> collected _
    units -> planned _
on collected(values)
on planned(units)
view
  text "Flows"
"#;
    let document = parse(source).unwrap();
    let Statement::TaskFlow {
        source: task_source,
        transforms,
        success,
        units,
        ..
    } = &document.handlers[0].statements[0]
    else {
        panic!("expected task flow");
    };
    assert!(matches!(
        task_source,
        TaskSource::Effect {
            kind: EffectKind::Stream,
            ..
        }
    ));
    assert_eq!(transforms.len(), 2);
    assert!(matches!(transforms[0], TaskTransform::Then { .. }));
    assert!(matches!(transforms[1], TaskTransform::Collect { .. }));
    assert!(success.is_some());
    assert!(units.is_some());

    let error = parse(&source.replace("    from stream numbers(3)", "    collect")).unwrap_err();
    assert_eq!(error.code, "E050");
    assert!(error.message.contains("first flow line"));
}

#[test]
fn parses_task_error_mapping_and_native_sources() {
    let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
theme
  background #000000
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
on finished(value)
view
  text "Errors"
"#;
    let document = parse(source).unwrap();
    assert_eq!(document.functions[0].kind, ExternKind::Sync);
    assert_eq!(
        document.states[0].ty,
        Type::List(Box::new(Type::Result(
            Box::new(Type::I64),
            Box::new(Type::Named("AppError".into()))
        )))
    );
    let Statement::TaskGroup { statements, .. } = &document.handlers[0].statements[0] else {
        panic!("expected task group");
    };
    assert!(matches!(
        &statements[0],
        Statement::TaskFlow { transforms, .. }
            if matches!(transforms[0], TaskTransform::MapError { .. })
    ));
    assert!(matches!(
        &statements[1],
        Statement::TaskFlow {
            source: TaskSource::Done { .. },
            ..
        }
    ));
    assert!(matches!(
        &statements[2],
        Statement::TaskFlow {
            source: TaskSource::None {
                output: Type::I64,
                ..
            },
            ..
        }
    ));
}

#[test]
fn parses_checked_application_and_window_settings() {
    let source = SOURCE.replace(
        "app Demo",
        r##"app Demo
  title "Configured"
  theme "dark"
  background "#123456"
  text-color "#abcdef"
  id "dev.example.demo"
  executor iced::executor::Default
  font "assets/Brand.ttf"
  font "assets/Icons.otf"
  default-text-size 15
  antialiasing false
  vsync false
  scale-factor 1.25
  window
    icon-rgba "assets/app.rgba" 2 1
    size 960 720
    min-size 480 360
    max-size 1920 1080
    position centered
    level always-on-top
    visible true
    platform linux
      application-id "dev.example.demo"
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
        "executor iced::executor::Default",
        "executor iced::bad-path",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E073");

    let error = parse(&source.replace(
            "    platform linux\n      application-id \"dev.example.demo\"\n      override-redirect false",
            "    platform plan9\n      application-id \"dev.example.demo\"",
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
        "handle static code=\"⌄\" font=default size=12.0 line-height=1.0 shaping=basic",
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
    active text=foreground placeholder=muted handle=primary background=surface border=border border-width=1.0 radius=4.0
    hovered text=foreground
    opened text=foreground
    opened-hovered text=foreground
    menu text=foreground selected-text=foreground selected-background=primary background=surface shadow=black shadow-y=2.0
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
