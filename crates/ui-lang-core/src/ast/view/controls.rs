use super::*;

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
