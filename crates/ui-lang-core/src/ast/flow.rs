use super::*;

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
        pane: String,
    },
    Restore,
    Maximized,
    Adjacent {
        pane: String,
        edge: PaneEdge,
    },
    Swap {
        first: String,
        second: String,
    },
    Close {
        pane: String,
    },
    Move {
        pane: String,
        edge: PaneEdge,
    },
    Resize {
        ratio: Expr,
    },
    Drop {
        pane: String,
        target: String,
        edge: Option<PaneEdge>,
    },
    Split {
        target: String,
        pane: String,
        axis: PaneAxis,
        ratio: Expr,
    },
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
