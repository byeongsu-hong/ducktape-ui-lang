use super::*;

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
