use std::collections::BTreeMap;

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
    List(Box<Type>),
    Option(Box<Type>),
    Combo(Box<Type>),
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
            Self::List(inner) => format!("::std::vec::Vec<{}>", inner.rust(structs)),
            Self::Option(inner) => format!("::std::option::Option<{}>", inner.rust(structs)),
            Self::Combo(inner) => {
                format!("::iced::widget::combo_box::State<{}>", inner.rust(structs))
            }
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
            Self::List(inner) => format!("[{}]", inner.display()),
            Self::Option(inner) => format!("{}?", inner.display()),
            Self::Combo(inner) => format!("combo[{}]", inner.display()),
            Self::Named(name) => name.clone(),
            Self::Unit => "unit".into(),
            Self::Unknown => "unknown".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    pub app: String,
    pub extern_path: Option<String>,
    pub structs: Vec<ExternStruct>,
    pub functions: Vec<ExternFn>,
    pub subscriptions: Vec<Subscription>,
    pub theme: BTreeMap<String, String>,
    pub states: Vec<State>,
    pub components: Vec<Component>,
    pub handlers: Vec<Handler>,
    pub view: ViewNode,
}

#[derive(Clone, Debug)]
pub struct ExternStruct {
    pub name: String,
    pub rust_path: String,
    pub fields: Vec<(String, Type)>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ExternFn {
    pub kind: ExternKind,
    pub name: String,
    pub rust_path: String,
    pub params: Vec<(String, Type)>,
    pub output: Type,
    pub error: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternKind {
    Future,
    Component,
    Task,
    Subscription,
}

#[derive(Clone, Debug)]
pub struct Subscription {
    pub function: String,
    pub args: Vec<Expr>,
    pub route: Route,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct State {
    pub name: String,
    pub ty: Type,
    pub initial: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub root: ViewNode,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Handler {
    pub name: String,
    pub params: Vec<HandlerParam>,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct HandlerParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Assign {
        target: String,
        value: Expr,
        span: Span,
    },
    ReturnIf {
        condition: Expr,
        span: Span,
    },
    Run {
        kind: EffectKind,
        function: String,
        args: Vec<Expr>,
        success: Route,
        error: Option<Route>,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    Future,
    Task,
}

#[derive(Clone, Debug)]
pub struct Route {
    pub handler: String,
    pub args: Vec<RouteArg>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum RouteArg {
    Expr(Expr),
    Payload,
}

#[derive(Clone, Debug)]
pub struct Id {
    pub name: String,
    pub key: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum ViewNode {
    Layout {
        kind: Layout,
        options: LayoutOptions,
        id: Option<Id>,
        styles: Vec<String>,
        children: Vec<ViewNode>,
        span: Span,
    },
    Text {
        value: Expr,
        styles: Vec<String>,
        span: Span,
    },
    Input {
        label: String,
        id: Option<Id>,
        binding: String,
        hint: String,
        disabled: Option<Expr>,
        options: InputOptions,
        styles: Vec<String>,
        span: Span,
    },
    Button {
        label: Option<String>,
        content: Option<Box<ViewNode>>,
        id: Option<Id>,
        disabled: Option<Expr>,
        options: ButtonOptions,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    Checkbox {
        label: Expr,
        id: Option<Id>,
        checked: Expr,
        disabled: Option<Expr>,
        options: BoolControlOptions,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    Toggler {
        label: Expr,
        checked: Expr,
        disabled: Option<Expr>,
        options: BoolControlOptions,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    Slider {
        value: Expr,
        min: Expr,
        max: Expr,
        step: Expr,
        vertical: bool,
        styles: Vec<String>,
        route: Route,
        release: Option<Route>,
        span: Span,
    },
    Progress {
        value: Expr,
        min: Expr,
        max: Expr,
        vertical: bool,
        styles: Vec<String>,
        span: Span,
    },
    Radio {
        label: Expr,
        value: Expr,
        selected: Expr,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    PickList {
        options: Expr,
        selected: Expr,
        options_config: PickListOptions,
        route: Route,
        span: Span,
    },
    ComboBox {
        state: String,
        selected: Expr,
        placeholder: String,
        options: ComboBoxOptions,
        route: Route,
        span: Span,
    },
    Rule {
        axis: Axis,
        thickness: Expr,
        styles: Vec<String>,
        span: Span,
    },
    Space {
        width: Option<Expr>,
        height: Option<Expr>,
        styles: Vec<String>,
        span: Span,
    },
    If {
        condition: Expr,
        children: Vec<ViewNode>,
        span: Span,
    },
    For {
        item: String,
        items: Expr,
        children: Vec<ViewNode>,
        span: Span,
    },
    Component {
        name: String,
        args: Vec<Expr>,
        id: Option<Id>,
        span: Span,
    },
    ExternComponent {
        function: String,
        args: Vec<Expr>,
        route: Option<Route>,
        span: Span,
    },
    Media {
        kind: MediaKind,
        source: Expr,
        options: MediaOptions,
        span: Span,
    },
    Tooltip {
        options: TooltipOptions,
        content: Box<ViewNode>,
        tip: Box<ViewNode>,
        span: Span,
    },
    MouseArea {
        options: MouseAreaOptions,
        content: Box<ViewNode>,
        span: Span,
    },
    Float {
        scale: Expr,
        x: Expr,
        y: Expr,
        content: Box<ViewNode>,
        span: Span,
    },
    Pin {
        width: Option<LengthValue>,
        height: Option<LengthValue>,
        x: Expr,
        y: Expr,
        content: Box<ViewNode>,
        span: Span,
    },
    Sensor {
        options: SensorOptions,
        content: Box<ViewNode>,
        span: Span,
    },
    Responsive {
        breakpoint: Expr,
        width: Option<LengthValue>,
        height: Option<LengthValue>,
        narrow: Box<ViewNode>,
        wide: Box<ViewNode>,
        span: Span,
    },
}

#[derive(Clone, Debug, Default)]
pub struct InputOptions {
    pub secure: Option<Expr>,
    pub submit: Option<Route>,
    pub paste: Option<Route>,
    pub width: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub align: Option<InputAlignment>,
    pub font: Option<FontPreset>,
    pub icon: Option<char>,
    pub icon_side: Option<IconSide>,
    pub icon_size: Option<Expr>,
    pub icon_spacing: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub clip: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontPreset {
    Default,
    Monospace,
}

#[derive(Clone, Debug, Default)]
pub struct BoolControlOptions {
    pub size: Option<Expr>,
    pub width: Option<LengthValue>,
    pub spacing: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub wrapping: Option<TextWrapping>,
    pub font: Option<FontPreset>,
    pub alignment: Option<TextAlignment>,
    pub icon: Option<char>,
    pub icon_size: Option<Expr>,
    pub icon_line_height: Option<Expr>,
    pub icon_shaping: Option<TextShaping>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShaping {
    Auto,
    Basic,
    Advanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextWrapping {
    None,
    Word,
    Glyph,
    WordOrGlyph,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlignment {
    Default,
    Left,
    Center,
    Right,
    Justified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Default)]
pub struct PickListOptions {
    pub placeholder: Option<Expr>,
    pub width: Option<LengthValue>,
    pub menu_height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub open: Option<Route>,
    pub close: Option<Route>,
}

#[derive(Clone, Debug, Default)]
pub struct ComboBoxOptions {
    pub width: Option<LengthValue>,
    pub menu_height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub input: Option<Route>,
    pub hover: Option<Route>,
    pub open: Option<Route>,
    pub close: Option<Route>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Svg,
}

#[derive(Clone, Debug, Default)]
pub struct MediaOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub fit: Option<ContentFit>,
    pub rotation: Option<Expr>,
    pub opacity: Option<Expr>,
    pub filter: Option<ImageFilter>,
    pub scale: Option<Expr>,
    pub expand: Option<Expr>,
    pub radius: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum LengthValue {
    Fill,
    Shrink,
    Fixed(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentFit {
    Contain,
    Cover,
    Fill,
    None,
    ScaleDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFilter {
    Linear,
    Nearest,
}

#[derive(Clone, Debug)]
pub struct TooltipOptions {
    pub position: TooltipPosition,
    pub gap: Expr,
    pub padding: Expr,
    pub delay_ms: Expr,
    pub snap: Expr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
    FollowCursor,
}

#[derive(Clone, Debug, Default)]
pub struct MouseAreaOptions {
    pub press: Option<Route>,
    pub release: Option<Route>,
    pub double_click: Option<Route>,
    pub right_press: Option<Route>,
    pub right_release: Option<Route>,
    pub middle_press: Option<Route>,
    pub middle_release: Option<Route>,
    pub enter: Option<Route>,
    pub move_route: Option<Route>,
    pub scroll: Option<Route>,
    pub exit: Option<Route>,
    pub interaction: Option<MouseInteraction>,
}

#[derive(Clone, Debug, Default)]
pub struct SensorOptions {
    pub show: Option<Route>,
    pub resize: Option<Route>,
    pub hide: Option<Route>,
    pub key: Option<Expr>,
    pub anticipate: Option<Expr>,
    pub delay_ms: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseInteraction {
    None,
    Hidden,
    Idle,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    ResizingHorizontally,
    ResizingVertically,
    ResizingDiagonallyUp,
    ResizingDiagonallyDown,
    ResizingColumn,
    ResizingRow,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Column,
    Row,
    Scroll,
    Grid,
    Stack,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutOptions {
    pub columns: Option<Expr>,
    pub clip: Option<Expr>,
    pub scroll: Option<ScrollOptions>,
}

#[derive(Clone, Debug)]
pub struct ScrollOptions {
    pub direction: ScrollDirection,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub hidden_bar: bool,
    pub bar_width: Option<Expr>,
    pub bar_margin: Option<Expr>,
    pub scroller_width: Option<Expr>,
    pub bar_spacing: Option<Expr>,
    pub anchor_x: ScrollAnchor,
    pub anchor_y: ScrollAnchor,
    pub auto_scroll: Option<Expr>,
    pub route: Option<Route>,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            direction: ScrollDirection::Vertical,
            width: None,
            height: None,
            hidden_bar: false,
            bar_width: None,
            bar_margin: None,
            scroller_width: None,
            bar_spacing: None,
            anchor_x: ScrollAnchor::Start,
            anchor_y: ScrollAnchor::Start,
            auto_scroll: None,
            route: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAnchor {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    EmptyList,
    List(Vec<Expr>),
    None,
    Path(Vec<String>),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}
