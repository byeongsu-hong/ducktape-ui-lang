use super::*;

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

mod binding;
mod children;
mod discovery;
mod routes;

pub(super) use binding::*;
pub(super) use children::*;
pub(super) use discovery::*;
pub(super) use routes::*;
