use super::*;

#[test]
fn parses_typed_keyboard_values() {
    let source = example!("keyboard_values.ice");
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
    let source = example!("pointer_values.ice");
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
    let source = example!("transformation_values.ice");
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
    let source = example!("geometry_values.ice");
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

    let source = example!("padding_angles.ice");
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
