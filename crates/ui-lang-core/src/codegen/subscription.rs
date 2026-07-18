use super::*;

pub(super) fn generate_subscription(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    if document.subscriptions.is_empty() {
        return Ok(());
    }
    let env = state_env(document, "self");
    writeln!(
        out,
        "fn __subscription(&self) -> ::iced::Subscription<{message}> {{"
    )
    .unwrap();
    writeln!(out, "::iced::Subscription::batch([").unwrap();
    for subscription in &document.subscriptions {
        let source_arity = subscription_payload_arity(&subscription.source, subscription.window_id);
        let filter = subscription
            .filter
            .as_ref()
            .map(|filter| {
                let function = document
                    .functions
                    .iter()
                    .find(|item| item.name == *filter && item.kind == ExternKind::Sync)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown subscription filter `{filter}`"),
                        )
                    })?;
                let args = match source_arity {
                    0 => String::new(),
                    1 => "__value".into(),
                    count => (0..count)
                        .map(|index| format!("__value.{index}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                };
                Ok(format!(
                    ".filter_map(|{}| {}({args}))",
                    if source_arity == 0 { "_" } else { "__value" },
                    function.rust_path
                ))
            })
            .transpose()?
            .unwrap_or_default();
        let context = subscription
            .context
            .as_ref()
            .map(|context| expr_code(context, &env, document, ValueMode::Owned))
            .transpose()?
            .map(|context| format!(".with({context})"))
            .unwrap_or_default();
        let output_arity = if subscription.filter.is_some() {
            1
        } else {
            source_arity
        };
        let mut payloads = Vec::new();
        if subscription.context.is_some() {
            payloads.push("__value.0".to_owned());
        }
        match output_arity {
            0 => {}
            1 => payloads.push(if subscription.context.is_some() {
                "__value.1".into()
            } else {
                "__value".into()
            }),
            count => payloads.extend((0..count).map(|index| {
                if subscription.context.is_some() {
                    format!("__value.1.{index}")
                } else {
                    format!("__value.{index}")
                }
            })),
        }
        let payloads = payloads.iter().map(String::as_str).collect::<Vec<_>>();
        let route = ordered_route_code(&subscription.route, &payloads, &env, document, message)?;
        let transforms = format!("{filter}{context}");
        let condition = subscription
            .condition
            .as_ref()
            .map(|condition| expr_code(condition, &env, document, ValueMode::Owned))
            .transpose()?;
        if let Some(condition) = &condition {
            write!(out, "if {condition} {{ ::iced::Subscription::batch([").unwrap();
        }
        match &subscription.source {
            SubscriptionSource::Every { milliseconds } => {
                writeln!(out, "::iced::time::every(::std::time::Duration::from_millis({milliseconds})){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Repeat {
                function,
                milliseconds,
            } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Future)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown repeated async function `{function}`"),
                        )
                    })?;
                writeln!(out, "::iced::time::repeat({}, ::std::time::Duration::from_millis({milliseconds})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
            }
            SubscriptionSource::Run { function, args } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Stream)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown subscription stream `{function}`"),
                        )
                    })?;
                if args.is_empty() {
                    writeln!(
                        out,
                        "::iced::Subscription::run({}){transforms}.map(move |__value| {route}),",
                        source.rust_path
                    )
                    .unwrap();
                } else {
                    let data = args
                        .iter()
                        .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
                        .collect::<Result<Vec<_>, _>>()?;
                    let types = source
                        .params
                        .iter()
                        .map(|(_, ty)| ty.rust(&document.structs))
                        .collect::<Vec<_>>();
                    let (data, data_type, builder_args) = if args.len() == 1 {
                        (data[0].clone(), types[0].clone(), "__data.clone()".into())
                    } else {
                        (
                            format!("({},)", data.join(", ")),
                            format!("({},)", types.join(", ")),
                            (0..args.len())
                                .map(|index| format!("__data.{index}.clone()"))
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    };
                    writeln!(out, "::iced::Subscription::run_with({data}, |__data: &{data_type}| {}({builder_args})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
                }
            }
            SubscriptionSource::Recipe { function, args } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Recipe)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown subscription recipe `{function}`"),
                        )
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                writeln!(out, "::iced::advanced::subscription::from_recipe({}({args})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
            }
            SubscriptionSource::Events { id, filter } => {
                let _source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *filter && item.kind == ExternKind::EventFilter)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown event filter `{filter}`"),
                        )
                    })?;
                let id = expr_code(id, &env, document, ValueMode::Owned)?;
                let recipe = format!("__IceEventFilter{}", pascal(filter));
                writeln!(out, "::iced::advanced::subscription::from_recipe({recipe} {{ id: {id} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Event { raw } => {
                if !*raw && subscription.status.is_none() && !subscription.window_id {
                    writeln!(
                        out,
                        "::iced::event::listen(){transforms}.map(move |__value| {route}),"
                    )
                    .unwrap();
                } else {
                    let value = if subscription.window_id {
                        "::std::option::Option::Some((__id, __event))"
                    } else {
                        "::std::option::Option::Some(__event)"
                    };
                    let status = if *raw || subscription.status.is_some() {
                        subscription.status
                    } else {
                        Some(EventStatus::Ignored)
                    };
                    let (filter, status) = event_status_filter(value, status);
                    let listen = if *raw { "listen_raw" } else { "listen_with" };
                    writeln!(out, "::iced::event::{listen}(|__event, {status}, __id| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                }
            }
            SubscriptionSource::Extern { function, args } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Subscription)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown extern subscription `{function}`"),
                        )
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                writeln!(
                    out,
                    "{}({args}){transforms}.map(move |__value| {route}),",
                    source.rust_path
                )
                .unwrap();
            }
            SubscriptionSource::InputMethod(event) => {
                let filter = match event {
                    InputMethodEvent::Opened => {
                        "matches!(__event, ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Opened)).then_some(())"
                    }
                    InputMethodEvent::Preedit => {
                        "match __event { ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Preedit(content, range)) => { let (start, end) = range.map_or((::std::option::Option::None, ::std::option::Option::None), |range| (::std::option::Option::Some(i64::try_from(range.start).unwrap_or(i64::MAX)), ::std::option::Option::Some(i64::try_from(range.end).unwrap_or(i64::MAX)))); ::std::option::Option::Some((content, start, end)) }, _ => ::std::option::Option::None }"
                    }
                    InputMethodEvent::Commit => {
                        "match __event { ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Commit(content)) => ::std::option::Option::Some(content), _ => ::std::option::Option::None }"
                    }
                    InputMethodEvent::Closed => {
                        "matches!(__event, ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Closed)).then_some(())"
                    }
                };
                let (filter, status) = event_status_filter(filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Keyboard(event) => {
                let filter = match event {
                    KeyboardEvent::Press => {
                        "match __event { ::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat } => ::std::option::Option::Some(__IceKeyPress { key, modified_key, physical_key, location, modifiers, text: text.map(|value| value.to_string()), repeat }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Release => {
                        "match __event { ::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers } => ::std::option::Option::Some(__IceKeyRelease { key, modified_key, physical_key, location, modifiers }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Modifiers => {
                        "match __event { ::iced::keyboard::Event::ModifiersChanged(modifiers) => ::std::option::Option::Some(modifiers), _ => ::std::option::Option::None }"
                    }
                };
                if subscription.status.is_some() {
                    let filter = format!(
                        "match __event {{ ::iced::Event::Keyboard(__event) => {{ {filter} }}, _ => ::std::option::Option::None }}"
                    );
                    let (filter, status) = event_status_filter(&filter, subscription.status);
                    writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                } else {
                    writeln!(out, "::iced::keyboard::listen().filter_map(|__event| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                }
            }
            SubscriptionSource::Mouse(event) => {
                let filter = match event {
                    MouseEvent::Entered => {
                        "matches!(__event, ::iced::Event::Mouse(::iced::mouse::Event::CursorEntered)).then_some(())"
                    }
                    MouseEvent::Left => {
                        "matches!(__event, ::iced::Event::Mouse(::iced::mouse::Event::CursorLeft)).then_some(())"
                    }
                    MouseEvent::Moved => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::CursorMoved { position }) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Pressed => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonPressed(button)) => ::std::option::Option::Some(button), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Released => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonReleased(button)) => ::std::option::Option::Some(button), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Wheel => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::WheelScrolled { delta }) => { let (x, y, pixels) = match delta { ::iced::mouse::ScrollDelta::Lines { x, y } => (x as f64, y as f64, false), ::iced::mouse::ScrollDelta::Pixels { x, y } => (x as f64, y as f64, true) }; ::std::option::Option::Some((x, y, pixels)) }, _ => ::std::option::Option::None }"
                    }
                };
                let (filter, status) = event_status_filter(filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::SystemTheme => {
                writeln!(out, "::iced::system::theme_changes().map(__ice_system_theme){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Touch(event) => {
                let variant = match event {
                    TouchEvent::Pressed => "FingerPressed",
                    TouchEvent::Moved => "FingerMoved",
                    TouchEvent::Lifted => "FingerLifted",
                    TouchEvent::Lost => "FingerLost",
                };
                let filter = format!(
                    "match __event {{ ::iced::Event::Touch(::iced::touch::Event::{variant} {{ id, position }}) => ::std::option::Option::Some((id, position.x as f64, position.y as f64)), _ => ::std::option::Option::None }}"
                );
                let (filter, status) = event_status_filter(&filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Window(event) => {
                if *event == WindowEvent::Frame {
                    writeln!(
                        out,
                        "::iced::window::frames(){transforms}.map(move |__value| {route}),"
                    )
                    .unwrap();
                    if condition.is_some() {
                        writeln!(out, "]) }} else {{ ::iced::Subscription::none() }},").unwrap();
                    }
                    continue;
                }
                let filter = match event {
                    WindowEvent::Opened => {
                        "match __event { ::iced::window::Event::Opened { position, size } => { let (x, y) = position.map_or((::std::option::Option::None, ::std::option::Option::None), |position| (::std::option::Option::Some(position.x as f64), ::std::option::Option::Some(position.y as f64))); ::std::option::Option::Some((x, y, size.width as f64, size.height as f64)) }, _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Closed => {
                        "matches!(__event, ::iced::window::Event::Closed).then_some(())"
                    }
                    WindowEvent::Moved => {
                        "match __event { ::iced::window::Event::Moved(position) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Resized => {
                        "match __event { ::iced::window::Event::Resized(size) => ::std::option::Option::Some((size.width as f64, size.height as f64)), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Rescaled => {
                        "match __event { ::iced::window::Event::Rescaled(scale) => ::std::option::Option::Some(scale as f64), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::CloseRequested => {
                        "matches!(__event, ::iced::window::Event::CloseRequested).then_some(())"
                    }
                    WindowEvent::Focused => {
                        "matches!(__event, ::iced::window::Event::Focused).then_some(())"
                    }
                    WindowEvent::Unfocused => {
                        "matches!(__event, ::iced::window::Event::Unfocused).then_some(())"
                    }
                    WindowEvent::FileHovered => {
                        "match __event { ::iced::window::Event::FileHovered(path) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::FileDropped => {
                        "match __event { ::iced::window::Event::FileDropped(path) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::FilesHoveredLeft => {
                        "matches!(__event, ::iced::window::Event::FilesHoveredLeft).then_some(())"
                    }
                    WindowEvent::Frame => unreachable!("handled above"),
                };
                let filter = if subscription.window_id {
                    identified_window_filter(
                        filter,
                        subscription_payload_arity(&subscription.source, false),
                    )
                } else {
                    filter.to_owned()
                };
                if subscription.status.is_some() {
                    let filter = format!(
                        "match __event {{ ::iced::Event::Window(__event) => {{ {filter} }}, _ => ::std::option::Option::None }}"
                    );
                    let (filter, status) = event_status_filter(&filter, subscription.status);
                    writeln!(out, "::iced::event::listen_with(|__event, {status}, __id| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                } else {
                    writeln!(out, "::iced::window::events().filter_map(|(__id, __event)| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                }
            }
        }
        if condition.is_some() {
            writeln!(out, "]) }} else {{ ::iced::Subscription::none() }},").unwrap();
        }
    }
    writeln!(out, "])\n}}").unwrap();
    Ok(())
}

fn event_status_filter(filter: &str, status: Option<EventStatus>) -> (String, &'static str) {
    match status {
        None | Some(EventStatus::Any) => (filter.to_owned(), "_"),
        Some(EventStatus::Captured) => (
            format!(
                "if matches!(__status, ::iced::event::Status::Captured) {{ {filter} }} else {{ ::std::option::Option::None }}"
            ),
            "__status",
        ),
        Some(EventStatus::Ignored) => (
            format!(
                "if matches!(__status, ::iced::event::Status::Ignored) {{ {filter} }} else {{ ::std::option::Option::None }}"
            ),
            "__status",
        ),
    }
}
