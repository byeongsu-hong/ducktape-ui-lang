use super::*;

pub(in crate::check) fn keyboard_variant<'a>(
    name: &str,
    args: &'a [Expr],
    span: &Span,
) -> Result<&'a str, Error> {
    if args.len() != 1 {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects one string literal"),
        ));
    }
    let Expr::Str(value) = &args[0] else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects a string literal"),
        ));
    };
    let mut chars = value.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_uppercase())
        || !chars.all(|ch| ch.is_ascii_alphanumeric())
    {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects an exact iced Rust variant like `Enter` or `KeyA`"),
        ));
    }
    Ok(value)
}

pub(in crate::check) fn animation_inner(
    expr: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    let Type::Animation(inner) = expr_type(expr, env, document, span)? else {
        return Err(Error::new("E152", span, "expected animation state"));
    };
    Ok(*inner)
}

pub(in crate::check) fn check_animation_instant(
    name: &str,
    args: &[Expr],
    required: usize,
    optional_instant: bool,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    let valid = args.len() == required || optional_instant && args.len() == required + 1;
    if !valid {
        return Err(Error::new(
            "E152",
            span,
            format!(
                "{name} expects {required}{} argument(s)",
                if optional_instant {
                    " or one more instant"
                } else {
                    ""
                }
            ),
        ));
    }
    if args.len() > required {
        require_type(
            &expr_type(&args[required], env, document, span)?,
            &Type::Instant,
            span,
        )?;
    }
    Ok(())
}

pub(in crate::check) fn check_builtin_args(
    name: &str,
    args: &[Expr],
    expected: &[Type],
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if args.len() != expected.len() {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects {} argument(s)", expected.len()),
        ));
    }
    for (value, expected) in args.iter().zip(expected) {
        require_type(&expr_type(value, env, document, span)?, expected, span)?;
    }
    Ok(())
}

pub(in crate::check) fn check_u32_literals(
    name: &str,
    args: &[Expr],
    span: &Span,
) -> Result<(u32, u32), Error> {
    let [Expr::I64(first), Expr::I64(second)] = args else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects two integer literals"),
        ));
    };
    let (Ok(first), Ok(second)) = (u32::try_from(*first), u32::try_from(*second)) else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} dimensions must be in 0..={}", u32::MAX),
        ));
    };
    Ok((first, second))
}

pub(in crate::check) fn check_u32_literal(
    name: &str,
    args: &[Expr],
    span: &Span,
) -> Result<u32, Error> {
    let [Expr::I64(value)] = args else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects one integer literal"),
        ));
    };
    u32::try_from(*value).map_err(|_| {
        Error::new(
            "E152",
            span,
            format!("{name} value must be in 0..={}", u32::MAX),
        )
    })
}

pub(in crate::check) fn check_u8_literal(
    name: &str,
    args: &[Expr],
    span: &Span,
) -> Result<u8, Error> {
    let [Expr::I64(value)] = args else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects one integer literal"),
        ));
    };
    u8::try_from(*value).map_err(|_| {
        Error::new(
            "E152",
            span,
            format!("{name} value must be in 0..={}", u8::MAX),
        )
    })
}

pub(in crate::check) fn check_i32_literal(
    name: &str,
    args: &[Expr],
    span: &Span,
) -> Result<i32, Error> {
    let value = match args {
        [Expr::I64(value)] => *value,
        [
            Expr::Unary {
                op: UnaryOp::Neg,
                value,
            },
        ] => match value.as_ref() {
            Expr::I64(value) => value.checked_neg().ok_or_else(|| {
                Error::new("E152", span, format!("{name} integer literal is too small"))
            })?,
            _ => {
                return Err(Error::new(
                    "E152",
                    span,
                    format!("{name} expects one integer literal"),
                ));
            }
        },
        _ => {
            return Err(Error::new(
                "E152",
                span,
                format!("{name} expects one integer literal"),
            ));
        }
    };
    i32::try_from(value).map_err(|_| {
        Error::new(
            "E152",
            span,
            format!("{name} value must be in {}..={}", i32::MIN, i32::MAX),
        )
    })
}

pub(in crate::check) fn check_u16_literal(
    name: &str,
    args: &[Expr],
    span: &Span,
) -> Result<u16, Error> {
    let [Expr::I64(value)] = args else {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects one integer literal"),
        ));
    };
    u16::try_from(*value).map_err(|_| {
        Error::new(
            "E152",
            span,
            format!("{name} value must be in 0..={}", u16::MAX),
        )
    })
}

pub(in crate::check) fn check_u8_literals(
    name: &str,
    args: &[Expr],
    count: usize,
    span: &Span,
) -> Result<(), Error> {
    if args.len() < count
        || !args[..count]
            .iter()
            .all(|value| matches!(value, Expr::I64(_)))
    {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} expects {count} integer literal channel(s)"),
        ));
    }
    if args[..count]
        .iter()
        .any(|value| matches!(value, Expr::I64(channel) if u8::try_from(*channel).is_err()))
    {
        return Err(Error::new(
            "E152",
            span,
            format!("{name} channels must be in 0..={}", u8::MAX),
        ));
    }
    Ok(())
}

pub(in crate::check) fn require_pixel_value(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    let actual = expr_type(value, env, document, span)?;
    if matches!(actual, Type::F64 | Type::Pixels) {
        Ok(actual)
    } else {
        Err(Error::new(
            "E101",
            span,
            format!("expected `f64` or `pixels`, got `{}`", actual.display()),
        ))
    }
}

pub(in crate::check) fn require_radius_value(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    let actual = expr_type(value, env, document, span)?;
    if matches!(actual, Type::F64 | Type::Radius) {
        Ok(actual)
    } else {
        Err(Error::new(
            "E101",
            span,
            format!("expected `f64` or `radius`, got `{}`", actual.display()),
        ))
    }
}

pub(in crate::check) fn check_length_value(
    length: &LengthValue,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
    label: &str,
) -> Result<(), Error> {
    let LengthValue::Fixed(value) = length else {
        return Ok(());
    };
    let actual = expr_type(value, env, document, span)?;
    if actual == Type::Length {
        return Ok(());
    }
    if actual != Type::F64 {
        return Err(Error::new(
            "E101",
            span,
            format!(
                "expected `f64` or `length`, got `{}` for {label}",
                actual.display()
            ),
        ));
    }
    require_literal_range(value, 0.0, None, label, span)
}

pub(in crate::check) fn require_radians_value(
    value: &Expr,
    env: &HashMap<String, Type>,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    let actual = expr_type(value, env, document, span)?;
    if matches!(actual, Type::F64 | Type::Radians) {
        Ok(actual)
    } else {
        Err(Error::new(
            "E101",
            span,
            format!("expected `f64` or `radians`, got `{}`", actual.display()),
        ))
    }
}

pub(in crate::check) fn arithmetic_type(left: &Type, op: BinaryOp, right: &Type) -> Option<Type> {
    if matches!(left, Type::I64 | Type::F64) && left == right {
        return Some(left.clone());
    }
    match (left, op, right) {
        (Type::Pixels, BinaryOp::Add | BinaryOp::Mul | BinaryOp::Div, Type::Pixels)
        | (Type::Pixels, BinaryOp::Add | BinaryOp::Mul | BinaryOp::Div, Type::F64) => {
            Some(Type::Pixels)
        }
        (Type::Degrees, BinaryOp::Mul, Type::F64) => Some(Type::Degrees),
        (Type::Radians, BinaryOp::Add, Type::Degrees)
        | (
            Type::Radians,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            Type::Radians,
        )
        | (Type::Radians, BinaryOp::Mul | BinaryOp::Div, Type::F64)
        | (Type::F64, BinaryOp::Mul, Type::Radians) => Some(Type::Radians),
        (Type::Radius, BinaryOp::Mul, Type::F64) => Some(Type::Radius),
        (Type::Point, BinaryOp::Add | BinaryOp::Sub, Type::Vector) => Some(Type::Point),
        (Type::Point, BinaryOp::Sub, Type::Point) => Some(Type::Vector),
        (Type::Vector, BinaryOp::Add | BinaryOp::Sub, Type::Vector) => Some(Type::Vector),
        (Type::Vector, BinaryOp::Mul | BinaryOp::Div, Type::F64) => Some(Type::Vector),
        (Type::Size, BinaryOp::Add | BinaryOp::Sub, Type::Size) => Some(Type::Size),
        (Type::Size, BinaryOp::Mul, Type::Vector) => Some(Type::Size),
        (Type::Size, BinaryOp::Mul | BinaryOp::Div, Type::F64) => Some(Type::Size),
        (Type::Rectangle, BinaryOp::Add | BinaryOp::Sub, Type::Vector) => Some(Type::Rectangle),
        (Type::Rectangle, BinaryOp::Mul, Type::F64) => Some(Type::Rectangle),
        (Type::Transformation, BinaryOp::Mul, Type::Transformation) => Some(Type::Transformation),
        (
            Type::Point
            | Type::Vector
            | Type::Size
            | Type::Rectangle
            | Type::MouseCursor
            | Type::MouseClick,
            BinaryOp::Mul,
            Type::Transformation,
        ) => Some(left.clone()),
        _ => None,
    }
}
