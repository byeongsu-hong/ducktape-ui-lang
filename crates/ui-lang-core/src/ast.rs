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
    Markdown,
    Editor,
    KeyPress,
    KeyRelease,
    KeyModifiers,
    SystemInfo,
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
            Self::Markdown => "::iced::widget::markdown::Content".into(),
            Self::Editor => "::iced::widget::text_editor::Content".into(),
            Self::KeyPress => "__IceKeyPress".into(),
            Self::KeyRelease => "__IceKeyRelease".into(),
            Self::KeyModifiers => "__IceKeyModifiers".into(),
            Self::SystemInfo => "__IceSystemInfo".into(),
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
            Self::Markdown => "markdown".into(),
            Self::Editor => "editor".into(),
            Self::KeyPress => "key-press".into(),
            Self::KeyRelease => "key-release".into(),
            Self::KeyModifiers => "key-modifiers".into(),
            Self::SystemInfo => "system-info".into(),
            Self::Named(name) => name.clone(),
            Self::Unit => "unit".into(),
            Self::Unknown => "unknown".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    pub app: String,
    pub settings: AppSettings,
    pub extern_path: Option<String>,
    pub structs: Vec<ExternStruct>,
    pub functions: Vec<ExternFn>,
    pub subscriptions: Vec<Subscription>,
    pub theme: BTreeMap<String, String>,
    pub fonts: Vec<FontDecl>,
    pub qr_codes: Vec<QrData>,
    pub states: Vec<State>,
    pub components: Vec<Component>,
    pub handlers: Vec<Handler>,
    pub view: ViewNode,
}

#[derive(Clone, Debug, Default)]
pub struct AppSettings {
    pub title: Option<String>,
    pub id: Option<String>,
    pub default_text_size: Option<f64>,
    pub antialiasing: Option<bool>,
    pub vsync: Option<bool>,
    pub scale_factor: Option<f64>,
    pub window: Option<WindowSettings>,
}

#[derive(Clone, Debug, Default)]
pub struct WindowSettings {
    pub size: Option<(f64, f64)>,
    pub maximized: Option<bool>,
    pub fullscreen: Option<bool>,
    pub position: Option<WindowPosition>,
    pub min_size: Option<(f64, f64)>,
    pub max_size: Option<(f64, f64)>,
    pub visible: Option<bool>,
    pub resizable: Option<bool>,
    pub closeable: Option<bool>,
    pub minimizable: Option<bool>,
    pub decorations: Option<bool>,
    pub transparent: Option<bool>,
    pub blur: Option<bool>,
    pub level: Option<WindowLevel>,
    pub exit_on_close_request: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowPosition {
    Default,
    Centered,
    Specific(f64, f64),
}

#[derive(Clone, Copy, Debug)]
pub enum WindowLevel {
    Normal,
    AlwaysOnBottom,
    AlwaysOnTop,
}

#[derive(Clone, Debug)]
pub struct FontDecl {
    pub name: String,
    pub family: FontFamily,
    pub weight: FontWeight,
    pub stretch: FontStretch,
    pub style: FontStyle,
    pub default: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum FontFamily {
    Named(String),
    Serif,
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

#[derive(Clone, Copy, Debug)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

#[derive(Clone, Copy, Debug)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

#[derive(Clone, Copy, Debug)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Debug)]
pub struct QrData {
    pub name: String,
    pub data: QrPayload,
    pub correction: Option<QrCorrection>,
    pub version: Option<QrVersion>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QrPayload {
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrVersion {
    Normal(u8),
    Micro(u8),
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
    pub source: SubscriptionSource,
    pub route: Route,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum SubscriptionSource {
    Extern { function: String, args: Vec<Expr> },
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    SystemTheme,
    Touch(TouchEvent),
    Window(WindowEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardEvent {
    Press,
    Release,
    Modifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEvent {
    Entered,
    Left,
    Moved,
    Pressed,
    Released,
    Wheel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchEvent {
    Pressed,
    Moved,
    Lifted,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEvent {
    Frame,
    Opened,
    Closed,
    Moved,
    Resized,
    Rescaled,
    CloseRequested,
    Focused,
    Unfocused,
    FileHovered,
    FileDropped,
    FilesHoveredLeft,
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
    ClipboardWrite {
        primary: bool,
        value: Expr,
        span: Span,
    },
    WidgetOperation {
        operation: WidgetOperation,
        route: Option<Route>,
        span: Span,
    },
    WindowOperation {
        operation: WindowOperation,
        route: Option<Route>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum WidgetOperation {
    FocusPrevious,
    FocusNext,
    Focus { id: String },
    Focused { id: String },
    CursorFront { id: String },
    CursorEnd { id: String },
    Cursor { id: String, position: Expr },
    SelectAll { id: String },
    Select { id: String, start: Expr, end: Expr },
    Snap { id: String, x: Expr, y: Expr },
    SnapEnd { id: String },
    ScrollTo { id: String, x: Expr, y: Expr },
    ScrollBy { id: String, x: Expr, y: Expr },
}

#[derive(Clone, Debug)]
pub enum WindowOperation {
    Close,
    Drag,
    DragResize(WindowDirection),
    Resize(Expr, Expr),
    Resizable(Expr),
    MinSize(Option<(Expr, Expr)>),
    MaxSize(Option<(Expr, Expr)>),
    ResizeIncrements(Option<(Expr, Expr)>),
    Size,
    IsMaximized,
    Maximize(Expr),
    IsMinimized,
    Minimize(Expr),
    Position,
    ScaleFactor,
    Move(Expr, Expr),
    Mode,
    SetMode(WindowMode),
    ToggleMaximize,
    ToggleDecorations,
    Attention(Option<WindowAttention>),
    Focus,
    SetLevel(WindowLevel),
    SystemMenu,
    MousePassthrough(Expr),
    MonitorSize,
    AutomaticTabbing(Expr),
}

#[derive(Clone, Copy, Debug)]
pub enum WindowDirection {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowMode {
    Windowed,
    Fullscreen,
    Hidden,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowAttention {
    Critical,
    Informational,
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
pub struct ComponentArg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub enum ViewNode {
    Layout {
        kind: Layout,
        options: Box<LayoutOptions>,
        id: Option<Id>,
        styles: Vec<String>,
        children: Vec<ViewNode>,
        span: Span,
    },
    Text {
        value: Expr,
        options: TextOptions,
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
        options: Box<SliderOptions>,
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
        options: ProgressOptions,
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
        options: RuleOptions,
        styles: Vec<String>,
        span: Span,
    },
    QrCode {
        data: String,
        cell_size: Option<Expr>,
        total_size: Option<Expr>,
        cell: Option<String>,
        background: Option<String>,
        span: Span,
    },
    Space {
        width: Option<LengthValue>,
        height: Option<LengthValue>,
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
    KeyedColumn {
        item: String,
        items: Expr,
        key: Expr,
        options: Box<LayoutOptions>,
        child: Box<ViewNode>,
        span: Span,
    },
    Lazy {
        dependency: Expr,
        binding: String,
        child: Box<ViewNode>,
        span: Span,
    },
    Markdown {
        content: String,
        options: MarkdownOptions,
        route: Route,
        span: Span,
    },
    TextEditor {
        binding: String,
        id: Option<Id>,
        disabled: Option<Expr>,
        options: TextEditorOptions,
        span: Span,
    },
    Table {
        item: String,
        rows: Expr,
        options: TableOptions,
        columns: Vec<TableColumn>,
        span: Span,
    },
    Component {
        name: String,
        args: Vec<ComponentArg>,
        id: Option<Id>,
        content: Option<Box<ViewNode>>,
        span: Span,
    },
    Slot {
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
    Theme {
        preset: ThemePreset,
        text: Option<String>,
        background: Option<String>,
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
        content: ResponsiveContent,
        width: Option<LengthValue>,
        height: Option<LengthValue>,
        span: Span,
    },
}

#[derive(Clone, Debug, Default)]
pub struct MarkdownOptions {
    pub text_size: Option<Expr>,
    pub h1_size: Option<Expr>,
    pub h2_size: Option<Expr>,
    pub h3_size: Option<Expr>,
    pub h4_size: Option<Expr>,
    pub h5_size: Option<Expr>,
    pub h6_size: Option<Expr>,
    pub code_size: Option<Expr>,
    pub spacing: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct TextEditorOptions {
    pub placeholder: Option<String>,
    pub width: Option<Expr>,
    pub height: Option<LengthValue>,
    pub min_height: Option<Expr>,
    pub max_height: Option<Expr>,
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub padding: Option<Expr>,
    pub wrapping: Option<TextWrapping>,
    pub font: Option<FontPreset>,
    pub highlight: Option<String>,
    pub highlight_theme: Option<HighlightTheme>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighlightTheme {
    SolarizedDark,
    Base16Mocha,
    Base16Ocean,
    Base16Eighties,
    InspiredGithub,
}

#[derive(Clone, Debug, Default)]
pub struct TableOptions {
    pub width: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub padding_x: Option<Expr>,
    pub padding_y: Option<Expr>,
    pub separator: Option<Expr>,
    pub separator_x: Option<Expr>,
    pub separator_y: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct TableColumn {
    pub width: Option<LengthValue>,
    pub align_x: Option<InputAlignment>,
    pub align_y: Option<VerticalAlignment>,
    pub header: ViewNode,
    pub cell: ViewNode,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemePreset {
    Default,
    App,
    BuiltIn(String),
}

#[derive(Clone, Debug)]
pub enum ResponsiveContent {
    Breakpoint {
        breakpoint: Expr,
        narrow: Box<ViewNode>,
        wide: Box<ViewNode>,
    },
    Size {
        width: String,
        height: String,
        content: Box<ViewNode>,
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
pub struct TextOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub font: Option<FontPreset>,
    pub align_x: Option<TextAlignment>,
    pub align_y: Option<VerticalAlignment>,
    pub shaping: Option<TextShaping>,
    pub wrapping: Option<TextWrapping>,
}

#[derive(Clone, Debug)]
pub enum TextLineHeight {
    Relative(Expr),
    Absolute(Expr),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontPreset {
    Default,
    Monospace,
    Named(String),
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
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, Default)]
pub struct RuleOptions {
    pub style: Option<RuleStyle>,
    pub fill: Option<RuleFill>,
    pub color: Option<String>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub snap: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct SliderOptions {
    pub default: Option<Expr>,
    pub shift_step: Option<Expr>,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub style: SliderStyleSet,
}

#[derive(Clone, Debug, Default)]
pub struct SliderStyleSet {
    pub active: Option<SliderStyle>,
    pub hovered: Option<SliderStyle>,
    pub dragged: Option<SliderStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct SliderStyle {
    pub span: Option<Span>,
    pub rail_start: Option<String>,
    pub rail_end: Option<String>,
    pub rail_width: Option<Expr>,
    pub rail_border_color: Option<String>,
    pub rail_border_width: Option<Expr>,
    pub rail_radius: Option<Expr>,
    pub rail_radius_top_left: Option<Expr>,
    pub rail_radius_top_right: Option<Expr>,
    pub rail_radius_bottom_right: Option<Expr>,
    pub rail_radius_bottom_left: Option<Expr>,
    pub handle_shape: Option<SliderHandleShape>,
    pub handle_color: Option<String>,
    pub handle_border_color: Option<String>,
    pub handle_border_width: Option<Expr>,
    pub handle_radius: Option<Expr>,
    pub handle_radius_top_left: Option<Expr>,
    pub handle_radius_top_right: Option<Expr>,
    pub handle_radius_bottom_right: Option<Expr>,
    pub handle_radius_bottom_left: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum SliderHandleShape {
    Circle(Expr),
    Rectangle { width: u16 },
}

#[derive(Clone, Debug, Default)]
pub struct ProgressOptions {
    pub length: Option<LengthValue>,
    pub girth: Option<LengthValue>,
    pub style: Option<ProgressStyle>,
    pub background: Option<String>,
    pub bar: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressStyle {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStyle {
    Default,
    Weak,
}

#[derive(Clone, Debug)]
pub enum RuleFill {
    Full,
    Percent(Expr),
    Padded(u16),
    AsymmetricPadding(u16, u16),
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
    FillPortion(u16),
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
    pub style: Option<TooltipStyle>,
    pub background: Option<String>,
    pub text_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub shadow_color: Option<String>,
    pub shadow_x: Option<Expr>,
    pub shadow_y: Option<Expr>,
    pub shadow_blur: Option<Expr>,
    pub pixel_snap: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipStyle {
    Transparent,
    Rounded,
    Bordered,
    Dark,
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
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
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub spacing: Option<Expr>,
    pub padding: PaddingOptions,
    pub max_width: Option<Expr>,
    pub align: Option<FlexAlignment>,
    pub wrap: bool,
    pub wrap_spacing: Option<Expr>,
    pub wrap_align: Option<FlexAlignment>,
    pub fluid: Option<Expr>,
    pub grid_height: Option<GridSizing>,
    pub under: u16,
    pub scroll: Option<ScrollOptions>,
}

#[derive(Clone, Debug, Default)]
pub struct PaddingOptions {
    pub all: Option<Expr>,
    pub x: Option<Expr>,
    pub y: Option<Expr>,
    pub top: Option<Expr>,
    pub right: Option<Expr>,
    pub bottom: Option<Expr>,
    pub left: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexAlignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug)]
pub enum GridSizing {
    AspectRatio { width: Expr, height: Expr },
    EvenlyDistribute(LengthValue),
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
