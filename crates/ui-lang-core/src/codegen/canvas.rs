use super::*;

pub(super) fn render_canvas(
    options: &CanvasOptions,
    locals: &[State],
    commands: &[CanvasCommand],
    events: &[CanvasEvent],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let state_fields = locals
        .iter()
        .map(|local| format!("{}: {},", local.name, local.ty.rust(&document.structs)))
        .collect::<Vec<_>>()
        .join(" ");
    let state_initials = locals
        .iter()
        .map(|local| {
            format!(
                "{}: {},",
                local.name,
                initial_code(&local.initial, &local.ty, document)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut canvas_env = env.clone();
    for local in locals {
        canvas_env.insert(
            local.name.clone(),
            Binding {
                code: format!("__state.{}", local.name),
                ty: local.ty.clone(),
                local: false,
            },
        );
    }
    canvas_env.insert(
        "canvas_width".into(),
        Binding {
            code: "(__bounds.width as f64)".into(),
            ty: Type::F64,
            local: true,
        },
    );
    canvas_env.insert(
        "canvas_height".into(),
        Binding {
            code: "(__bounds.height as f64)".into(),
            ty: Type::F64,
            local: true,
        },
    );
    let draw_commands = canvas_commands_code(commands, &canvas_env, document)?;
    let use_cache = options.cache.is_some();
    let cache_key = if let Some(dependency) = &options.cache {
        let dependency = expr_code(dependency, env, document, ValueMode::Owned)?;
        format!(
            "::std::option::Option::Some({{ let mut __hasher = ::std::hash::DefaultHasher::new(); ::std::hash::Hash::hash(&({dependency}), &mut __hasher); ::std::hash::Hasher::finish(&__hasher) }})"
        )
    } else {
        "::std::option::Option::None".into()
    };
    let update = canvas_update_code(
        options,
        events,
        env,
        &canvas_env,
        document,
        message,
        use_cache,
    )?;
    let interaction = if let Some(interaction) = &options.interaction_expr {
        let interaction = expr_code(interaction, &canvas_env, document, ValueMode::Owned)?;
        format!(
            "{{ let __interaction = {interaction}; __ice_canvas_interaction(__interaction.as_str()) }}"
        )
    } else {
        format!(
            "::iced::mouse::Interaction::{}",
            options
                .interaction
                .map(mouse_interaction_code)
                .unwrap_or("None")
        )
    };
    let interaction_outside = options
        .interaction_outside
        .as_ref()
        .map(|outside| expr_code(outside, &canvas_env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let cache_group = options.cache_group.as_ref().map_or_else(
        || "::std::option::Option::None".into(),
        |group| {
            format!(
                "::std::option::Option::Some(*{}.get_or_init(::iced::widget::canvas::Group::unique))",
                canvas_group_symbol(group)
            )
        },
    );
    let cache_setup = if use_cache {
        "let __cache = __state.cache.get_or_init(|| match __cache_group { ::std::option::Option::Some(group) => ::iced::widget::canvas::Cache::with_group(group), ::std::option::Option::None => ::iced::widget::canvas::Cache::new() }); if __state.cache_key.get() != __cache_key { __cache.clear(); __state.cache_key.set(__cache_key); }"
    } else {
        ""
    };
    let geometry = if use_cache {
        "__cache.draw(__renderer, __bounds.size(), __paint)"
    } else {
        "{ let mut __frame = ::iced::widget::canvas::Frame::new(__renderer, __bounds.size()); __paint(&mut __frame); __frame.into_geometry() }"
    };
    let mut code = format!(
        "{{ #[allow(dead_code)] struct __IceCanvasState {{ cache: ::std::cell::OnceCell<::iced::widget::canvas::Cache>, cache_key: ::std::cell::Cell<::std::option::Option<u64>>, inside: bool, {state_fields} }} impl ::std::default::Default for __IceCanvasState {{ fn default() -> Self {{ Self {{ cache: ::std::cell::OnceCell::new(), cache_key: ::std::cell::Cell::new(::std::option::Option::None), inside: false, {state_initials} }} }} }} let __cache_key: ::std::option::Option<u64> = {cache_key}; let __cache_group: ::std::option::Option<::iced::widget::canvas::Group> = {cache_group}; let __program = __IceCanvasProgram::<__IceCanvasState, {message}, _, _, _> {{ draw: move |__state: &__IceCanvasState, __renderer: &::iced::Renderer, __theme: &::iced::Theme, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ let _ = (&__cache_key, &__cache_group); {cache_setup} let __paint = move |__frame: &mut ::iced::widget::canvas::Frame| {{ {draw_commands} }}; let __geometry = {geometry}; ::std::vec![__geometry] }}, update: {update}, interaction: move |__state: &__IceCanvasState, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ if ({interaction_outside}) || __cursor.is_over(__bounds) {{ {interaction} }} else {{ ::iced::mouse::Interaction::default() }} }}, message: ::std::marker::PhantomData }}; let __canvas = ::iced::widget::canvas(__program)"
    );
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.height {
        write!(code, ".height({})", length_code(height, env, document)?).unwrap();
    }
    code.push_str("; __canvas.into() }");
    Ok(code)
}

pub(super) fn canvas_update_code(
    options: &CanvasOptions,
    events: &[CanvasEvent],
    env: &HashMap<String, Binding>,
    canvas_env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
    use_cache: bool,
) -> Result<String, Error> {
    let capture = options
        .capture
        .as_ref()
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let action = |message: String, capture: &str| {
        format!(
            "::std::option::Option::Some(if {capture} {{ ::iced::widget::canvas::Action::publish({message}).and_capture() }} else {{ ::iced::widget::canvas::Action::publish({message}) }})"
        )
    };
    let mut code = format!(
        "move |__state: &mut __IceCanvasState, __event: &::iced::widget::canvas::Event, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ let __capture = {capture};"
    );
    if options.enter.is_some() || options.exit.is_some() {
        code.push_str(" let __inside = __cursor.is_over(__bounds); if __inside != __state.inside { __state.inside = __inside;");
        if let Some(route) = &options.enter {
            let route = route_code(route, "", env, document, message)?;
            write!(
                code,
                " if __inside {{ return {}; }}",
                action(route, "__capture")
            )
            .unwrap();
        }
        if let Some(route) = &options.exit {
            let route = route_code(route, "", env, document, message)?;
            write!(
                code,
                " if !__inside {{ return {}; }}",
                action(route, "__capture")
            )
            .unwrap();
        }
        code.push_str(" }");
    }
    let has_pointer_routes = options.press.is_some()
        || options.release.is_some()
        || options.right_press.is_some()
        || options.right_release.is_some()
        || options.middle_press.is_some()
        || options.middle_release.is_some()
        || options.move_route.is_some()
        || options.scroll.is_some();
    if has_pointer_routes {
        code.push_str(
            " if let ::std::option::Option::Some(__point) = __cursor.position_in(__bounds) { match __event {",
        );
        for (route, event) in [
            (
                &options.press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Left))",
            ),
            (
                &options.release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Left))",
            ),
            (
                &options.right_press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Right))",
            ),
            (
                &options.right_release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Right))",
            ),
            (
                &options.middle_press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Middle))",
            ),
            (
                &options.middle_release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Middle))",
            ),
        ] {
            if let Some(route) = route {
                let route = ordered_route_code(
                    route,
                    &["__point.x as f64", "__point.y as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, " {event} => return {},", action(route, "__capture")).unwrap();
            }
        }
        if let Some(route) = &options.move_route {
            let route = ordered_route_code(
                route,
                &["__point.x as f64", "__point.y as f64"],
                env,
                document,
                message,
            )?;
            write!(
                code,
                " ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorMoved {{ .. }}) => return {},",
                action(route, "__capture")
            )
            .unwrap();
        }
        if let Some(route) = &options.scroll {
            let lines = ordered_route_code(
                route,
                &["__x as f64", "__y as f64", "false"],
                env,
                document,
                message,
            )?;
            let pixels = ordered_route_code(
                route,
                &["__x as f64", "__y as f64", "true"],
                env,
                document,
                message,
            )?;
            write!(
                code,
                " ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::WheelScrolled {{ delta }}) => return match delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {} }},",
                action(lines, "__capture"),
                action(pixels, "__capture")
            )
            .unwrap();
        }
        code.push_str(" _ => {} } }");
    }
    for event in events {
        let filter = canvas_event_filter(&event.source);
        let payloads = canvas_event_payload_types(&event.source);
        let mut event_env = canvas_env.clone();
        for (binding, ty) in event.bindings.iter().zip(payloads) {
            event_env.insert(
                binding.clone(),
                Binding {
                    code: binding.clone(),
                    ty,
                    local: false,
                },
            );
        }
        let bindings = match event.bindings.as_slice() {
            [] => String::new(),
            [binding] => format!("let {binding} = __value;"),
            bindings => format!("let ({}) = __value;", bindings.join(", ")),
        };
        let mut updates = event
            .updates
            .iter()
            .enumerate()
            .map(|(index, update)| {
                Ok(format!(
                    "let __next_canvas_state_{index} = {}; __state.{} = __next_canvas_state_{index};",
                    expr_code(&update.value, &event_env, document, ValueMode::Owned)?,
                    update.name,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?
            .join(" ");
        if use_cache && !event.updates.is_empty() {
            updates.push_str(
                " if let ::std::option::Option::Some(__cache) = __state.cache.get() { __cache.clear(); }",
            );
        }
        let event_capture = if event.capture { "true" } else { "__capture" };
        let result = match &event.action {
            Some(CanvasEventAction::Route(route)) => {
                let route = if event.route_payload {
                    canvas_event_route_code(&event.source, route, env, document, message)?
                } else {
                    route_code(route, "", &event_env, document, message)?
                };
                action(route, event_capture)
            }
            Some(CanvasEventAction::Redraw { after_ms }) => {
                let redraw = after_ms.map_or_else(
                    || "::iced::widget::canvas::Action::request_redraw()".into(),
                    |milliseconds| {
                        format!(
                            "::iced::widget::canvas::Action::request_redraw_at(::iced::time::Instant::now() + ::iced::time::Duration::from_millis({milliseconds}))"
                        )
                    },
                );
                format!(
                    "::std::option::Option::Some(if {event_capture} {{ {redraw}.and_capture() }} else {{ {redraw} }})"
                )
            }
            None => format!(
                "if {event_capture} {{ ::std::option::Option::Some(::iced::widget::canvas::Action::capture()) }} else {{ ::std::option::Option::None }}"
            ),
        };
        write!(
            code,
            " if let ::std::option::Option::Some(__value) = {filter} {{ let _ = &__value; {bindings} {updates} return {result}; }}"
        )
        .unwrap();
    }
    code.push_str(" ::std::option::Option::None }");
    Ok(code)
}

pub(super) fn canvas_event_filter(source: &SubscriptionSource) -> String {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened => "matches!(__event, ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Opened)).then_some(())".into(),
            InputMethodEvent::Preedit => "match __event { ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Preedit(content, range)) => { let (start, end) = range.as_ref().map_or((::std::option::Option::None, ::std::option::Option::None), |range| (::std::option::Option::Some(i64::try_from(range.start).unwrap_or(i64::MAX)), ::std::option::Option::Some(i64::try_from(range.end).unwrap_or(i64::MAX)))); ::std::option::Option::Some((content.clone(), start, end)) }, _ => ::std::option::Option::None }".into(),
            InputMethodEvent::Commit => "match __event { ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Commit(content)) => ::std::option::Option::Some(content.clone()), _ => ::std::option::Option::None }".into(),
            InputMethodEvent::Closed => "matches!(__event, ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Closed)).then_some(())".into(),
        },
        SubscriptionSource::Keyboard(event) => match event {
            KeyboardEvent::Press => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat }) => ::std::option::Option::Some(__IceKeyPress { key: key.clone(), modified_key: modified_key.clone(), physical_key: *physical_key, location: *location, modifiers: *modifiers, text: text.as_ref().map(::std::string::ToString::to_string), repeat: *repeat }), _ => ::std::option::Option::None }".into(),
            KeyboardEvent::Release => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers }) => ::std::option::Option::Some(__IceKeyRelease { key: key.clone(), modified_key: modified_key.clone(), physical_key: *physical_key, location: *location, modifiers: *modifiers }), _ => ::std::option::Option::None }".into(),
            KeyboardEvent::Modifiers => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::ModifiersChanged(modifiers)) => ::std::option::Option::Some(*modifiers), _ => ::std::option::Option::None }".into(),
        },
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered => "matches!(__event, ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorEntered)).then_some(())".into(),
            MouseEvent::Left => "matches!(__event, ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorLeft)).then_some(())".into(),
            MouseEvent::Moved => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorMoved { position }) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }".into(),
            MouseEvent::Pressed => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(button)) => ::std::option::Option::Some(*button), _ => ::std::option::Option::None }".into(),
            MouseEvent::Released => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(button)) => ::std::option::Option::Some(*button), _ => ::std::option::Option::None }".into(),
            MouseEvent::Wheel => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::WheelScrolled { delta }) => { let (x, y, pixels) = match delta { ::iced::mouse::ScrollDelta::Lines { x, y } => (*x as f64, *y as f64, false), ::iced::mouse::ScrollDelta::Pixels { x, y } => (*x as f64, *y as f64, true) }; ::std::option::Option::Some((x, y, pixels)) }, _ => ::std::option::Option::None }".into(),
        },
        SubscriptionSource::Touch(event) => {
            let variant = match event {
                TouchEvent::Pressed => "FingerPressed",
                TouchEvent::Moved => "FingerMoved",
                TouchEvent::Lifted => "FingerLifted",
                TouchEvent::Lost => "FingerLost",
            };
            format!("match __event {{ ::iced::widget::canvas::Event::Touch(::iced::touch::Event::{variant} {{ id, position }}) => ::std::option::Option::Some((*id, position.x as f64, position.y as f64)), _ => ::std::option::Option::None }}")
        }
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Frame => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::RedrawRequested(_))).then_some(())".into(),
            WindowEvent::Opened => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Opened { position, size }) => { let (x, y) = position.as_ref().map_or((::std::option::Option::None, ::std::option::Option::None), |position| (::std::option::Option::Some(position.x as f64), ::std::option::Option::Some(position.y as f64))); ::std::option::Option::Some((x, y, size.width as f64, size.height as f64)) }, _ => ::std::option::Option::None }".into(),
            WindowEvent::Closed => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Closed)).then_some(())".into(),
            WindowEvent::Moved => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Moved(position)) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }".into(),
            WindowEvent::Resized => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Resized(size)) => ::std::option::Option::Some((size.width as f64, size.height as f64)), _ => ::std::option::Option::None }".into(),
            WindowEvent::Rescaled => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Rescaled(scale)) => ::std::option::Option::Some(*scale as f64), _ => ::std::option::Option::None }".into(),
            WindowEvent::CloseRequested => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::CloseRequested)).then_some(())".into(),
            WindowEvent::Focused => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Focused)).then_some(())".into(),
            WindowEvent::Unfocused => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Unfocused)).then_some(())".into(),
            WindowEvent::FileHovered => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::FileHovered(path)) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }".into(),
            WindowEvent::FileDropped => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::FileDropped(path)) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }".into(),
            WindowEvent::FilesHoveredLeft => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::FilesHoveredLeft)).then_some(())".into(),
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

pub(super) fn canvas_event_payload_types(source: &SubscriptionSource) -> Vec<Type> {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => Vec::new(),
            InputMethodEvent::Preedit => vec![
                Type::Str,
                Type::Option(Box::new(Type::I64)),
                Type::Option(Box::new(Type::I64)),
            ],
            InputMethodEvent::Commit => vec![Type::Str],
        },
        SubscriptionSource::Keyboard(event) => vec![match event {
            KeyboardEvent::Press => Type::KeyPress,
            KeyboardEvent::Release => Type::KeyRelease,
            KeyboardEvent::Modifiers => Type::KeyModifiers,
        }],
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => Vec::new(),
            MouseEvent::Moved => vec![Type::F64, Type::F64],
            MouseEvent::Pressed | MouseEvent::Released => vec![Type::MouseButton],
            MouseEvent::Wheel => vec![Type::F64, Type::F64, Type::Bool],
        },
        SubscriptionSource::Touch(_) => vec![Type::TouchFinger, Type::F64, Type::F64],
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => Vec::new(),
            WindowEvent::Opened => vec![
                Type::Option(Box::new(Type::F64)),
                Type::Option(Box::new(Type::F64)),
                Type::F64,
                Type::F64,
            ],
            WindowEvent::Moved | WindowEvent::Resized => vec![Type::F64, Type::F64],
            WindowEvent::Rescaled => vec![Type::F64],
            WindowEvent::FileHovered | WindowEvent::FileDropped => vec![Type::Str],
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

pub(super) fn canvas_event_route_code(
    source: &SubscriptionSource,
    route: &Route,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => {
                route_code(route, "", env, document, message)
            }
            InputMethodEvent::Preedit => ordered_route_code(
                route,
                &["__value.0", "__value.1", "__value.2"],
                env,
                document,
                message,
            ),
            InputMethodEvent::Commit => route_code(route, "__value", env, document, message),
        },
        SubscriptionSource::Keyboard(_) => route_code(route, "__value", env, document, message),
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => route_code(route, "", env, document, message),
            MouseEvent::Moved => {
                ordered_route_code(route, &["__value.0", "__value.1"], env, document, message)
            }
            MouseEvent::Pressed | MouseEvent::Released => {
                route_code(route, "__value", env, document, message)
            }
            MouseEvent::Wheel => ordered_route_code(
                route,
                &["__value.0", "__value.1", "__value.2"],
                env,
                document,
                message,
            ),
        },
        SubscriptionSource::Touch(_) => ordered_route_code(
            route,
            &["__value.0", "__value.1", "__value.2"],
            env,
            document,
            message,
        ),
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Opened => ordered_route_code(
                route,
                &["__value.0", "__value.1", "__value.2", "__value.3"],
                env,
                document,
                message,
            ),
            WindowEvent::Moved | WindowEvent::Resized => {
                ordered_route_code(route, &["__value.0", "__value.1"], env, document, message)
            }
            WindowEvent::Rescaled | WindowEvent::FileHovered | WindowEvent::FileDropped => {
                route_code(route, "__value", env, document, message)
            }
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => route_code(route, "", env, document, message),
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

pub(super) fn canvas_commands_code(
    commands: &[CanvasCommand],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::new();
    for command in commands {
        match command {
            CanvasCommand::Rectangle {
                x,
                y,
                width,
                height,
                radius,
                paint,
                ..
            } => {
                let point = canvas_point_code(x, y, env, document)?;
                let size = canvas_size_code(width, height, env, document)?;
                if canvas_radius_is_empty(radius) {
                    if let Some(fill) = &paint.fill {
                        write!(
                            code,
                            " __frame.fill_rectangle({point}, {size}, {});",
                            canvas_fill_code(fill, paint.fill_rule, env, document)?
                        )
                        .unwrap();
                    }
                    if let Some(stroke) = &paint.stroke {
                        write!(
                            code,
                            " __frame.stroke_rectangle({point}, {size}, {});",
                            canvas_stroke_code(stroke, env, document)?
                        )
                        .unwrap();
                    }
                } else {
                    let radius = canvas_radius_code(radius, env, document)?;
                    write!(
                        code,
                        " {{ let __path = ::iced::widget::canvas::Path::rounded_rectangle({point}, {size}, {radius}); {} }}",
                        canvas_paint_code(paint, "&__path", env, document)?
                    )
                    .unwrap();
                }
            }
            CanvasCommand::Circle {
                x,
                y,
                radius,
                paint,
                ..
            } => {
                let point = canvas_point_code(x, y, env, document)?;
                let radius = canvas_expr_code(radius, env, document)?;
                write!(
                    code,
                    " {{ let __path = ::iced::widget::canvas::Path::circle({point}, {radius} as f32); {} }}",
                    canvas_paint_code(paint, "&__path", env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                ..
            } => {
                let from = canvas_point_code(x1, y1, env, document)?;
                let to = canvas_point_code(x2, y2, env, document)?;
                write!(
                    code,
                    " {{ let __path = ::iced::widget::canvas::Path::line({from}, {to}); __frame.stroke(&__path, {}); }}",
                    canvas_stroke_code(stroke, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Text {
                value,
                x,
                y,
                max_width,
                color,
                size,
                line_height,
                font,
                align_x,
                align_y,
                shaping,
                span,
            } => {
                let ty = expr_type(
                    value,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?;
                let value = expr_code(value, env, document, ValueMode::Owned)?;
                let content = if ty == Type::Str {
                    value
                } else {
                    format!("::std::format!(\"{{}}\", {value})")
                };
                let position = canvas_point_code(x, y, env, document)?;
                let max_width = max_width
                    .as_ref()
                    .map(|value| canvas_expr_code(value, env, document))
                    .transpose()?
                    .map_or_else(|| "f32::INFINITY".into(), |value| format!("{value} as f32"));
                let color = color.as_ref().map_or_else(
                    || theme_color(document, "foreground"),
                    |color| theme_color(document, color),
                );
                let size = size
                    .as_ref()
                    .map(|value| canvas_expr_code(value, env, document))
                    .transpose()?
                    .unwrap_or_else(|| "16.0".into());
                let line_height = match line_height {
                    Some(TextLineHeight::Relative(value)) => format!(
                        "::iced::widget::text::LineHeight::Relative({} as f32)",
                        canvas_expr_code(value, env, document)?
                    ),
                    Some(TextLineHeight::Absolute(value)) => format!(
                        "::iced::widget::text::LineHeight::Absolute(::iced::Pixels({} as f32))",
                        canvas_expr_code(value, env, document)?
                    ),
                    None => "::iced::widget::text::LineHeight::default()".into(),
                };
                let font = font
                    .as_ref()
                    .map(|font| font_preset_code(font, document))
                    .transpose()?
                    .unwrap_or_else(|| "::iced::Font::DEFAULT".into());
                let align_x = align_x.map_or("Default", |value| text_alignment_code(value));
                let align_y = match align_y {
                    None | Some(VerticalAlignment::Top) => "Top",
                    Some(VerticalAlignment::Center) => "Center",
                    Some(VerticalAlignment::Bottom) => "Bottom",
                };
                let shaping = shaping.map_or("Auto", text_shaping_code);
                write!(
                    code,
                    " __frame.fill_text(::iced::widget::canvas::Text {{ content: {content}, position: {position}, max_width: {max_width}, color: {color}, size: ::iced::Pixels({size} as f32), line_height: {line_height}, font: {font}, align_x: ::iced::widget::text::Alignment::{align_x}, align_y: ::iced::alignment::Vertical::{align_y}, shaping: ::iced::widget::text::Shaping::{shaping} }});"
                )
                .unwrap();
            }
            CanvasCommand::Image {
                source,
                x,
                y,
                width,
                height,
                filter,
                rotation,
                opacity,
                snap,
                radius,
                span,
            } => {
                let source_ty = expr_type(
                    source,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?;
                let source = expr_code(source, env, document, ValueMode::Owned)?;
                let handle = if source_ty == Type::Str {
                    format!("::iced::widget::image::Handle::from_path({source})")
                } else {
                    source
                };
                let filter = match filter {
                    ImageFilter::Linear => "Linear",
                    ImageFilter::Nearest => "Nearest",
                };
                write!(
                    code,
                    " __frame.draw_image(::iced::Rectangle::new({}, {}), ::iced::widget::canvas::Image {{ handle: {handle}, filter_method: ::iced::widget::image::FilterMethod::{filter}, rotation: ::iced::Radians({} as f32), border_radius: {}, opacity: {} as f32, snap: {} }});",
                    canvas_point_code(x, y, env, document)?,
                    canvas_size_code(width, height, env, document)?,
                    canvas_expr_code(rotation, env, document)?,
                    canvas_radius_code(radius, env, document)?,
                    canvas_expr_code(opacity, env, document)?,
                    canvas_expr_code(snap, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Svg {
                source,
                memory,
                x,
                y,
                width,
                height,
                color,
                rotation,
                opacity,
                span,
            } => {
                let source_ty = expr_type(
                    source,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?;
                let source = expr_code(source, env, document, ValueMode::Owned)?;
                let handle = if *memory && source_ty == Type::Bytes {
                    format!("::iced::advanced::svg::Handle::from_memory({source})")
                } else if *memory {
                    format!("::iced::advanced::svg::Handle::from_memory(({source}).into_bytes())")
                } else {
                    format!("::iced::advanced::svg::Handle::from_path({source})")
                };
                let color = color.as_ref().map_or_else(
                    || "::std::option::Option::None".into(),
                    |color| {
                        format!(
                            "::std::option::Option::Some({})",
                            theme_color(document, color)
                        )
                    },
                );
                write!(
                    code,
                    " __frame.draw_svg(::iced::Rectangle::new({}, {}), ::iced::advanced::svg::Svg {{ handle: {handle}, color: {color}, rotation: ::iced::Radians({} as f32), opacity: {} as f32 }});",
                    canvas_point_code(x, y, env, document)?,
                    canvas_size_code(width, height, env, document)?,
                    canvas_expr_code(rotation, env, document)?,
                    canvas_expr_code(opacity, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Path {
                segments, paint, ..
            } => {
                let path = canvas_path_code(segments, env, document)?;
                write!(
                    code,
                    " {{ let __path = {path}; {} }}",
                    canvas_paint_code(paint, "&__path", env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Group {
                transform,
                commands,
                ..
            } => {
                let inner = canvas_commands_code(commands, env, document)?;
                let mut body = String::new();
                if transform.x.is_some() || transform.y.is_some() {
                    let x = transform
                        .x
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "0.0".into());
                    let y = transform
                        .y
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "0.0".into());
                    write!(
                        body,
                        " __frame.translate(::iced::Vector::new({x} as f32, {y} as f32));"
                    )
                    .unwrap();
                }
                if let Some(value) = &transform.rotate {
                    write!(
                        body,
                        " __frame.rotate({} as f32);",
                        canvas_expr_code(value, env, document)?
                    )
                    .unwrap();
                }
                if let Some(value) = &transform.scale {
                    write!(
                        body,
                        " __frame.scale({} as f32);",
                        canvas_expr_code(value, env, document)?
                    )
                    .unwrap();
                }
                if transform.scale_x.is_some() || transform.scale_y.is_some() {
                    let x = transform
                        .scale_x
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "1.0".into());
                    let y = transform
                        .scale_y
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "1.0".into());
                    write!(
                        body,
                        " __frame.scale_nonuniform(::iced::Vector::new({x} as f32, {y} as f32));"
                    )
                    .unwrap();
                }
                if let Some([x, y, width, height]) = &transform.clip {
                    let point = canvas_point_code(x, y, env, document)?;
                    let size = canvas_size_code(width, height, env, document)?;
                    write!(
                        body,
                        " __frame.with_clip(::iced::Rectangle {{ x: {point}.x, y: {point}.y, width: {size}.width, height: {size}.height }}, |__frame| {{ {inner} }});"
                    )
                    .unwrap();
                } else {
                    body.push_str(&inner);
                }
                write!(code, " __frame.with_save(|__frame| {{ {body} }});").unwrap();
            }
            CanvasCommand::If {
                condition,
                commands,
                ..
            } => {
                let condition = expr_code(condition, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    " if {condition} {{ {} }}",
                    canvas_commands_code(commands, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::For {
                item,
                items,
                commands,
                span,
            } => {
                let Type::List(inner) = expr_type(
                    items,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?
                else {
                    return Err(Error::new("E190", span, "canvas for expects a list"));
                };
                let items = expr_code(items, env, document, ValueMode::Borrowed)?;
                let mut child_env = env.clone();
                child_env.insert(
                    item.clone(),
                    Binding {
                        code: item.clone(),
                        ty: *inner,
                        local: false,
                    },
                );
                write!(
                    code,
                    " for {item} in {items}.iter() {{ {} }}",
                    canvas_commands_code(commands, &child_env, document)?
                )
                .unwrap();
            }
        }
    }
    Ok(code)
}

pub(super) fn canvas_path_code(
    segments: &[CanvasPathSegment],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::from("::iced::widget::canvas::Path::new(|__path| {");
    for segment in segments {
        match segment {
            CanvasPathSegment::Move(x, y) => write!(
                code,
                " __path.move_to({});",
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Line(x, y) => write!(
                code,
                " __path.line_to({});",
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => write!(
                code,
                " __path.arc(::iced::widget::canvas::path::Arc {{ center: {}, radius: {} as f32, start_angle: ::iced::Radians({} as f32), end_angle: ::iced::Radians({} as f32) }});",
                canvas_point_code(x, y, env, document)?,
                canvas_expr_code(radius, env, document)?,
                canvas_expr_code(start, env, document)?,
                canvas_expr_code(end, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => write!(
                code,
                " __path.arc_to({}, {}, {} as f32);",
                canvas_point_code(ax, ay, env, document)?,
                canvas_point_code(bx, by, env, document)?,
                canvas_expr_code(radius, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                rotation,
                start,
                end,
            } => write!(
                code,
                " __path.ellipse(::iced::widget::canvas::path::arc::Elliptical {{ center: {}, radii: ::iced::Vector::new({} as f32, {} as f32), rotation: ::iced::Radians({} as f32), start_angle: ::iced::Radians({} as f32), end_angle: ::iced::Radians({} as f32) }});",
                canvas_point_code(x, y, env, document)?,
                canvas_expr_code(radius_x, env, document)?,
                canvas_expr_code(radius_y, env, document)?,
                canvas_expr_code(rotation, env, document)?,
                canvas_expr_code(start, env, document)?,
                canvas_expr_code(end, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => write!(
                code,
                " __path.bezier_curve_to({}, {}, {});",
                canvas_point_code(control_ax, control_ay, env, document)?,
                canvas_point_code(control_bx, control_by, env, document)?,
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => write!(
                code,
                " __path.quadratic_curve_to({}, {});",
                canvas_point_code(control_x, control_y, env, document)?,
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            } => write!(
                code,
                " __path.rectangle({}, {});",
                canvas_point_code(x, y, env, document)?,
                canvas_size_code(width, height, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                radius,
            } => write!(
                code,
                " __path.rounded_rectangle({}, {}, {});",
                canvas_point_code(x, y, env, document)?,
                canvas_size_code(width, height, env, document)?,
                canvas_radius_code(radius, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Circle { x, y, radius } => write!(
                code,
                " __path.circle({}, {} as f32);",
                canvas_point_code(x, y, env, document)?,
                canvas_expr_code(radius, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Close => code.push_str(" __path.close();"),
        }
    }
    code.push_str(" })");
    Ok(code)
}

pub(super) fn canvas_paint_code(
    paint: &CanvasPaint,
    path: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::new();
    if let Some(fill) = &paint.fill {
        write!(
            code,
            " __frame.fill({path}, {});",
            canvas_fill_code(fill, paint.fill_rule, env, document)?
        )
        .unwrap();
    }
    if let Some(stroke) = &paint.stroke {
        write!(
            code,
            " __frame.stroke({path}, {});",
            canvas_stroke_code(stroke, env, document)?
        )
        .unwrap();
    }
    Ok(code)
}

pub(super) fn canvas_fill_code(
    fill: &BackgroundValue,
    rule: CanvasFillRule,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let rule = match rule {
        CanvasFillRule::NonZero => "NonZero",
        CanvasFillRule::EvenOdd => "EvenOdd",
    };
    Ok(format!(
        "::iced::widget::canvas::Fill {{ style: {}, rule: ::iced::widget::canvas::fill::Rule::{rule} }}",
        canvas_style_code(fill, env, document)?
    ))
}

pub(super) fn canvas_stroke_code(
    stroke: &CanvasStroke,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let cap = match stroke.cap {
        CanvasLineCap::Butt => "Butt",
        CanvasLineCap::Square => "Square",
        CanvasLineCap::Round => "Round",
    };
    let join = match stroke.join {
        CanvasLineJoin::Miter => "Miter",
        CanvasLineJoin::Round => "Round",
        CanvasLineJoin::Bevel => "Bevel",
    };
    let dash = stroke
        .dash
        .iter()
        .map(|value| canvas_expr_code(value, env, document).map(|value| format!("{value} as f32")))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!(
        "::iced::widget::canvas::Stroke {{ style: {}, width: {} as f32, line_cap: ::iced::widget::canvas::LineCap::{cap}, line_join: ::iced::widget::canvas::LineJoin::{join}, line_dash: ::iced::widget::canvas::LineDash {{ segments: &[{dash}], offset: usize::try_from({}).unwrap_or(usize::MAX) }} }}",
        canvas_style_code(&stroke.style, env, document)?,
        canvas_expr_code(&stroke.width, env, document)?,
        canvas_expr_code(&stroke.dash_offset, env, document)?
    ))
}

pub(super) fn canvas_style_code(
    style: &BackgroundValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match style {
        BackgroundValue::Color(color) => format!(
            "::iced::widget::canvas::Style::Solid({})",
            theme_color(document, color)
        ),
        BackgroundValue::Linear { angle, stops } => {
            let mut gradient =
                String::from("::iced::widget::canvas::gradient::Linear::new(__start, __end)");
            for stop in stops {
                write!(
                    gradient,
                    ".add_stop({} as f32, {})",
                    canvas_expr_code(&stop.offset, env, document)?,
                    theme_color(document, &stop.color)
                )
                .unwrap();
            }
            format!(
                "{{ let __angle = {} as f32; let __direction = ::iced::Vector::new(__angle.cos(), __angle.sin()); let __center = ::iced::Point::new(__bounds.width / 2.0, __bounds.height / 2.0); let __extent = (__bounds.width * __direction.x.abs() + __bounds.height * __direction.y.abs()) / 2.0; let __start = ::iced::Point::new(__center.x - __direction.x * __extent, __center.y - __direction.y * __extent); let __end = ::iced::Point::new(__center.x + __direction.x * __extent, __center.y + __direction.y * __extent); ::iced::widget::canvas::Style::Gradient(::iced::widget::canvas::Gradient::Linear({gradient})) }}",
                canvas_expr_code(angle, env, document)?
            )
        }
    })
}

pub(super) fn canvas_radius_is_empty(radius: &CanvasRadius) -> bool {
    radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
}

pub(super) fn canvas_radius_code(
    radius: &CanvasRadius,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    radius_code(
        radius.all.as_ref(),
        [
            radius.top_left.as_ref(),
            radius.top_right.as_ref(),
            radius.bottom_right.as_ref(),
            radius.bottom_left.as_ref(),
        ],
        env,
        document,
    )
    .map(|radius| radius.unwrap_or_else(|| "::iced::border::Radius::default()".into()))
}

pub(super) fn canvas_point_code(
    x: &Expr,
    y: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Point::new({} as f32, {} as f32)",
        canvas_expr_code(x, env, document)?,
        canvas_expr_code(y, env, document)?
    ))
}

pub(super) fn canvas_size_code(
    width: &Expr,
    height: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Size::new({} as f32, {} as f32)",
        canvas_expr_code(width, env, document)?,
        canvas_expr_code(height, env, document)?
    ))
}

pub(super) fn canvas_expr_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    expr_code(value, env, document, ValueMode::Owned)
}
