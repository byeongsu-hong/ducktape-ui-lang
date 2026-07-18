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

#[derive(Clone, Debug)]
pub struct Document {
    pub app: String,
    pub settings: AppSettings,
    pub presets: Vec<Preset>,
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

#[derive(Clone, Debug)]
pub struct Preset {
    pub name: String,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct AppSettings {
    pub title: Option<AppExpression>,
    pub theme: Option<AppExpression>,
    pub background: Option<AppExpression>,
    pub text_color: Option<AppExpression>,
    pub id: Option<String>,
    pub executor: Option<String>,
    pub fonts: Vec<FontAsset>,
    pub default_text_size: Option<f64>,
    pub antialiasing: Option<bool>,
    pub vsync: Option<bool>,
    pub scale_factor: Option<AppExpression>,
    pub window: Option<WindowSettings>,
    pub windows: Vec<NamedWindow>,
}

#[derive(Clone, Debug)]
pub struct AppExpression {
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct NamedWindow {
    pub name: String,
    pub settings: WindowSettings,
}

#[derive(Clone, Debug)]
pub struct FontAsset {
    pub path: String,
    pub span: Span,
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
    pub icon: Option<WindowIcon>,
    pub exit_on_close_request: Option<bool>,
    pub linux: Option<LinuxWindowSettings>,
    pub windows: Option<WindowsWindowSettings>,
    pub macos: Option<MacosWindowSettings>,
    pub wasm: Option<WasmWindowSettings>,
}

#[derive(Clone, Debug, Default)]
pub struct LinuxWindowSettings {
    pub application_id: Option<String>,
    pub override_redirect: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct WindowsWindowSettings {
    pub drag_and_drop: Option<bool>,
    pub skip_taskbar: Option<bool>,
    pub undecorated_shadow: Option<bool>,
    pub corner: Option<WindowCorner>,
}

#[derive(Clone, Debug, Default)]
pub struct MacosWindowSettings {
    pub title_hidden: Option<bool>,
    pub titlebar_transparent: Option<bool>,
    pub fullsize_content_view: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct WasmWindowSettings {
    pub target: Option<Option<String>>,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowCorner {
    Default,
    DoNotRound,
    Round,
    RoundSmall,
}

#[derive(Clone, Debug)]
pub struct WindowIcon {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub byte_len: usize,
    pub span: Span,
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
    pub progress: Option<Type>,
    pub output: Type,
    pub error: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternKind {
    Future,
    Component,
    Shader,
    Task,
    Stream,
    Sip,
    Recipe,
    Selector,
    EventFilter,
    Sync,
    Subscription,
    Theme,
    Themer,
    Window,
    MarkdownViewer,
    EditorBinding,
    EditorHighlighter,
    EditorStyle,
    TextStyle,
    SliderStyle,
    ProgressStyle,
    ButtonStyle,
    CheckboxStyle,
    TogglerStyle,
    RadioStyle,
    ContainerStyle,
    SvgStyle,
    InputStyle,
    ScrollStyle,
    PickListStyle,
    MenuStyle,
    PaneGridStyle,
}

#[derive(Clone, Debug)]
pub struct Subscription {
    pub source: SubscriptionSource,
    pub window_id: bool,
    pub context: Option<Expr>,
    pub filter: Option<String>,
    pub condition: Option<Expr>,
    pub status: Option<EventStatus>,
    pub route: Route,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventStatus {
    Any,
    Captured,
    Ignored,
}

#[derive(Clone, Debug)]
pub enum SubscriptionSource {
    Every { milliseconds: u64 },
    Repeat { function: String, milliseconds: u64 },
    Run { function: String, args: Vec<Expr> },
    Recipe { function: String, args: Vec<Expr> },
    Events { id: Expr, filter: String },
    Event { raw: bool },
    Extern { function: String, args: Vec<Expr> },
    InputMethod(InputMethodEvent),
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    SystemTheme,
    Touch(TouchEvent),
    Window(WindowEvent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMethodEvent {
    Opened,
    Preedit,
    Commit,
    Closed,
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
    MarkdownAppend {
        target: String,
        value: Expr,
        span: Span,
    },
    ComboPush {
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
    Sip {
        function: String,
        args: Vec<Expr>,
        progress: Route,
        success: Route,
        error: Option<Route>,
        span: Span,
    },
    TaskFlow {
        source: TaskSource,
        transforms: Vec<TaskTransform>,
        success: Option<Route>,
        error: Option<Route>,
        units: Option<Route>,
        span: Span,
    },
    TaskGroup {
        kind: TaskGroupKind,
        statements: Vec<Statement>,
        span: Span,
    },
    Abortable {
        handle: String,
        abort_on_drop: bool,
        task: Box<Statement>,
        span: Span,
    },
    Abort {
        handle: String,
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
        target: Option<Expr>,
        route: Option<Route>,
        span: Span,
    },
    PaneOperation {
        grid: String,
        operation: PaneOperation,
        route: Option<Route>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum TaskSource {
    Effect {
        kind: EffectKind,
        function: String,
        args: Vec<Expr>,
        span: Span,
    },
    Done {
        value: Expr,
        span: Span,
    },
    None {
        output: Type,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum TaskTransform {
    Map {
        binding: String,
        value: Expr,
        span: Span,
    },
    Then {
        binding: String,
        source: TaskSource,
        span: Span,
    },
    AndThen {
        binding: String,
        source: TaskSource,
        span: Span,
    },
    MapError {
        binding: String,
        value: Expr,
        span: Span,
    },
    Collect {
        span: Span,
    },
    Discard {
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum PaneOperation {
    Maximize {
        pane: PaneReference,
    },
    Restore,
    Maximized,
    Adjacent {
        pane: PaneReference,
        edge: PaneEdge,
    },
    Swap {
        first: PaneReference,
        second: PaneReference,
    },
    Close {
        pane: PaneReference,
    },
    Move {
        pane: PaneReference,
        edge: PaneEdge,
    },
    Resize {
        split: Option<String>,
        ratio: Expr,
    },
    Drop {
        pane: PaneReference,
        target: PaneReference,
        edge: Option<PaneEdge>,
    },
    Split {
        target: PaneReference,
        pane: PaneReference,
        axis: PaneAxis,
        ratio: Expr,
    },
}

#[derive(Clone, Debug)]
pub enum PaneReference {
    Static(String),
    Dynamic { template: String, key: Expr },
}

#[derive(Clone, Copy, Debug)]
pub enum PaneEdge {
    Top,
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Debug)]
pub enum WidgetOperation {
    FocusPrevious,
    FocusNext,
    Focus {
        target: WidgetTarget,
    },
    Focused {
        target: WidgetTarget,
    },
    CursorFront {
        target: WidgetTarget,
    },
    CursorEnd {
        target: WidgetTarget,
    },
    Cursor {
        target: WidgetTarget,
        position: Expr,
    },
    SelectAll {
        target: WidgetTarget,
    },
    Select {
        target: WidgetTarget,
        start: Expr,
        end: Expr,
    },
    Snap {
        target: WidgetTarget,
        x: Expr,
        y: Expr,
    },
    SnapEnd {
        target: WidgetTarget,
    },
    ScrollTo {
        target: WidgetTarget,
        x: Expr,
        y: Expr,
    },
    ScrollBy {
        target: WidgetTarget,
        x: Expr,
        y: Expr,
    },
    Find {
        selector: WidgetSelector,
        all: bool,
    },
}

#[derive(Clone, Debug)]
pub struct WidgetTarget {
    pub segments: Vec<Id>,
}

#[derive(Clone, Debug)]
pub enum WidgetSelector {
    Id(WidgetTarget),
    Text(Expr),
    Point { x: Expr, y: Expr },
    Focused,
    Extern { function: String, args: Vec<Expr> },
}

#[derive(Clone, Debug)]
pub enum WindowOperation {
    Open(Option<String>),
    Oldest,
    Latest,
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
    RawId,
    Screenshot,
    MousePassthrough(Expr),
    MonitorSize,
    AutomaticTabbing(Expr),
    Icon {
        pixels: Expr,
        width: Expr,
        height: Expr,
    },
    Callback {
        function: String,
        args: Vec<Expr>,
    },
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
    Stream,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskGroupKind {
    Parallel,
    Sequential,
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
pub struct ComponentSlot {
    pub name: String,
    pub content: Box<ViewNode>,
    pub span: Span,
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
    Container {
        options: Box<ContainerOptions>,
        id: Option<Id>,
        styles: Vec<String>,
        content: Box<ViewNode>,
        span: Span,
    },
    Overlay {
        options: OverlayOptions,
        content: Box<ViewNode>,
        layer: Box<ViewNode>,
        span: Span,
    },
    PaneGrid {
        name: String,
        configuration: PaneConfiguration,
        options: PaneGridOptions,
        panes: Vec<PaneView>,
        templates: Vec<PaneTemplate>,
        span: Span,
    },
    Text {
        value: Expr,
        options: TextOptions,
        styles: Vec<String>,
        span: Span,
    },
    RichText {
        options: TextOptions,
        color: Option<String>,
        spans: Vec<RichSpan>,
        styles: Vec<String>,
        route: Option<Route>,
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
        style: Box<CheckboxStyleSet>,
        styles: Vec<String>,
        route: Route,
        span: Span,
    },
    Toggler {
        label: Expr,
        checked: Expr,
        disabled: Option<Expr>,
        options: BoolControlOptions,
        style: Box<TogglerStyleSet>,
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
        options: BoolControlOptions,
        style: Box<RadioStyleSet>,
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
        options: Box<MarkdownOptions>,
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
        slots: Vec<ComponentSlot>,
        span: Span,
    },
    Slot {
        name: String,
        span: Span,
    },
    ExternComponent {
        function: String,
        args: Vec<Expr>,
        route: Option<Route>,
        span: Span,
    },
    Themer {
        function: String,
        args: Vec<Expr>,
        route: Option<Route>,
        span: Span,
    },
    Shader {
        function: String,
        args: Vec<Expr>,
        width: Option<LengthValue>,
        height: Option<LengthValue>,
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
    Canvas {
        options: Box<CanvasOptions>,
        locals: Vec<State>,
        commands: Vec<CanvasCommand>,
        events: Vec<CanvasEvent>,
        span: Span,
    },
    Theme {
        preset: ThemePreset,
        text: Option<String>,
        background: Option<BackgroundValue>,
        content: Box<ViewNode>,
        span: Span,
    },
    Float {
        scale: Expr,
        x: Expr,
        y: Expr,
        style: FloatStyleOptions,
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
pub struct FloatStyleOptions {
    pub shadow_color: Option<String>,
    pub shadow_x: Option<Expr>,
    pub shadow_y: Option<Expr>,
    pub shadow_blur: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
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
    pub viewer: Option<ExternCall>,
    pub style: MarkdownStyleOptions,
}

#[derive(Clone, Debug)]
pub struct ExternCall {
    pub function: String,
    pub args: Vec<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct MarkdownStyleOptions {
    pub font: Option<FontPreset>,
    pub inline_code_background: Option<BackgroundValue>,
    pub inline_code_color: Option<String>,
    pub inline_code_font: Option<FontPreset>,
    pub code_block_font: Option<FontPreset>,
    pub link_color: Option<String>,
    pub inline_code_padding: PaddingOptions,
    pub inline_code_border_color: Option<String>,
    pub inline_code_border_width: Option<Expr>,
    pub inline_code_radius: Option<Expr>,
    pub inline_code_radius_top_left: Option<Expr>,
    pub inline_code_radius_top_right: Option<Expr>,
    pub inline_code_radius_bottom_right: Option<Expr>,
    pub inline_code_radius_bottom_left: Option<Expr>,
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
    pub highlighter: Option<ExternCall>,
    pub key_binding: Option<ExternCall>,
    pub key_binding_route: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
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

#[derive(Clone, Debug)]
pub enum ThemePreset {
    Default,
    App,
    BuiltIn(String),
    Factory(ExternCall),
}

pub(crate) const BUILT_IN_THEMES: &[&str] = &[
    "light",
    "dark",
    "dracula",
    "nord",
    "solarized-light",
    "solarized-dark",
    "gruvbox-light",
    "gruvbox-dark",
    "catppuccin-latte",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "catppuccin-mocha",
    "tokyo-night",
    "tokyo-night-storm",
    "tokyo-night-light",
    "kanagawa-wave",
    "kanagawa-dragon",
    "kanagawa-lotus",
    "moonfly",
    "nightfly",
    "oxocarbon",
    "ferra",
];

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
    pub icon: Option<TextInputIcon>,
    pub custom_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
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
    pub custom_style: Option<ExternCall>,
}

#[derive(Clone, Debug)]
pub enum TextLineHeight {
    Relative(Expr),
    Absolute(Expr),
}

#[derive(Clone, Debug)]
pub struct RichSpan {
    pub value: Expr,
    pub options: RichSpanOptions,
    pub styles: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct RichSpanOptions {
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub font: Option<FontPreset>,
    pub color: Option<String>,
    pub link: Option<Expr>,
    pub background: Option<BackgroundValue>,
    pub border: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub padding: PaddingOptions,
    pub underline: Option<Expr>,
    pub strikethrough: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub clip: Option<Expr>,
    pub style: Box<ButtonStyleSet>,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonStyleSet {
    pub preset: ButtonStylePreset,
    pub custom: Option<ExternCall>,
    pub active: Option<ButtonStatusStyle>,
    pub hovered: Option<ButtonStatusStyle>,
    pub pressed: Option<ButtonStatusStyle>,
    pub disabled: Option<ButtonStatusStyle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStylePreset {
    #[default]
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Text,
    Background,
    Subtle,
}

#[derive(Clone, Debug)]
pub struct ButtonStatusStyle {
    pub options: ContainerStyleOptions,
    pub span: Span,
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

#[derive(Clone, Debug, Default)]
pub struct CheckboxStyleSet {
    pub preset: CheckboxStylePreset,
    pub custom: Option<ExternCall>,
    pub active_checked: Option<CheckboxStatusStyle>,
    pub active_unchecked: Option<CheckboxStatusStyle>,
    pub hovered_checked: Option<CheckboxStatusStyle>,
    pub hovered_unchecked: Option<CheckboxStatusStyle>,
    pub disabled_checked: Option<CheckboxStatusStyle>,
    pub disabled_unchecked: Option<CheckboxStatusStyle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckboxStylePreset {
    #[default]
    Primary,
    Secondary,
    Success,
    Danger,
}

#[derive(Clone, Debug, Default)]
pub struct CheckboxStatusStyle {
    pub background: Option<BackgroundValue>,
    pub icon_color: Option<String>,
    pub text_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct TogglerStyleSet {
    pub custom: Option<ExternCall>,
    pub active_checked: Option<TogglerStatusStyle>,
    pub active_unchecked: Option<TogglerStatusStyle>,
    pub hovered_checked: Option<TogglerStatusStyle>,
    pub hovered_unchecked: Option<TogglerStatusStyle>,
    pub disabled_checked: Option<TogglerStatusStyle>,
    pub disabled_unchecked: Option<TogglerStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct TogglerStatusStyle {
    pub background: Option<BackgroundValue>,
    pub background_border_color: Option<String>,
    pub background_border_width: Option<Expr>,
    pub foreground: Option<BackgroundValue>,
    pub foreground_border_color: Option<String>,
    pub foreground_border_width: Option<Expr>,
    pub text_color: Option<String>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub padding_ratio: Option<Expr>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct RadioStyleSet {
    pub custom: Option<ExternCall>,
    pub active_selected: Option<RadioStatusStyle>,
    pub active_unselected: Option<RadioStatusStyle>,
    pub hovered_selected: Option<RadioStatusStyle>,
    pub hovered_unselected: Option<RadioStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct RadioStatusStyle {
    pub background: Option<BackgroundValue>,
    pub dot_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub text_color: Option<String>,
    pub span: Option<Span>,
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
    pub custom: Option<ExternCall>,
    pub active: Option<SliderStyle>,
    pub hovered: Option<SliderStyle>,
    pub dragged: Option<SliderStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct SliderStyle {
    pub span: Option<Span>,
    pub rail_start: Option<BackgroundValue>,
    pub rail_end: Option<BackgroundValue>,
    pub rail_width: Option<Expr>,
    pub rail_border_color: Option<String>,
    pub rail_border_width: Option<Expr>,
    pub rail_radius: Option<Expr>,
    pub rail_radius_top_left: Option<Expr>,
    pub rail_radius_top_right: Option<Expr>,
    pub rail_radius_bottom_right: Option<Expr>,
    pub rail_radius_bottom_left: Option<Expr>,
    pub handle_shape: Option<SliderHandleShape>,
    pub handle_color: Option<BackgroundValue>,
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
    pub custom_style: Option<ExternCall>,
    pub background: Option<BackgroundValue>,
    pub bar: Option<BackgroundValue>,
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
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub font: Option<FontPreset>,
    pub handle: Option<PickListHandle>,
    pub open: Option<Route>,
    pub close: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub custom_menu_style: Option<ExternCall>,
    pub style: Box<PickListStyleSet>,
    pub menu_style: Option<Box<MenuStyleOptions>>,
}

#[derive(Clone, Debug, Default)]
pub struct PickListStyleSet {
    pub active: Option<PickListStatusStyle>,
    pub hovered: Option<PickListStatusStyle>,
    pub opened: Option<PickListStatusStyle>,
    pub opened_hovered: Option<PickListStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct PickListStatusStyle {
    pub options: ContainerStyleOptions,
    pub placeholder_color: Option<String>,
    pub handle_color: Option<String>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct MenuStyleOptions {
    pub options: ContainerStyleOptions,
    pub selected_text_color: Option<String>,
    pub selected_background: Option<BackgroundValue>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
pub enum PickListHandle {
    Arrow {
        size: Option<Expr>,
    },
    Static(PickListIcon),
    Dynamic {
        closed: PickListIcon,
        open: PickListIcon,
    },
    None,
}

#[derive(Clone, Debug)]
pub struct PickListIcon {
    pub code_point: char,
    pub font: Option<FontPreset>,
    pub size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ComboBoxOptions {
    pub width: Option<LengthValue>,
    pub menu_height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub font: Option<FontPreset>,
    pub icon: Option<TextInputIcon>,
    pub input: Option<Route>,
    pub hover: Option<Route>,
    pub open: Option<Route>,
    pub close: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub custom_menu_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
    pub menu_style: Option<Box<MenuStyleOptions>>,
}

#[derive(Clone, Debug, Default)]
pub struct TextInputStyleSet {
    pub active: Option<TextInputStatusStyle>,
    pub hovered: Option<TextInputStatusStyle>,
    pub focused: Option<TextInputStatusStyle>,
    pub focused_hovered: Option<TextInputStatusStyle>,
    pub disabled: Option<TextInputStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct TextInputStatusStyle {
    pub options: ContainerStyleOptions,
    pub icon_color: Option<String>,
    pub placeholder_color: Option<String>,
    pub value_color: Option<String>,
    pub selection_color: Option<String>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct TextInputIcon {
    pub code_point: char,
    pub font: Option<FontPreset>,
    pub size: Option<Expr>,
    pub spacing: Option<Expr>,
    pub side: IconSide,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Svg,
    Viewer,
}

#[derive(Clone, Debug, Default)]
pub struct MediaOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub fit: Option<ContentFit>,
    pub rotation: Option<Expr>,
    pub rotation_solid: bool,
    pub opacity: Option<Expr>,
    pub svg_memory: bool,
    pub svg_color: Option<String>,
    pub svg_hover_color: Option<Option<String>>,
    pub svg_style: Option<ExternCall>,
    pub filter: Option<ImageFilter>,
    pub scale: Option<Expr>,
    pub expand: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub crop: Option<[Expr; 4]>,
    pub padding: Option<Expr>,
    pub min_scale: Option<Expr>,
    pub max_scale: Option<Expr>,
    pub scale_step: Option<Expr>,
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
    pub custom_style: Option<ExternCall>,
    pub background: Option<BackgroundValue>,
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
pub struct CanvasOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub cache: Option<Expr>,
    pub cache_group: Option<String>,
    pub capture: Option<Expr>,
    pub press: Option<Route>,
    pub release: Option<Route>,
    pub right_press: Option<Route>,
    pub right_release: Option<Route>,
    pub middle_press: Option<Route>,
    pub middle_release: Option<Route>,
    pub enter: Option<Route>,
    pub move_route: Option<Route>,
    pub scroll: Option<Route>,
    pub exit: Option<Route>,
    pub interaction: Option<MouseInteraction>,
    pub interaction_expr: Option<Expr>,
    pub interaction_outside: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct CanvasEvent {
    pub source: SubscriptionSource,
    pub bindings: Vec<String>,
    pub updates: Vec<CanvasStateUpdate>,
    pub action: Option<CanvasEventAction>,
    pub capture: bool,
    pub route_payload: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum CanvasEventAction {
    Route(Route),
    Redraw { after_ms: Option<u64> },
}

#[derive(Clone, Debug)]
pub struct CanvasStateUpdate {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum CanvasCommand {
    Rectangle {
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        radius: Box<CanvasRadius>,
        paint: Box<CanvasPaint>,
        span: Span,
    },
    Circle {
        x: Expr,
        y: Expr,
        radius: Expr,
        paint: Box<CanvasPaint>,
        span: Span,
    },
    Line {
        x1: Expr,
        y1: Expr,
        x2: Expr,
        y2: Expr,
        stroke: Box<CanvasStroke>,
        span: Span,
    },
    Text {
        value: Expr,
        x: Expr,
        y: Expr,
        max_width: Option<Expr>,
        color: Option<String>,
        size: Option<Expr>,
        line_height: Option<TextLineHeight>,
        font: Option<FontPreset>,
        align_x: Option<TextAlignment>,
        align_y: Option<VerticalAlignment>,
        shaping: Option<TextShaping>,
        span: Span,
    },
    Image {
        source: Expr,
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        filter: ImageFilter,
        rotation: Expr,
        opacity: Expr,
        snap: Expr,
        radius: Box<CanvasRadius>,
        span: Span,
    },
    Svg {
        source: Expr,
        memory: bool,
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        color: Option<String>,
        rotation: Expr,
        opacity: Expr,
        span: Span,
    },
    Path {
        segments: Vec<CanvasPathSegment>,
        paint: Box<CanvasPaint>,
        span: Span,
    },
    Group {
        transform: Box<CanvasTransform>,
        commands: Vec<CanvasCommand>,
        span: Span,
    },
    If {
        condition: Expr,
        commands: Vec<CanvasCommand>,
        span: Span,
    },
    For {
        item: String,
        items: Expr,
        commands: Vec<CanvasCommand>,
        span: Span,
    },
}

#[derive(Clone, Debug, Default)]
pub struct CanvasPaint {
    pub fill: Option<BackgroundValue>,
    pub fill_rule: CanvasFillRule,
    pub stroke: Option<CanvasStroke>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasFillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Clone, Debug)]
pub struct CanvasStroke {
    pub style: BackgroundValue,
    pub width: Expr,
    pub cap: CanvasLineCap,
    pub join: CanvasLineJoin,
    pub dash: Vec<Expr>,
    pub dash_offset: Expr,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasLineCap {
    #[default]
    Butt,
    Square,
    Round,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasLineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Debug, Default)]
pub struct CanvasRadius {
    pub all: Option<Expr>,
    pub top_left: Option<Expr>,
    pub top_right: Option<Expr>,
    pub bottom_right: Option<Expr>,
    pub bottom_left: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct CanvasTransform {
    pub x: Option<Expr>,
    pub y: Option<Expr>,
    pub rotate: Option<Expr>,
    pub scale: Option<Expr>,
    pub scale_x: Option<Expr>,
    pub scale_y: Option<Expr>,
    pub clip: Option<[Expr; 4]>,
}

#[derive(Clone, Debug)]
pub enum CanvasPathSegment {
    Move(Expr, Expr),
    Line(Expr, Expr),
    Arc {
        x: Expr,
        y: Expr,
        radius: Expr,
        start: Expr,
        end: Expr,
    },
    ArcTo {
        ax: Expr,
        ay: Expr,
        bx: Expr,
        by: Expr,
        radius: Expr,
    },
    Ellipse {
        x: Expr,
        y: Expr,
        radius_x: Expr,
        radius_y: Expr,
        rotation: Expr,
        start: Expr,
        end: Expr,
    },
    Bezier {
        control_ax: Expr,
        control_ay: Expr,
        control_bx: Expr,
        control_by: Expr,
        x: Expr,
        y: Expr,
    },
    Quadratic {
        control_x: Expr,
        control_y: Expr,
        x: Expr,
        y: Expr,
    },
    Rectangle {
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
    },
    RoundedRectangle {
        x: Expr,
        y: Expr,
        width: Expr,
        height: Expr,
        radius: CanvasRadius,
    },
    Circle {
        x: Expr,
        y: Expr,
        radius: Expr,
    },
    Close,
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
pub struct ContainerOptions {
    pub padding: PaddingOptions,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub max_width: Option<Expr>,
    pub max_height: Option<Expr>,
    pub align_x: Option<FlexAlignment>,
    pub align_y: Option<FlexAlignment>,
    pub clip: Option<Expr>,
    pub custom_style: Option<ExternCall>,
    pub style: ContainerStyleOptions,
}

#[derive(Clone, Debug)]
pub struct OverlayOptions {
    pub visible: Expr,
    pub dismiss: Option<Route>,
    pub backdrop: String,
    pub padding: Expr,
    pub align_x: FlexAlignment,
    pub align_y: FlexAlignment,
}

#[derive(Clone, Copy, Debug)]
pub enum PaneAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub enum PaneConfiguration {
    Pane(String),
    Split {
        name: Option<String>,
        axis: PaneAxis,
        ratio: f32,
        a: Box<PaneConfiguration>,
        b: Box<PaneConfiguration>,
    },
}

#[derive(Clone, Debug)]
pub struct PaneView {
    pub name: String,
    pub maximized: Option<String>,
    pub content: Box<ViewNode>,
    pub title: Option<PaneTitle>,
    pub styles: Vec<String>,
    pub style: ContainerStyleOptions,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PaneTemplate {
    pub item: String,
    pub items: String,
    pub key: Expr,
    pub pane: PaneView,
    pub span: Span,
}

impl PaneView {
    pub fn nodes(&self) -> impl Iterator<Item = &ViewNode> {
        [
            Some(self.content.as_ref()),
            self.title.as_ref().map(|title| title.content.as_ref()),
            self.title
                .as_ref()
                .and_then(|title| title.controls.as_deref()),
            self.title
                .as_ref()
                .and_then(|title| title.compact_controls.as_deref()),
        ]
        .into_iter()
        .flatten()
    }
}

#[derive(Clone, Debug)]
pub struct PaneTitle {
    pub content: Box<ViewNode>,
    pub controls: Option<Box<ViewNode>>,
    pub compact_controls: Option<Box<ViewNode>>,
    pub padding: PaddingOptions,
    pub always_show_controls: bool,
    pub styles: Vec<String>,
    pub style: ContainerStyleOptions,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct PaneGridOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub spacing: Option<Expr>,
    pub min_size: Option<Expr>,
    pub resize_leeway: Option<Expr>,
    pub draggable: bool,
    pub click: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub style: PaneGridStyle,
}

#[derive(Clone, Debug, Default)]
pub struct PaneGridStyle {
    pub region_background: Option<BackgroundValue>,
    pub region_border: Option<String>,
    pub region_border_width: Option<Expr>,
    pub region_radius: Option<Expr>,
    pub region_radius_top_left: Option<Expr>,
    pub region_radius_top_right: Option<Expr>,
    pub region_radius_bottom_right: Option<Expr>,
    pub region_radius_bottom_left: Option<Expr>,
    pub hovered_split: Option<String>,
    pub hovered_split_width: Option<Expr>,
    pub picked_split: Option<String>,
    pub picked_split_width: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum BackgroundValue {
    Color(String),
    Linear {
        angle: Expr,
        stops: Vec<GradientStop>,
    },
}

#[derive(Clone, Debug)]
pub struct GradientStop {
    pub color: String,
    pub offset: Expr,
}

#[derive(Clone, Debug, Default)]
pub struct ContainerStyleOptions {
    pub background: Option<BackgroundValue>,
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
    pub viewport_route: Option<Route>,
    pub custom_style: Option<ExternCall>,
    pub styles: Vec<ScrollStatusStyle>,
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
            viewport_route: None,
            custom_style: None,
            styles: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollStatus {
    Active,
    Hovered,
    Dragged,
}

#[derive(Clone, Debug)]
pub struct ScrollStatusStyle {
    pub status: ScrollStatus,
    pub horizontal_interaction: Option<bool>,
    pub vertical_interaction: Option<bool>,
    pub horizontal_disabled: Option<bool>,
    pub vertical_disabled: Option<bool>,
    pub container: ContainerStyleOptions,
    pub horizontal_rail: ScrollRailStyle,
    pub vertical_rail: ScrollRailStyle,
    pub gap: Option<BackgroundValue>,
    pub auto_scroll: ContainerStyleOptions,
    pub auto_scroll_icon: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ScrollRailStyle {
    pub rail: ContainerStyleOptions,
    pub scroller: ContainerStyleOptions,
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
    Bytes(Vec<u8>),
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
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}
