use super::*;

pub(crate) fn expr_type(
    expr: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    match expr {
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::I64(_) => Ok(Type::I64),
        Expr::F64(_) => Ok(Type::F64),
        Expr::Str(_) => Ok(Type::Str),
        Expr::Bytes(_) => Ok(Type::Bytes),
        Expr::EmptyList => Ok(Type::List(Box::new(Type::Unknown))),
        Expr::List(values) => {
            let Some(first) = values.first() else {
                return Ok(Type::List(Box::new(Type::Unknown)));
            };
            let ty = expr_type(first, env, document, span)?;
            for value in &values[1..] {
                let actual = expr_type(value, env, document, span)?;
                require_type(&actual, &ty, span)?;
            }
            Ok(Type::List(Box::new(ty)))
        }
        Expr::None => Ok(Type::Option(Box::new(Type::Unknown))),
        Expr::Path(path) => {
            let mut ty = env
                .get(&path[0])
                .cloned()
                .ok_or_else(|| Error::new("E150", span, format!("unknown value `{}`", path[0])))?;
            for field in &path[1..] {
                ty = field_type(&ty, field, document, span)?;
            }
            Ok(ty)
        }
        Expr::Call { name, args } => match name.as_str() {
            "pixels" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Pixels)
            }
            "pixels.zero" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Pixels)
            }
            "pixels.from_u32" => {
                check_u32_literal(name, args, span)?;
                Ok(Type::Pixels)
            }
            "pixels.try_from_u32" => {
                check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Pixels)))
            }
            "padding" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::F64, Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Padding)
            }
            "padding.zero" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.all" | "padding.top" | "padding.right" | "padding.bottom" | "padding.left"
            | "padding.horizontal" | "padding.vertical" => {
                if args.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("{name} expects one argument"),
                    ));
                }
                require_pixel_value(&args[0], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.axes" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.from_pixels" => {
                check_builtin_args(name, args, &[Type::Pixels], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.with_top"
            | "padding.with_right"
            | "padding.with_bottom"
            | "padding.with_left"
            | "padding.with_horizontal"
            | "padding.with_vertical" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("{name} expects padding and a pixel value"),
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Padding,
                    span,
                )?;
                require_pixel_value(&args[1], env, document, span)?;
                Ok(Type::Padding)
            }
            "padding.fit" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Padding, Type::Size, Type::Size],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Padding)
            }
            "degrees" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Degrees)
            }
            "radians" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Radians)
            }
            "degrees.range_start" | "degrees.range_end" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Degrees)
            }
            "radians.range_start" | "radians.range_end" | "radians.pi" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Radians)
            }
            "degrees.in_range" => {
                check_builtin_args(name, args, &[Type::Degrees], env, document, span)?;
                Ok(Type::Bool)
            }
            "radians.in_range" => {
                check_builtin_args(name, args, &[Type::Radians], env, document, span)?;
                Ok(Type::Bool)
            }
            "radians.from_degrees" => {
                check_builtin_args(name, args, &[Type::Degrees], env, document, span)?;
                Ok(Type::Radians)
            }
            "radians.distance_start" | "radians.distance_end" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Radians, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Point)
            }
            "point" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Point)
            }
            "vector" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Vector)
            }
            "size" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Size)
            }
            "rectangle" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::F64, Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "point.origin" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Point)
            }
            "point.distance" => {
                check_builtin_args(name, args, &[Type::Point, Type::Point], env, document, span)?;
                Ok(Type::F64)
            }
            "point.snap" => {
                check_builtin_args(name, args, &[Type::Point], env, document, span)?;
                Ok(Type::PointU32)
            }
            "vector.zero" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Vector)
            }
            "size.zero" | "size.unit" | "size.infinite" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Size)
            }
            "size.min" | "size.max" | "size.expand" => {
                check_builtin_args(name, args, &[Type::Size, Type::Size], env, document, span)?;
                Ok(Type::Size)
            }
            "size.rotate" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "size.rotate expects a size and radians",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Size,
                    span,
                )?;
                require_radians_value(&args[1], env, document, span)?;
                Ok(Type::Size)
            }
            "size.ratio" => {
                check_builtin_args(name, args, &[Type::Size, Type::F64], env, document, span)?;
                Ok(Type::Size)
            }
            "size.from_vector" => {
                check_builtin_args(name, args, &[Type::Vector], env, document, span)?;
                Ok(Type::Size)
            }
            "vector.from_size" => {
                check_builtin_args(name, args, &[Type::Size], env, document, span)?;
                Ok(Type::Vector)
            }
            "size.from_padding" => {
                check_builtin_args(name, args, &[Type::Padding], env, document, span)?;
                Ok(Type::Size)
            }
            "size.from_u32" => {
                check_u32_literals(name, args, span)?;
                Ok(Type::Size)
            }
            "size.try_from_u32" => {
                check_builtin_args(name, args, &[Type::I64, Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Size)))
            }
            "rectangle.zero" | "rectangle.infinite" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.with_size" => {
                check_builtin_args(name, args, &[Type::Size], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.with_radius" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.with_vertices"
            | "rectangle.vertices_rotation"
            | "rectangle.vertices_angle" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Point, Type::Point, Type::Point],
                    env,
                    document,
                    span,
                )?;
                Ok(match name.as_str() {
                    "rectangle.with_vertices" => Type::Rectangle,
                    "rectangle.vertices_rotation" => Type::F64,
                    _ => Type::Radians,
                })
            }
            "rectangle.contains" | "rectangle.distance" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Point],
                    env,
                    document,
                    span,
                )?;
                Ok(if name == "rectangle.contains" {
                    Type::Bool
                } else {
                    Type::F64
                })
            }
            "rectangle.offset" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Vector)
            }
            "rectangle.is_within" | "rectangle.intersects" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Bool)
            }
            "rectangle.intersection" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Rectangle)))
            }
            "rectangle.union" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.snap" => {
                check_builtin_args(name, args, &[Type::Rectangle], env, document, span)?;
                Ok(Type::Option(Box::new(Type::RectangleU32)))
            }
            "rectangle.expand" | "rectangle.shrink" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::F64, Type::F64, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.expand_padding" | "rectangle.shrink_padding" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::Padding],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.rotate" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.rotate expects a rectangle and radians",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Rectangle,
                    span,
                )?;
                require_radians_value(&args[1], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "rectangle.zoom" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Rectangle, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Rectangle)
            }
            "rectangle.anchor" => {
                if args.len() != 4 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor expects a rectangle, size, horizontal alignment, and vertical alignment",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Rectangle,
                    span,
                )?;
                require_type(
                    &expr_type(&args[1], env, document, span)?,
                    &Type::Size,
                    span,
                )?;
                let Expr::Str(horizontal) = &args[2] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor horizontal alignment must be left, center, or right",
                    ));
                };
                let Expr::Str(vertical) = &args[3] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor vertical alignment must be top, center, or bottom",
                    ));
                };
                if !matches!(horizontal.as_str(), "left" | "center" | "right") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor horizontal alignment must be left, center, or right",
                    ));
                }
                if !matches!(vertical.as_str(), "top" | "center" | "bottom") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rectangle.anchor vertical alignment must be top, center, or bottom",
                    ));
                }
                Ok(Type::Point)
            }
            "rectangle.from_u32" => {
                check_builtin_args(name, args, &[Type::RectangleU32], env, document, span)?;
                Ok(Type::Rectangle)
            }
            "transform.identity" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.orthographic" => {
                check_u32_literals(name, args, span)?;
                Ok(Type::Transformation)
            }
            "transform.try_orthographic" => {
                check_builtin_args(name, args, &[Type::I64, Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Transformation)))
            }
            "transform.translate" => {
                check_builtin_args(name, args, &[Type::F64, Type::F64], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.scale" => {
                check_builtin_args(name, args, &[Type::F64], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.inverse" => {
                check_builtin_args(name, args, &[Type::Transformation], env, document, span)?;
                Ok(Type::Transformation)
            }
            "transform.compose" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::Transformation, Type::Transformation],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Transformation)
            }
            "transform.point"
            | "transform.vector"
            | "transform.size"
            | "transform.rectangle"
            | "transform.cursor"
            | "transform.click" => {
                let value = match name.as_str() {
                    "transform.point" => Type::Point,
                    "transform.vector" => Type::Vector,
                    "transform.size" => Type::Size,
                    "transform.rectangle" => Type::Rectangle,
                    "transform.cursor" => Type::MouseCursor,
                    "transform.click" => Type::MouseClick,
                    _ => unreachable!(),
                };
                check_builtin_args(
                    name,
                    args,
                    &[value.clone(), Type::Transformation],
                    env,
                    document,
                    span,
                )?;
                Ok(value)
            }
            "mouse.button" => {
                let [Expr::Str(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "mouse.button expects one string literal",
                    ));
                };
                if !matches!(
                    value.as_str(),
                    "left" | "right" | "middle" | "back" | "forward"
                ) {
                    return Err(Error::new(
                        "E152",
                        span,
                        "mouse.button must be left, right, middle, back, or forward",
                    ));
                }
                Ok(Type::MouseButton)
            }
            "mouse.other_button" => {
                let [Expr::I64(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "mouse.other_button expects one integer literal",
                    ));
                };
                if !(0..=u16::MAX as i64).contains(value) {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("mouse.other_button must be in 0..={}", u16::MAX),
                    ));
                }
                Ok(Type::MouseButton)
            }
            "mouse.try_other_button" => {
                check_builtin_args(name, args, &[Type::I64], env, document, span)?;
                Ok(Type::Option(Box::new(Type::MouseButton)))
            }
            "mouse.cursor" | "mouse.levitating" => {
                check_builtin_args(name, args, &[Type::Point], env, document, span)?;
                Ok(Type::MouseCursor)
            }
            "mouse.unavailable" => {
                check_builtin_args(name, args, &[], env, document, span)?;
                Ok(Type::MouseCursor)
            }
            "mouse.cursor_position" => {
                check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                Ok(Type::Option(Box::new(Type::Point)))
            }
            "mouse.cursor_over" | "mouse.cursor_in" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Point)))
            }
            "mouse.cursor_from" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::Point],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Point)))
            }
            "mouse.cursor_is_over" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::Rectangle],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::Bool)
            }
            "mouse.cursor_is_levitating" => {
                check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                Ok(Type::Bool)
            }
            "mouse.cursor_levitate" | "mouse.cursor_land" => {
                check_builtin_args(name, args, &[Type::MouseCursor], env, document, span)?;
                Ok(Type::MouseCursor)
            }
            "mouse.cursor_translate" => {
                check_builtin_args(
                    name,
                    args,
                    &[Type::MouseCursor, Type::F64, Type::F64],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::MouseCursor)
            }
            "mouse.click" => {
                check_builtin_args(
                    name,
                    args,
                    &[
                        Type::Point,
                        Type::MouseButton,
                        Type::Option(Box::new(Type::MouseClick)),
                    ],
                    env,
                    document,
                    span,
                )?;
                Ok(Type::MouseClick)
            }
            "touch.finger" => {
                let [Expr::Str(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "touch.finger expects one decimal string literal",
                    ));
                };
                if value.is_empty()
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                    || value.parse::<u64>().is_err()
                {
                    return Err(Error::new(
                        "E152",
                        span,
                        "touch.finger must contain a decimal u64",
                    ));
                }
                Ok(Type::TouchFinger)
            }
            "touch.try_finger" => {
                check_builtin_args(name, args, &[Type::Str], env, document, span)?;
                Ok(Type::Option(Box::new(Type::TouchFinger)))
            }
            "key.named" => {
                keyboard_variant(name, args, span)?;
                Ok(Type::Key)
            }
            "key.character" => {
                if args.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.character expects one str argument",
                    ));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Key)
            }
            "key.unidentified" | "key.native_unidentified" | "key.command_modifiers" => {
                if !args.is_empty() {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("{name} expects no arguments"),
                    ));
                }
                Ok(match name.as_str() {
                    "key.unidentified" => Type::Key,
                    "key.native_unidentified" => Type::PhysicalKey,
                    _ => Type::KeyModifiers,
                })
            }
            "key.code" => {
                keyboard_variant(name, args, span)?;
                Ok(Type::PhysicalKey)
            }
            "key.try_native" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.try_native expects a platform and integer code",
                    ));
                }
                let Expr::Str(platform) = &args[0] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.try_native platform must be a string literal",
                    ));
                };
                if !matches!(platform.as_str(), "android" | "macos" | "windows" | "xkb") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.try_native platform must be android, macos, windows, or xkb",
                    ));
                }
                require_type(&expr_type(&args[1], env, document, span)?, &Type::I64, span)?;
                Ok(Type::Option(Box::new(Type::PhysicalKey)))
            }
            "key.native" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.native expects a platform and integer code",
                    ));
                }
                let Expr::Str(platform) = &args[0] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.native platform must be a string literal",
                    ));
                };
                let Expr::I64(code) = args[1] else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.native code must be an integer literal",
                    ));
                };
                let maximum = match platform.as_str() {
                    "android" | "xkb" => u32::MAX as i64,
                    "macos" | "windows" => u16::MAX as i64,
                    _ => {
                        return Err(Error::new(
                            "E152",
                            span,
                            "key.native platform must be android, macos, windows, or xkb",
                        ));
                    }
                };
                if !(0..=maximum).contains(&code) {
                    return Err(Error::new(
                        "E152",
                        span,
                        format!("key.native {platform} code must be in 0..={maximum}"),
                    ));
                }
                Ok(Type::PhysicalKey)
            }
            "key.location" => {
                let [Expr::Str(value)] = args.as_slice() else {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.location expects one string literal",
                    ));
                };
                if !matches!(value.as_str(), "standard" | "left" | "right" | "numpad") {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.location must be standard, left, right, or numpad",
                    ));
                }
                Ok(Type::KeyLocation)
            }
            "key.modifiers" => {
                if args.len() != 4 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.modifiers expects shift, control, alt, and logo booleans",
                    ));
                }
                for value in args {
                    require_type(&expr_type(value, env, document, span)?, &Type::Bool, span)?;
                }
                Ok(Type::KeyModifiers)
            }
            "key.latin" => {
                if args.len() != 2 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "key.latin expects a logical key and physical key",
                    ));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Key, span)?;
                require_type(
                    &expr_type(&args[1], env, document, span)?,
                    &Type::PhysicalKey,
                    span,
                )?;
                Ok(Type::Option(Box::new(Type::Str)))
            }
            "len" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "len expects one argument"));
                }
                match expr_type(&args[0], env, document, span)? {
                    Type::List(_) | Type::Str | Type::Bytes => Ok(Type::I64),
                    actual => Err(Error::new(
                        "E152",
                        span,
                        format!("len does not accept `{}`", actual.display()),
                    )),
                }
            }
            "empty" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "empty expects one argument"));
                }
                match expr_type(&args[0], env, document, span)? {
                    Type::List(_) | Type::Str | Type::Bytes => Ok(Type::Bool),
                    actual => Err(Error::new(
                        "E152",
                        span,
                        format!("empty does not accept `{}`", actual.display()),
                    )),
                }
            }
            "trim" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "trim expects one argument"));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Str)
            }
            "some" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "some expects one argument"));
                }
                Ok(Type::Option(Box::new(expr_type(
                    &args[0], env, document, span,
                )?)))
            }
            "markdown" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "markdown expects one argument"));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Markdown)
            }
            "markdown_images" => {
                if args.len() != 1 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "markdown_images expects one argument",
                    ));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Markdown,
                    span,
                )?;
                Ok(Type::List(Box::new(Type::Str)))
            }
            "editor" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "editor expects one argument"));
                }
                require_type(&expr_type(&args[0], env, document, span)?, &Type::Str, span)?;
                Ok(Type::Editor)
            }
            "encoded" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "encoded expects one argument"));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Bytes,
                    span,
                )?;
                Ok(Type::Image)
            }
            "rgba" => {
                if args.len() != 3 {
                    return Err(Error::new(
                        "E152",
                        span,
                        "rgba expects width, height, and pixel bytes",
                    ));
                }
                for (value, label) in [(&args[0], "rgba width"), (&args[1], "rgba height")] {
                    require_type(&expr_type(value, env, document, span)?, &Type::I64, span)?;
                    require_literal_range(value, 0.0, Some(u32::MAX as f64), label, span)?;
                }
                require_type(
                    &expr_type(&args[2], env, document, span)?,
                    &Type::Bytes,
                    span,
                )?;
                if let (Expr::I64(width), Expr::I64(height), Expr::Bytes(pixels)) =
                    (&args[0], &args[1], &args[2])
                {
                    let expected = (*width as u128) * (*height as u128) * 4;
                    if expected != pixels.len() as u128 {
                        return Err(Error::new(
                            "E152",
                            span,
                            "rgba pixel data must contain width × height × 4 bytes",
                        ));
                    }
                }
                Ok(Type::Image)
            }
            "aborted" => {
                if args.len() != 1 {
                    return Err(Error::new("E152", span, "aborted expects one argument"));
                }
                require_type(
                    &expr_type(&args[0], env, document, span)?,
                    &Type::Option(Box::new(Type::TaskHandle)),
                    span,
                )?;
                Ok(Type::Bool)
            }
            _ => {
                let function = extern_function(document, name, ExternKind::Sync, span)?;
                check_call_args(function, args, env, document, span)?;
                Ok(function.output.clone())
            }
        },
        Expr::Unary { op, value } => {
            let actual = expr_type(value, env, document, span)?;
            match op {
                UnaryOp::Not => {
                    require_type(&actual, &Type::Bool, span)?;
                    Ok(Type::Bool)
                }
                UnaryOp::Neg if matches!(actual, Type::I64 | Type::F64 | Type::Vector) => {
                    Ok(actual)
                }
                UnaryOp::Neg => Err(Error::new(
                    "E153",
                    span,
                    "negation expects i64, f64, or vector",
                )),
            }
        }
        Expr::Binary { left, op, right } => {
            let left = expr_type(left, env, document, span)?;
            let right = expr_type(right, env, document, span)?;
            match op {
                BinaryOp::And | BinaryOp::Or => {
                    require_type(&left, &Type::Bool, span)?;
                    require_type(&right, &Type::Bool, span)?;
                    Ok(Type::Bool)
                }
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq => {
                    if contains_task_handle(&left) || contains_task_handle(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "task handles are opaque; use `aborted(handle)`",
                        ));
                    }
                    if contains_mouse_click(&left) || contains_mouse_click(&right) {
                        return Err(Error::new(
                            "E153",
                            span,
                            "mouse-click values are opaque; compare their kind or position",
                        ));
                    }
                    if !matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
                        && matches!(
                            left,
                            Type::Padding
                                | Type::Point
                                | Type::PointU32
                                | Type::Vector
                                | Type::Size
                                | Type::Rectangle
                                | Type::RectangleU32
                                | Type::Transformation
                        )
                    {
                        return Err(Error::new(
                            "E153",
                            span,
                            format!(
                                "operator `{op:?}` does not accept `{}` and `{}`",
                                left.display(),
                                right.display()
                            ),
                        ));
                    }
                    if !matches!((&left, &right), (Type::Degrees | Type::Radians, Type::F64)) {
                        require_type(&left, &right, span)?;
                    }
                    Ok(Type::Bool)
                }
                _ => arithmetic_type(&left, *op, &right).ok_or_else(|| {
                    Error::new(
                        "E153",
                        span,
                        format!(
                            "operator `{op:?}` does not accept `{}` and `{}`",
                            left.display(),
                            right.display()
                        ),
                    )
                }),
            }
        }
    }
}

mod fields;
mod validation;

pub(super) use fields::*;
pub(super) use validation::*;
