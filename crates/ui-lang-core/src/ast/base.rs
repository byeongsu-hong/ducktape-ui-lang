use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn line(line: usize) -> Self {
        Self { line, column: 1 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Bool,
    I64,
    F64,
    Str,
    Bytes,
    Image,
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Combo(Box<Type>),
    Markdown,
    Editor,
    Event,
    Key,
    PhysicalKey,
    KeyLocation,
    KeyPress,
    KeyRelease,
    KeyModifiers,
    Pixels,
    Padding,
    Degrees,
    Radians,
    Point,
    PointU32,
    Vector,
    Size,
    Rectangle,
    RectangleU32,
    Transformation,
    MouseButton,
    MouseCursor,
    MouseClick,
    TouchFinger,
    SystemInfo,
    Instant,
    WindowId,
    WidgetId,
    WidgetTarget,
    TaskHandle,
    Named(String),
    Unit,
    Unknown,
}

impl Type {
    pub fn rust(&self, structs: &[ExternStruct]) -> String {
        match self {
            Self::Bool => "bool".into(),
            Self::I64 => "i64".into(),
            Self::F64 => "f64".into(),
            Self::Str => "::std::string::String".into(),
            Self::Bytes => "::std::vec::Vec<u8>".into(),
            Self::Image => "::iced::widget::image::Handle".into(),
            Self::List(inner) => format!("::std::vec::Vec<{}>", inner.rust(structs)),
            Self::Option(inner) => format!("::std::option::Option<{}>", inner.rust(structs)),
            Self::Result(output, error) => format!(
                "::std::result::Result<{}, {}>",
                output.rust(structs),
                error.rust(structs)
            ),
            Self::Combo(inner) => {
                format!("::iced::widget::combo_box::State<{}>", inner.rust(structs))
            }
            Self::Markdown => "::iced::widget::markdown::Content".into(),
            Self::Editor => "::iced::widget::text_editor::Content".into(),
            Self::Event => "::iced::Event".into(),
            Self::Key => "::iced::keyboard::Key".into(),
            Self::PhysicalKey => "::iced::keyboard::key::Physical".into(),
            Self::KeyLocation => "::iced::keyboard::Location".into(),
            Self::KeyPress => "__IceKeyPress".into(),
            Self::KeyRelease => "__IceKeyRelease".into(),
            Self::KeyModifiers => "::iced::keyboard::Modifiers".into(),
            Self::Pixels => "::iced::Pixels".into(),
            Self::Padding => "::iced::Padding".into(),
            Self::Degrees => "::iced::Degrees".into(),
            Self::Radians => "::iced::Radians".into(),
            Self::Point => "::iced::Point".into(),
            Self::PointU32 => "::iced::Point<u32>".into(),
            Self::Vector => "::iced::Vector".into(),
            Self::Size => "::iced::Size".into(),
            Self::Rectangle => "::iced::Rectangle".into(),
            Self::RectangleU32 => "::iced::Rectangle<u32>".into(),
            Self::Transformation => "::iced::Transformation".into(),
            Self::MouseButton => "::iced::mouse::Button".into(),
            Self::MouseCursor => "::iced::mouse::Cursor".into(),
            Self::MouseClick => "::iced::advanced::mouse::Click".into(),
            Self::TouchFinger => "::iced::touch::Finger".into(),
            Self::SystemInfo => "__IceSystemInfo".into(),
            Self::Instant => "::iced::time::Instant".into(),
            Self::WindowId => "::iced::window::Id".into(),
            Self::WidgetId => "::iced::widget::Id".into(),
            Self::WidgetTarget => "__IceWidgetTarget".into(),
            Self::TaskHandle => "::iced::task::Handle".into(),
            Self::Named(name) => structs
                .iter()
                .find(|item| item.name == *name)
                .map_or_else(|| name.clone(), |item| item.rust_path.clone()),
            Self::Unit => "()".into(),
            Self::Unknown => "_".into(),
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Bool => "bool".into(),
            Self::I64 => "i64".into(),
            Self::F64 => "f64".into(),
            Self::Str => "str".into(),
            Self::Bytes => "bytes".into(),
            Self::Image => "image".into(),
            Self::List(inner) => format!("[{}]", inner.display()),
            Self::Option(inner) => format!("{}?", inner.display()),
            Self::Result(output, error) => {
                format!("result[{},{}]", output.display(), error.display())
            }
            Self::Combo(inner) => format!("combo[{}]", inner.display()),
            Self::Markdown => "markdown".into(),
            Self::Editor => "editor".into(),
            Self::Event => "event".into(),
            Self::Key => "key".into(),
            Self::PhysicalKey => "physical-key".into(),
            Self::KeyLocation => "key-location".into(),
            Self::KeyPress => "key-press".into(),
            Self::KeyRelease => "key-release".into(),
            Self::KeyModifiers => "key-modifiers".into(),
            Self::Pixels => "pixels".into(),
            Self::Padding => "padding".into(),
            Self::Degrees => "degrees".into(),
            Self::Radians => "radians".into(),
            Self::Point => "point".into(),
            Self::PointU32 => "point-u32".into(),
            Self::Vector => "vector".into(),
            Self::Size => "size".into(),
            Self::Rectangle => "rectangle".into(),
            Self::RectangleU32 => "rectangle-u32".into(),
            Self::Transformation => "transformation".into(),
            Self::MouseButton => "mouse-button".into(),
            Self::MouseCursor => "mouse-cursor".into(),
            Self::MouseClick => "mouse-click".into(),
            Self::TouchFinger => "touch-finger".into(),
            Self::SystemInfo => "system-info".into(),
            Self::Instant => "instant".into(),
            Self::WindowId => "window-id".into(),
            Self::WidgetId => "widget-id".into(),
            Self::WidgetTarget => "widget-target".into(),
            Self::TaskHandle => "task-handle".into(),
            Self::Named(name) => name.clone(),
            Self::Unit => "unit".into(),
            Self::Unknown => "unknown".into(),
        }
    }
}
