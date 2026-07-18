use super::*;

pub(super) fn render_children(
    out: &mut String,
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<(), Error> {
    for child in children {
        match child {
            ViewNode::If {
                condition,
                children,
                ..
            } => {
                let condition = expr_code(condition, env, document, ValueMode::Owned)?;
                write!(out, " if {condition} {{").unwrap();
                render_children(out, children, document, message, env, scope, slot)?;
                out.push_str(" }");
            }
            ViewNode::For {
                item,
                items,
                children,
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
                    return Err(Error::new("E121", span, "for expects a list"));
                };
                let items = expr_code(items, env, document, ValueMode::Borrowed)?;
                write!(out, " for {item} in {items}.iter() {{").unwrap();
                let mut child_env = env.clone();
                child_env.insert(
                    item.clone(),
                    Binding {
                        code: item.clone(),
                        ty: *inner,
                        local: false,
                    },
                );
                render_children(out, children, document, message, &child_env, scope, slot)?;
                out.push_str(" }");
            }
            _ => {
                let child = render_node(child, document, message, env, scope, slot)?;
                write!(out, " __children.push({child});").unwrap();
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
pub(super) struct Binding {
    pub(super) code: String,
    pub(super) ty: Type,
    pub(super) local: bool,
}

#[derive(Clone)]
pub(super) struct SlotContext {
    pub(super) entries: Vec<SlotContent>,
    pub(super) parent: Option<Box<SlotContext>>,
}

#[derive(Clone)]
pub(super) struct SlotContent {
    pub(super) name: String,
    pub(super) node: ViewNode,
    pub(super) env: HashMap<String, Binding>,
}

#[derive(Clone, Copy)]
pub(super) enum ValueMode {
    Owned,
    Borrowed,
}

pub(super) fn state_env(document: &Document, name: &str) -> HashMap<String, Binding> {
    document
        .states
        .iter()
        .map(|state| {
            (
                state.name.clone(),
                Binding {
                    code: format!("{name}.{}", state.name),
                    ty: state.ty.clone(),
                    local: false,
                },
            )
        })
        .collect()
}

pub(super) fn env_types(env: &HashMap<String, Binding>) -> HashMap<String, Type> {
    env.iter()
        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
        .collect()
}

pub(super) fn pixel_value_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let code = expr_code(value, env, document, ValueMode::Owned)?;
    Ok(
        if expr_type(value, &env_types(env), document, &Span::line(1))? == Type::Pixels {
            code
        } else {
            format!("({code}) as f32")
        },
    )
}

pub(super) fn radians_value_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let code = expr_code(value, env, document, ValueMode::Owned)?;
    Ok(
        if expr_type(value, &env_types(env), document, &Span::line(1))? == Type::Radians {
            code
        } else {
            format!("::iced::Radians(({code}) as f32)")
        },
    )
}

pub(super) fn native_field_type(ty: &Type, field: &str) -> Option<Type> {
    match ty {
        Type::KeyPress => match field {
            "key" | "modified_key" => Some(Type::Key),
            "physical_key" => Some(Type::PhysicalKey),
            "location" => Some(Type::KeyLocation),
            "modifiers" => Some(Type::KeyModifiers),
            "text" => Some(Type::Option(Box::new(Type::Str))),
            "repeat" => Some(Type::Bool),
            _ => None,
        },
        Type::KeyRelease => match field {
            "key" | "modified_key" => Some(Type::Key),
            "physical_key" => Some(Type::PhysicalKey),
            "location" => Some(Type::KeyLocation),
            "modifiers" => Some(Type::KeyModifiers),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn native_field_projection(
    ty: &Type,
    field: &str,
    code: &str,
) -> Option<(String, Type)> {
    let projected = match (ty, field) {
        (Type::Key, "kind") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Key::Named(_) => \"named\", ::iced::keyboard::Key::Character(_) => \"character\", ::iced::keyboard::Key::Unidentified => \"unidentified\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::Key, "named") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Key::Named(value) => ::std::option::Option::Some(::std::format!(\"{{value:?}}\")), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::Key, "character") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Key::Character(value) => ::std::option::Option::Some(value.to_string()), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::PhysicalKey, "kind") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Code(_) => \"code\", ::iced::keyboard::key::Physical::Unidentified(_) => \"native\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::PhysicalKey, "code") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Code(value) => ::std::option::Option::Some(::std::format!(\"{{value:?}}\")), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::PhysicalKey, "native_platform") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Unidentified) => ::std::option::Option::Some(\"unidentified\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Android(_)) => ::std::option::Option::Some(\"android\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::MacOS(_)) => ::std::option::Option::Some(\"macos\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Windows(_)) => ::std::option::Option::Some(\"windows\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Xkb(_)) => ::std::option::Option::Some(\"xkb\".to_owned()), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::PhysicalKey, "native_code") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Android(value) | ::iced::keyboard::key::NativeCode::Xkb(value)) => ::std::option::Option::Some(i64::from(*value)), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::MacOS(value) | ::iced::keyboard::key::NativeCode::Windows(value)) => ::std::option::Option::Some(i64::from(*value)), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::I64)),
        ),
        (Type::KeyLocation, "name") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Location::Standard => \"standard\", ::iced::keyboard::Location::Left => \"left\", ::iced::keyboard::Location::Right => \"right\", ::iced::keyboard::Location::Numpad => \"numpad\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::KeyModifiers, field) => {
            let method = match field {
                "shift" => "shift",
                "control" => "control",
                "alt" => "alt",
                "logo" => "logo",
                "command" => "command",
                "jump" => "jump",
                "macos_command" => "macos_command",
                _ => return None,
            };
            (format!("({code}).{method}()"), Type::Bool)
        }
        (Type::Pixels | Type::Degrees | Type::Radians, "value") => {
            (format!("({code}).0 as f64"), Type::F64)
        }
        (Type::Padding, "top" | "right" | "bottom" | "left") => {
            (format!("({code}).{field} as f64"), Type::F64)
        }
        (Type::Padding, "x" | "y") => (format!("({code}).{field}() as f64"), Type::F64),
        (Type::Radians, "display") => (format!("::std::format!(\"{{}}\", {code})"), Type::Str),
        (Type::Point, "x" | "y")
        | (Type::Vector, "x" | "y")
        | (Type::Size, "width" | "height")
        | (Type::Rectangle, "x" | "y" | "width" | "height") => {
            (format!("({code}).{field} as f64"), Type::F64)
        }
        (Type::PointU32, "x" | "y") | (Type::RectangleU32, "x" | "y" | "width" | "height") => {
            (format!("({code}).{field} as i64"), Type::I64)
        }
        (Type::Point | Type::Vector | Type::Size, "values") => (
            format!(
                "::std::convert::Into::<[f32; 2]>::into({code}).into_iter().map(f64::from).collect::<::std::vec::Vec<_>>()"
            ),
            Type::List(Box::new(Type::F64)),
        ),
        (Type::Point, "display") => (format!("::std::format!(\"{{}}\", {code})"), Type::Str),
        (Type::Rectangle, "center") => (format!("({code}).center()"), Type::Point),
        (Type::Rectangle, "center_x") => (format!("({code}).center_x() as f64"), Type::F64),
        (Type::Rectangle, "center_y") => (format!("({code}).center_y() as f64"), Type::F64),
        (Type::Rectangle, "position") => (format!("({code}).position()"), Type::Point),
        (Type::Rectangle, "size") => (format!("({code}).size()"), Type::Size),
        (Type::Rectangle, "area") => (format!("({code}).area() as f64"), Type::F64),
        (Type::Transformation, "scale_factor") => {
            (format!("({code}).scale_factor() as f64"), Type::F64)
        }
        (Type::Transformation, "translation") => (format!("({code}).translation()"), Type::Vector),
        (Type::Transformation, "matrix") => (
            format!(
                "::std::convert::Into::<[f32; 16]>::into({code}).into_iter().map(f64::from).collect::<::std::vec::Vec<_>>()"
            ),
            Type::List(Box::new(Type::F64)),
        ),
        (Type::MouseButton, "kind") => (
            format!(
                "match &({code}) {{ ::iced::mouse::Button::Left => \"left\", ::iced::mouse::Button::Right => \"right\", ::iced::mouse::Button::Middle => \"middle\", ::iced::mouse::Button::Back => \"back\", ::iced::mouse::Button::Forward => \"forward\", ::iced::mouse::Button::Other(_) => \"other\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::MouseButton, "number") => (
            format!(
                "match &({code}) {{ ::iced::mouse::Button::Other(value) => ::std::option::Option::Some(i64::from(*value)), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::I64)),
        ),
        (Type::MouseCursor, "kind") => (
            format!(
                "match &({code}) {{ ::iced::mouse::Cursor::Available(_) => \"available\", ::iced::mouse::Cursor::Levitating(_) => \"levitating\", ::iced::mouse::Cursor::Unavailable => \"unavailable\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::MouseCursor, "position") => (
            format!("({code}).position()"),
            Type::Option(Box::new(Type::Point)),
        ),
        (Type::MouseCursor, "levitating") => (format!("({code}).is_levitating()"), Type::Bool),
        (Type::MouseClick, "kind") => (
            format!(
                "match ({code}).kind() {{ ::iced::advanced::mouse::click::Kind::Single => \"single\", ::iced::advanced::mouse::click::Kind::Double => \"double\", ::iced::advanced::mouse::click::Kind::Triple => \"triple\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::MouseClick, "position") => (format!("({code}).position()"), Type::Point),
        (Type::TouchFinger, "id") => (format!("({code}).0.to_string()"), Type::Str),
        _ => return None,
    };
    Some(projected)
}

pub(super) fn expr_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
    mode: ValueMode,
) -> Result<String, Error> {
    Ok(match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::I64(value) => value.to_string(),
        Expr::F64(value) => rust_f64(*value),
        Expr::Str(value) => match mode {
            ValueMode::Owned => format!("{}.to_owned()", rust_string(value)),
            ValueMode::Borrowed => rust_string(value),
        },
        Expr::Bytes(values) => format!(
            "::std::vec![{}]",
            values
                .iter()
                .map(|value| format!("0x{value:02x}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::EmptyList => "::std::vec::Vec::new()".into(),
        Expr::List(values) => format!(
            "::std::vec![{}]",
            values
                .iter()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Expr::None => "::std::option::Option::None".into(),
        Expr::Path(path) => {
            let binding = env.get(&path[0]).ok_or_else(|| {
                Error::new(
                    "E150",
                    &Span::line(1),
                    format!("unknown value `{}`", path[0]),
                )
            })?;
            let mut code = binding.code.clone();
            let mut ty = binding.ty.clone();
            let mut owned_projection = false;
            for field in &path[1..] {
                if let Some((projection, projected_ty)) =
                    native_field_projection(&ty, field, &code)
                {
                    code = projection;
                    ty = projected_ty;
                    owned_projection = true;
                    continue;
                }
                if let Type::Option(inner) = &ty
                    && **inner == Type::WidgetTarget
                {
                    code = format!("({code}).as_ref().map(|value| value.{field}.clone())");
                    ty = Type::Option(Box::new(
                        widget_target_field_type(field).unwrap_or(Type::Unknown),
                    ));
                    owned_projection = true;
                    continue;
                }
                write!(code, ".{field}").unwrap();
                if let Type::Named(name) = &ty {
                    ty = document
                        .structs
                        .iter()
                        .find(|item| item.name == *name)
                        .and_then(|item| item.fields.iter().find(|(name, _)| name == field))
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(Type::Unknown);
                } else if ty == Type::WidgetTarget {
                    ty = widget_target_field_type(field).unwrap_or(Type::Unknown);
                } else if let Some(field_ty) = native_field_type(&ty, field) {
                    ty = field_ty;
                }
            }
            let clone_unnecessary = matches!(
                ty,
                Type::Bool
                    | Type::I64
                    | Type::F64
                    | Type::PhysicalKey
                    | Type::KeyLocation
                    | Type::KeyModifiers
                    | Type::Pixels
                    | Type::Padding
                    | Type::Degrees
                    | Type::Radians
                    | Type::Point
                    | Type::PointU32
                    | Type::Vector
                    | Type::Size
                    | Type::Rectangle
                    | Type::RectangleU32
                    | Type::Transformation
                    | Type::MouseButton
                    | Type::MouseCursor
                    | Type::MouseClick
                    | Type::TouchFinger
                    | Type::Unit
            )
                || (binding.local && path.len() == 1)
                || owned_projection;
            if matches!(mode, ValueMode::Owned) && !clone_unnecessary {
                code.push_str(".clone()");
            }
            code
        }
        Expr::Call { name, args } => match name.as_str() {
            "pixels" => format!(
                "::iced::Pixels(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "pixels.zero" => "::iced::Pixels::ZERO".into(),
            "pixels.from_u32" => {
                let Expr::I64(value) = &args[0] else {
                    unreachable!("checker requires a pixels u32 literal")
                };
                format!("::iced::Pixels::from({value}u32)")
            }
            "pixels.try_from_u32" => format!(
                "<u32>::try_from({}).ok().map(::iced::Pixels::from)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "padding" => format!(
                "::iced::Padding {{ top: ({}) as f32, right: ({}) as f32, bottom: ({}) as f32, left: ({}) as f32 }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?,
                expr_code(&args[3], env, document, ValueMode::Owned)?
            ),
            "padding.zero" => "::iced::Padding::ZERO".into(),
            "padding.all"
            | "padding.top"
            | "padding.right"
            | "padding.bottom"
            | "padding.left"
            | "padding.horizontal"
            | "padding.vertical" => {
                let function = name.strip_prefix("padding.").expect("checked prefix");
                format!(
                    "::iced::padding::{function}({})",
                    pixel_value_code(&args[0], env, document)?
                )
            }
            "padding.axes" => format!(
                "::iced::Padding::from([({}) as f32, ({}) as f32])",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "padding.from_pixels" => format!(
                "::iced::Padding::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "padding.with_top"
            | "padding.with_right"
            | "padding.with_bottom"
            | "padding.with_left"
            | "padding.with_horizontal"
            | "padding.with_vertical" => {
                let method = name.strip_prefix("padding.with_").expect("checked prefix");
                format!(
                    "({}).{method}({})",
                    expr_code(&args[0], env, document, ValueMode::Owned)?,
                    pixel_value_code(&args[1], env, document)?
                )
            }
            "padding.fit" => format!(
                "({}).fit({}, {})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "degrees" => format!(
                "::iced::Degrees(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians" => format!(
                "::iced::Radians(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "degrees.range_start" => "*::iced::Degrees::RANGE.start()".into(),
            "degrees.range_end" => "*::iced::Degrees::RANGE.end()".into(),
            "radians.range_start" => "*::iced::Radians::RANGE.start()".into(),
            "radians.range_end" => "*::iced::Radians::RANGE.end()".into(),
            "radians.pi" => "::iced::Radians::PI".into(),
            "degrees.in_range" => format!(
                "::iced::Degrees::RANGE.contains(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians.in_range" => format!(
                "::iced::Radians::RANGE.contains(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians.from_degrees" => format!(
                "::iced::Radians::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians.distance_start" | "radians.distance_end" => format!(
                "({}).to_distance(&({})).{}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                if name == "radians.distance_start" { 0 } else { 1 }
            ),
            "point" => format!(
                "::iced::Point::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "vector" => format!(
                "::iced::Vector::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size" => format!(
                "::iced::Size::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle" => format!(
                "::iced::Rectangle {{ x: ({}) as f32, y: ({}) as f32, width: ({}) as f32, height: ({}) as f32 }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?,
                expr_code(&args[3], env, document, ValueMode::Owned)?
            ),
            "point.origin" => "::iced::Point::ORIGIN".into(),
            "point.distance" => format!(
                "({}).distance({}) as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "point.snap" => format!(
                "({}).snap()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "vector.zero" => "::iced::Vector::ZERO".into(),
            "size.zero" => "::iced::Size::ZERO".into(),
            "size.unit" => "::iced::Size::UNIT".into(),
            "size.infinite" => "::iced::Size::INFINITE".into(),
            "size.min" => format!(
                "({}).min({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.max" => format!(
                "({}).max({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.expand" => format!(
                "({}).expand({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.rotate" => format!(
                "({}).rotate({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                radians_value_code(&args[1], env, document)?
            ),
            "size.ratio" => format!(
                "({}).ratio(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.from_vector" => format!(
                "::iced::Size::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "vector.from_size" => format!(
                "::iced::Vector::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "size.from_padding" => format!(
                "::iced::Size::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "size.from_u32" => {
                let (Expr::I64(width), Expr::I64(height)) = (&args[0], &args[1]) else {
                    unreachable!("checker requires size dimensions as integer literals")
                };
                format!("::iced::Size::from(({width}u32, {height}u32))")
            }
            "size.try_from_u32" => format!(
                "match (<u32>::try_from({}), <u32>::try_from({})) {{ (::std::result::Result::Ok(width), ::std::result::Result::Ok(height)) => ::std::option::Option::Some(::iced::Size::from((width, height))), _ => ::std::option::Option::None }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.zero" => "::iced::Rectangle::default()".into(),
            "rectangle.infinite" => "::iced::Rectangle::INFINITE".into(),
            "rectangle.with_size" => format!(
                "::iced::Rectangle::with_size({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rectangle.with_radius" => format!(
                "::iced::Rectangle::with_radius(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rectangle.with_vertices" => format!(
                "::iced::Rectangle::with_vertices({}, {}, {}).0",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "rectangle.vertices_rotation" => format!(
                "::iced::Rectangle::with_vertices({}, {}, {}).1.0 as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "rectangle.vertices_angle" => format!(
                "::iced::Rectangle::with_vertices({}, {}, {}).1",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "rectangle.contains" => format!(
                "({}).contains({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.distance" => format!(
                "({}).distance({}) as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.offset" => format!(
                "({}).offset(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.is_within" => format!(
                "({}).is_within(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.intersection" => format!(
                "({}).intersection(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.intersects" => format!(
                "({}).intersects(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.union" => format!(
                "({}).union(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.snap" => format!(
                "({}).snap()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rectangle.expand" | "rectangle.shrink" => format!(
                "({}).{}(::iced::Padding {{ top: ({}) as f32, right: ({}) as f32, bottom: ({}) as f32, left: ({}) as f32 }})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                if name == "rectangle.expand" {
                    "expand"
                } else {
                    "shrink"
                },
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?,
                expr_code(&args[3], env, document, ValueMode::Owned)?,
                expr_code(&args[4], env, document, ValueMode::Owned)?
            ),
            "rectangle.expand_padding" | "rectangle.shrink_padding" => format!(
                "({}).{}({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                if name == "rectangle.expand_padding" {
                    "expand"
                } else {
                    "shrink"
                },
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.rotate" => format!(
                "({}).rotate({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                radians_value_code(&args[1], env, document)?
            ),
            "rectangle.zoom" => format!(
                "({}).zoom(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.anchor" => {
                let (Expr::Str(horizontal), Expr::Str(vertical)) = (&args[2], &args[3]) else {
                    unreachable!("checker requires literal rectangle alignments")
                };
                let horizontal = match horizontal.as_str() {
                    "left" => "Left",
                    "center" => "Center",
                    "right" => "Right",
                    _ => unreachable!("checker validates horizontal alignment"),
                };
                let vertical = match vertical.as_str() {
                    "top" => "Top",
                    "center" => "Center",
                    "bottom" => "Bottom",
                    _ => unreachable!("checker validates vertical alignment"),
                };
                format!(
                    "({}).anchor({}, ::iced::alignment::Horizontal::{horizontal}, ::iced::alignment::Vertical::{vertical})",
                    expr_code(&args[0], env, document, ValueMode::Owned)?,
                    expr_code(&args[1], env, document, ValueMode::Owned)?
                )
            }
            "rectangle.from_u32" => format!(
                "::iced::Rectangle::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "transform.identity" => "::iced::Transformation::IDENTITY".into(),
            "transform.orthographic" => {
                let (Expr::I64(width), Expr::I64(height)) = (&args[0], &args[1]) else {
                    unreachable!("checker requires orthographic dimension literals")
                };
                format!("::iced::Transformation::orthographic({width}u32, {height}u32)")
            }
            "transform.try_orthographic" => format!(
                "match (<u32>::try_from({}), <u32>::try_from({})) {{ (::std::result::Result::Ok(width), ::std::result::Result::Ok(height)) => ::std::option::Option::Some(::iced::Transformation::orthographic(width, height)), _ => ::std::option::Option::None }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "transform.translate" => format!(
                "::iced::Transformation::translate(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "transform.scale" => format!(
                "::iced::Transformation::scale(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "transform.inverse" => format!(
                "({}).inverse()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "transform.compose" => format!(
                "({}) * ({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "transform.point"
            | "transform.vector"
            | "transform.size"
            | "transform.rectangle"
            | "transform.cursor"
            | "transform.click" => format!(
                "({}) * ({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.button" => {
                let Expr::Str(value) = &args[0] else {
                    unreachable!("checker requires a mouse button literal")
                };
                let variant = match value.as_str() {
                    "left" => "Left",
                    "right" => "Right",
                    "middle" => "Middle",
                    "back" => "Back",
                    "forward" => "Forward",
                    _ => unreachable!("checker validates mouse buttons"),
                };
                format!("::iced::mouse::Button::{variant}")
            }
            "mouse.other_button" => {
                let Expr::I64(value) = &args[0] else {
                    unreachable!("checker requires a mouse button literal")
                };
                format!("::iced::mouse::Button::Other({value}u16)")
            }
            "mouse.try_other_button" => format!(
                "<u16>::try_from({}).ok().map(::iced::mouse::Button::Other)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor" => format!(
                "::iced::mouse::Cursor::Available({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.levitating" => format!(
                "::iced::mouse::Cursor::Levitating({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.unavailable" => "::iced::mouse::Cursor::Unavailable".into(),
            "mouse.cursor_position" => format!(
                "({}).position()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_over" => format!(
                "({}).position_over({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_in" => format!(
                "({}).position_in({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_from" => format!(
                "({}).position_from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_is_over" => format!(
                "({}).is_over({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_is_levitating" => format!(
                "({}).is_levitating()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_levitate" => format!(
                "({}).levitate()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_land" => format!(
                "({}).land()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_translate" => format!(
                "({}) + ::iced::Vector::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "mouse.click" => format!(
                "::iced::advanced::mouse::Click::new({}, {}, {})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "touch.finger" => {
                let Expr::Str(value) = &args[0] else {
                    unreachable!("checker requires a touch finger literal")
                };
                let value = value
                    .parse::<u64>()
                    .expect("checker validates touch finger literals");
                format!("::iced::touch::Finger({value}u64)")
            }
            "touch.try_finger" => format!(
                "({}).parse::<u64>().ok().map(::iced::touch::Finger)",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "key.named" => {
                let Expr::Str(variant) = &args[0] else {
                    unreachable!("checker requires a named key variant")
                };
                format!(
                    "::iced::keyboard::Key::Named(::iced::keyboard::key::Named::{variant})"
                )
            }
            "key.character" => format!(
                "::iced::keyboard::Key::Character(({}).into())",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "key.unidentified" => "::iced::keyboard::Key::Unidentified".into(),
            "key.code" => {
                let Expr::Str(variant) = &args[0] else {
                    unreachable!("checker requires a physical key variant")
                };
                format!(
                    "::iced::keyboard::key::Physical::Code(::iced::keyboard::key::Code::{variant})"
                )
            }
            "key.native_unidentified" => "::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Unidentified)".into(),
            "key.command_modifiers" => "::iced::keyboard::Modifiers::COMMAND".into(),
            "key.native" => {
                let (Expr::Str(platform), Expr::I64(value)) = (&args[0], &args[1]) else {
                    unreachable!("checker requires a literal native key")
                };
                let (variant, ty) = match platform.as_str() {
                    "android" => ("Android", "u32"),
                    "macos" => ("MacOS", "u16"),
                    "windows" => ("Windows", "u16"),
                    "xkb" => ("Xkb", "u32"),
                    _ => unreachable!("checker validates native key platforms"),
                };
                format!(
                    "::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::{variant}({value}{ty}))"
                )
            }
            "key.try_native" => {
                let Expr::Str(platform) = &args[0] else {
                    unreachable!("checker requires a native key platform")
                };
                let (variant, ty) = match platform.as_str() {
                    "android" => ("Android", "u32"),
                    "macos" => ("MacOS", "u16"),
                    "windows" => ("Windows", "u16"),
                    "xkb" => ("Xkb", "u32"),
                    _ => unreachable!("checker validates native key platforms"),
                };
                format!(
                    "<{ty}>::try_from({}).ok().map(|value| ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::{variant}(value)))",
                    expr_code(&args[1], env, document, ValueMode::Owned)?
                )
            }
            "key.location" => {
                let Expr::Str(value) = &args[0] else {
                    unreachable!("checker requires a key location literal")
                };
                let variant = match value.as_str() {
                    "standard" => "Standard",
                    "left" => "Left",
                    "right" => "Right",
                    "numpad" => "Numpad",
                    _ => unreachable!("checker validates key locations"),
                };
                format!("::iced::keyboard::Location::{variant}")
            }
            "key.modifiers" => {
                let values = ["SHIFT", "CTRL", "ALT", "LOGO"]
                    .into_iter()
                    .zip(args)
                    .map(|(flag, value)| {
                        Ok(format!(
                            "if {} {{ ::iced::keyboard::Modifiers::{flag} }} else {{ ::iced::keyboard::Modifiers::empty() }}",
                            expr_code(value, env, document, ValueMode::Owned)?
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                format!("({})", values.join(" | "))
            }
            "key.latin" => format!(
                "({}).to_latin({}).map(|value| value.to_string())",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "len" => format!(
                "({}).len() as i64",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "empty" => format!(
                "({}).is_empty()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "trim" => format!(
                "({}).trim().to_owned()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "some" => format!(
                "::std::option::Option::Some({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "markdown" => format!(
                "::iced::widget::markdown::Content::parse(&{})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "markdown_images" => format!(
                "({}).images().iter().cloned().collect::<::std::vec::Vec<_>>()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "editor" => format!(
                "::iced::widget::text_editor::Content::with_text(&{})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "encoded" => format!(
                "::iced::widget::image::Handle::from_bytes({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rgba" => format!(
                "::iced::widget::image::Handle::from_rgba({}, {}, {})",
                u32_code(&args[0], env, document)?,
                u32_code(&args[1], env, document)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "aborted" => format!(
                "({}).as_ref().is_some_and(::iced::task::Handle::is_aborted)",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            _ => {
                let function = document
                    .functions
                    .iter()
                    .find(|function| function.name == *name && function.kind == ExternKind::Sync)
                    .expect("checker accepts only declared sync calls");
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("{}({args})", function.rust_path)
            }
        },
        Expr::Unary { op, value } => format!(
            "({}{})",
            match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            },
            expr_code(value, env, document, ValueMode::Owned)?
        ),
        Expr::Binary { left, op, right } => {
            let mode = if matches!(
                op,
                BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
            ) {
                ValueMode::Borrowed
            } else {
                ValueMode::Owned
            };
            let types = env_types(env);
            let left_ty = expr_type(left, &types, document, &Span::line(1))?;
            let right_ty = expr_type(right, &types, document, &Span::line(1))?;
            let left = expr_code(left, env, document, mode)?;
            let right = expr_code(right, env, document, mode)?;
            let left = if left_ty == Type::F64
                && right_ty == Type::Radians
                && *op == BinaryOp::Mul
            {
                format!("({left}) as f32")
            } else {
                left
            };
            let right = if right_ty == Type::F64
                && matches!(
                    left_ty,
                    Type::Pixels
                        | Type::Degrees
                        | Type::Radians
                        | Type::Vector
                        | Type::Size
                        | Type::Rectangle
                )
            {
                format!("({right}) as f32")
            } else {
                right
            };
            format!(
                "({} {} {})",
                left,
                match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Rem => "%",
                    BinaryOp::Eq => "==",
                    BinaryOp::NotEq => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                },
                right
            )
        }
    })
}

pub(super) fn widget_target_field_type(field: &str) -> Option<Type> {
    match field {
        "kind" => Some(Type::Str),
        "id" => Some(Type::Option(Box::new(Type::WidgetId))),
        "x" | "y" | "width" | "height" => Some(Type::F64),
        "visible_x" | "visible_y" | "visible_width" | "visible_height" | "content_x"
        | "content_y" | "content_width" | "content_height" | "translation_x" | "translation_y" => {
            Some(Type::Option(Box::new(Type::F64)))
        }
        "content" => Some(Type::Option(Box::new(Type::Str))),
        _ => None,
    }
}

pub(super) fn u32_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "({}).clamp(0, u32::MAX as i64) as u32",
        expr_code(expr, env, document, ValueMode::Owned)?
    ))
}

pub(super) fn route_code(
    route: &Route,
    payload: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    if route.args.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok(payload.into()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}

pub(super) fn size_route_code(
    route: &Route,
    size: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    ordered_route_code(
        route,
        &[
            &format!("{size}.width as f64"),
            &format!("{size}.height as f64"),
        ],
        env,
        document,
        message,
    )
}

pub(super) fn ordered_route_code(
    route: &Route,
    payloads: &[&str],
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    if route.args.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let mut payload = payloads.iter();
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok((*payload.next().expect("checked payload count")).to_owned()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}

pub(super) fn initial_code(expr: &Expr, ty: &Type, document: &Document) -> String {
    match (expr, ty) {
        (Expr::Str(value), Type::Str) => format!("{}.to_owned()", rust_string(value)),
        (Expr::Str(value), Type::Markdown) => format!(
            "::iced::widget::markdown::Content::parse({})",
            rust_string(value)
        ),
        (Expr::Str(value), Type::Editor) => format!(
            "::iced::widget::text_editor::Content::with_text({})",
            rust_string(value)
        ),
        (Expr::EmptyList, Type::List(_)) => "::std::vec::Vec::new()".into(),
        (Expr::EmptyList, Type::Combo(_)) => {
            "::iced::widget::combo_box::State::new(::std::vec::Vec::new())".into()
        }
        (Expr::List(values), Type::Combo(_)) => format!(
            "::iced::widget::combo_box::State::new(::std::vec![{}])",
            values
                .iter()
                .map(|value| {
                    expr_code(value, &HashMap::new(), document, ValueMode::Owned)
                        .unwrap_or_else(|_| "::core::default::Default::default()".into())
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        (Expr::None, Type::Option(_)) => "::std::option::Option::None".into(),
        (Expr::Bool(value), _) => value.to_string(),
        (Expr::I64(value), _) => value.to_string(),
        (Expr::F64(value), _) => rust_f64(*value),
        _ => expr_code(expr, &HashMap::new(), document, ValueMode::Owned)
            .unwrap_or_else(|_| "::core::default::Default::default()".into()),
    }
}

pub(super) fn pane_field(name: &str) -> String {
    format!("__pane_{name}")
}

pub(super) fn pane_configuration_code(configuration: &PaneConfiguration) -> String {
    match configuration {
        PaneConfiguration::Pane(name) => format!(
            "::iced::widget::pane_grid::Configuration::Pane({})",
            rust_string(name)
        ),
        PaneConfiguration::Split { axis, ratio, a, b } => {
            let axis = match axis {
                PaneAxis::Horizontal => "Horizontal",
                PaneAxis::Vertical => "Vertical",
            };
            format!(
                "::iced::widget::pane_grid::Configuration::Split {{ axis: ::iced::widget::pane_grid::Axis::{axis}, ratio: {ratio:?}, a: ::std::boxed::Box::new({}), b: ::std::boxed::Box::new({}) }}",
                pane_configuration_code(a),
                pane_configuration_code(b)
            )
        }
    }
}

pub(super) fn pane_resize_variant(name: &str) -> String {
    format!("__Pane{}Resize", pascal(name))
}

pub(super) fn pane_drag_variant(name: &str) -> String {
    format!("__Pane{}Drag", pascal(name))
}

pub(super) fn pane_grids(root: &ViewNode) -> Vec<&ViewNode> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<&'a ViewNode>) {
        match node {
            ViewNode::PaneGrid { panes, .. } => {
                output.push(node);
                for pane in panes {
                    for node in pane.nodes() {
                        collect(node, output);
                    }
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(root, &mut output);
    output
}

pub(super) fn uses_canvas(document: &Document) -> bool {
    !canvases(document).is_empty()
}

pub(super) fn canvases(document: &Document) -> Vec<(&CanvasOptions, &[CanvasEvent])> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<(&'a CanvasOptions, &'a [CanvasEvent])>) {
        match node {
            ViewNode::Canvas {
                options, events, ..
            } => output.push((options, events)),
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::PaneGrid { panes, .. } => {
                for node in panes.iter().flat_map(PaneView::nodes) {
                    collect(node, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(&document.view, &mut output);
    for component in &document.components {
        collect(&component.root, &mut output);
    }
    output
}

pub(super) fn canvas_cache_groups(document: &Document) -> Vec<&str> {
    let mut groups = Vec::new();
    for group in canvases(document)
        .into_iter()
        .filter_map(|(options, _)| options.cache_group.as_deref())
    {
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    groups
}

pub(super) fn canvas_events(document: &Document) -> Vec<&CanvasEvent> {
    canvases(document)
        .into_iter()
        .flat_map(|(_, events)| events)
        .collect()
}

pub(super) fn canvas_group_symbol(group: &str) -> String {
    format!("__ICE_CANVAS_GROUP_{}", group.to_ascii_uppercase())
}

pub(super) fn needs_extern_noop(document: &Document) -> bool {
    fn contains(node: &ViewNode) -> bool {
        match node {
            ViewNode::ExternComponent { route: None, .. }
            | ViewNode::Themer { route: None, .. }
            | ViewNode::Shader { route: None, .. } => true,
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => children.iter().any(contains),
            ViewNode::Tooltip { content, tip, .. } => contains(content) || contains(tip),
            ViewNode::Overlay { .. } => true,
            ViewNode::PaneGrid { panes, .. } => {
                panes.iter().flat_map(PaneView::nodes).any(contains)
            }
            ViewNode::Table { columns, .. } => columns
                .iter()
                .any(|column| contains(&column.header) || contains(&column.cell)),
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. } => contains(content),
            ViewNode::Component { slots, .. } => slots.iter().any(|slot| contains(&slot.content)),
            ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => contains(child),
            ViewNode::Button {
                content: Some(content),
                ..
            } => contains(content),
            ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => contains(content),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    contains(narrow) || contains(wide)
                }
                ResponsiveContent::Size { content, .. } => contains(content),
            },
            _ => false,
        }
    }
    contains(&document.view) || document.components.iter().any(|item| contains(&item.root))
}
