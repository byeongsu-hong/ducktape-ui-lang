use super::*;

#[derive(Clone)]
pub(in crate::codegen) struct Binding {
    pub(in crate::codegen) code: String,
    pub(in crate::codegen) ty: Type,
    pub(in crate::codegen) local: bool,
}

#[derive(Clone)]
pub(in crate::codegen) struct SlotContext {
    pub(in crate::codegen) entries: Vec<SlotContent>,
    pub(in crate::codegen) parent: Option<Box<SlotContext>>,
}

#[derive(Clone)]
pub(in crate::codegen) struct SlotContent {
    pub(in crate::codegen) name: String,
    pub(in crate::codegen) node: ViewNode,
    pub(in crate::codegen) env: HashMap<String, Binding>,
}

#[derive(Clone, Copy)]
pub(in crate::codegen) enum ValueMode {
    Owned,
    Borrowed,
}

pub(in crate::codegen) fn state_env(document: &Document, name: &str) -> HashMap<String, Binding> {
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

pub(in crate::codegen) fn env_types(env: &HashMap<String, Binding>) -> HashMap<String, Type> {
    env.iter()
        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
        .collect()
}

pub(in crate::codegen) fn pixel_value_code(
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

pub(in crate::codegen) fn radians_value_code(
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

pub(in crate::codegen) fn native_field_type(ty: &Type, field: &str) -> Option<Type> {
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

pub(in crate::codegen) fn native_field_projection(
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
