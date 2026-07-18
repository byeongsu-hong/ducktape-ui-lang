use super::*;

pub(in crate::check) fn contains_task_handle(ty: &Type) -> bool {
    match ty {
        Type::TaskHandle => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => contains_task_handle(inner),
        Type::Result(output, error) => contains_task_handle(output) || contains_task_handle(error),
        _ => false,
    }
}

pub(in crate::check) fn contains_mouse_click(ty: &Type) -> bool {
    match ty {
        Type::MouseClick => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => contains_mouse_click(inner),
        Type::Result(output, error) => contains_mouse_click(output) || contains_mouse_click(error),
        _ => false,
    }
}

pub(in crate::check) fn field_type(
    ty: &Type,
    field: &str,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    if let Type::Option(inner) = ty
        && **inner == Type::WidgetTarget
    {
        return Ok(Type::Option(Box::new(field_type(
            inner, field, document, span,
        )?)));
    }
    let found = match ty {
        Type::Named(name) => {
            let item = document
                .structs
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| {
                    Error::new("E151", span, format!("unknown extern struct `{name}`"))
                })?;
            return item
                .fields
                .iter()
                .find(|(name, _)| name == field)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| {
                    Error::new(
                        "E151",
                        span,
                        format!("struct `{name}` has no field `{field}`"),
                    )
                });
        }
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
        Type::Key => match field {
            "kind" => Some(Type::Str),
            "named" | "character" => Some(Type::Option(Box::new(Type::Str))),
            _ => None,
        },
        Type::PhysicalKey => match field {
            "kind" => Some(Type::Str),
            "code" | "native_platform" => Some(Type::Option(Box::new(Type::Str))),
            "native_code" => Some(Type::Option(Box::new(Type::I64))),
            _ => None,
        },
        Type::KeyLocation => match field {
            "name" => Some(Type::Str),
            _ => None,
        },
        Type::KeyModifiers => match field {
            "shift" | "control" | "alt" | "logo" | "command" | "jump" | "macos_command" => {
                Some(Type::Bool)
            }
            _ => None,
        },
        Type::Pixels => match field {
            "value" => Some(Type::F64),
            _ => None,
        },
        Type::Padding => match field {
            "top" | "right" | "bottom" | "left" | "x" | "y" => Some(Type::F64),
            _ => None,
        },
        Type::Degrees => match field {
            "value" => Some(Type::F64),
            _ => None,
        },
        Type::Radians => match field {
            "value" => Some(Type::F64),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::Point => match field {
            "x" | "y" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::PointU32 => match field {
            "x" | "y" => Some(Type::I64),
            _ => None,
        },
        Type::Vector => match field {
            "x" | "y" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::Size => match field {
            "width" | "height" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::Rectangle => match field {
            "x" | "y" | "width" | "height" => Some(Type::F64),
            "center" | "position" => Some(Type::Point),
            "center_x" | "center_y" | "area" => Some(Type::F64),
            "size" => Some(Type::Size),
            _ => None,
        },
        Type::RectangleU32 => match field {
            "x" | "y" | "width" | "height" => Some(Type::I64),
            _ => None,
        },
        Type::Transformation => match field {
            "scale_factor" => Some(Type::F64),
            "translation" => Some(Type::Vector),
            "matrix" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::MouseButton => match field {
            "kind" => Some(Type::Str),
            "number" => Some(Type::Option(Box::new(Type::I64))),
            _ => None,
        },
        Type::MouseCursor => match field {
            "kind" => Some(Type::Str),
            "position" => Some(Type::Option(Box::new(Type::Point))),
            "levitating" => Some(Type::Bool),
            _ => None,
        },
        Type::MouseClick => match field {
            "kind" => Some(Type::Str),
            "position" => Some(Type::Point),
            _ => None,
        },
        Type::TouchFinger => match field {
            "id" => Some(Type::Str),
            _ => None,
        },
        Type::SystemInfo => match field {
            "system_name" | "system_kernel" | "system_version" | "system_short_version" => {
                Some(Type::Option(Box::new(Type::Str)))
            }
            "cpu_brand" | "graphics_backend" | "graphics_adapter" => Some(Type::Str),
            "cpu_cores" | "memory_used" => Some(Type::Option(Box::new(Type::I64))),
            "memory_total" => Some(Type::I64),
            _ => None,
        },
        Type::WidgetTarget => match field {
            "kind" => Some(Type::Str),
            "id" => Some(Type::Option(Box::new(Type::WidgetId))),
            "x" | "y" | "width" | "height" => Some(Type::F64),
            "visible_x" | "visible_y" | "visible_width" | "visible_height" | "content_x"
            | "content_y" | "content_width" | "content_height" | "translation_x"
            | "translation_y" => Some(Type::Option(Box::new(Type::F64))),
            "content" => Some(Type::Option(Box::new(Type::Str))),
            _ => None,
        },
        _ => None,
    };
    found.ok_or_else(|| {
        Error::new(
            "E151",
            span,
            format!("type `{}` has no field `{field}`", ty.display()),
        )
    })
}
