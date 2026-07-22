use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Padding, Point, Rectangle, Size, Vector};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    fn axis(self) -> Axis {
        match self {
            Self::Row | Self::RowReverse => Axis::Horizontal,
            Self::Column | Self::ColumnReverse => Axis::Vertical,
        }
    }

    fn is_reverse(self) -> bool {
        matches!(self, Self::RowReverse | Self::ColumnReverse)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    End,
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignContent {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    #[default]
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexBasis {
    #[default]
    Auto,
    Content,
    Fixed(f32),
    Percent(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexMargin {
    #[default]
    Zero,
    Fixed(f32),
    Percent(f32),
    Auto,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlexMargins {
    pub top: FlexMargin,
    pub right: FlexMargin,
    pub bottom: FlexMargin,
    pub left: FlexMargin,
}

pub struct FlexItem<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    order: i32,
    grow: Option<f32>,
    shrink: f32,
    basis: FlexBasis,
    align_self: Option<AlignItems>,
    margins: FlexMargins,
}

pub fn flex_item<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> FlexItem<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    FlexItem {
        content: content.into(),
        order: 0,
        grow: None,
        shrink: 1.0,
        basis: FlexBasis::Auto,
        align_self: None,
        margins: FlexMargins::default(),
    }
}

impl<Message, Theme, Renderer> FlexItem<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn order(mut self, order: i64) -> Self {
        self.order = order.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        self
    }

    pub fn grow(mut self, grow: f32) -> Self {
        self.grow = Some(non_negative(grow));
        self
    }

    pub fn shrink(mut self, shrink: f32) -> Self {
        self.shrink = non_negative(shrink);
        self
    }

    pub fn basis(mut self, basis: FlexBasis) -> Self {
        self.basis = basis;
        self
    }

    pub fn align_self(mut self, align: AlignItems) -> Self {
        self.align_self = Some(align);
        self
    }

    pub fn margins(mut self, margins: FlexMargins) -> Self {
        self.margins = margins;
        self
    }
}

pub struct Flex<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    direction: FlexDirection,
    wrap: FlexWrap,
    justify_content: JustifyContent,
    align_items: AlignItems,
    align_content: AlignContent,
    row_gap: f32,
    column_gap: f32,
    padding: Padding,
    width: Length,
    height: Length,
    max_width: f32,
    max_height: f32,
    clip: bool,
    items: Vec<FlexItem<'a, Message, Theme, Renderer>>,
}

pub fn flex<'a, Message, Theme, Renderer>(
    items: Vec<FlexItem<'a, Message, Theme, Renderer>>,
) -> Flex<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    Flex {
        direction: FlexDirection::Row,
        wrap: FlexWrap::NoWrap,
        justify_content: JustifyContent::FlexStart,
        align_items: AlignItems::Stretch,
        align_content: AlignContent::Stretch,
        row_gap: 0.0,
        column_gap: 0.0,
        padding: Padding::ZERO,
        width: Length::Shrink,
        height: Length::Shrink,
        max_width: f32::INFINITY,
        max_height: f32::INFINITY,
        clip: false,
        items,
    }
}

impl<Message, Theme, Renderer> Flex<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn wrap(mut self, wrap: FlexWrap) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    pub fn align_content(mut self, align: AlignContent) -> Self {
        self.align_content = align;
        self
    }

    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = non_negative(gap);
        self
    }

    pub fn column_gap(mut self, gap: f32) -> Self {
        self.column_gap = non_negative(gap);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        let gap = non_negative(gap);
        self.row_gap = gap;
        self.column_gap = gap;
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = non_negative(width);
        self
    }

    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = non_negative(height);
        self
    }

    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }
}

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    fn main(self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.width,
            Self::Vertical => size.height,
        }
    }

    fn cross(self, size: Size) -> f32 {
        match self {
            Self::Horizontal => size.height,
            Self::Vertical => size.width,
        }
    }

    fn lengths(self, size: Size<Length>) -> (Length, Length) {
        match self {
            Self::Horizontal => (size.width, size.height),
            Self::Vertical => (size.height, size.width),
        }
    }

    fn size(self, main: f32, cross: f32) -> Size {
        match self {
            Self::Horizontal => Size::new(main, cross),
            Self::Vertical => Size::new(cross, main),
        }
    }

    fn point(self, main: f32, cross: f32) -> Point {
        match self {
            Self::Horizontal => Point::new(main, cross),
            Self::Vertical => Point::new(cross, main),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ResolvedMargins {
    main_start: Option<f32>,
    main_end: Option<f32>,
    cross_start: Option<f32>,
    cross_end: Option<f32>,
}

impl ResolvedMargins {
    fn fixed_main(self) -> f32 {
        self.main_start.unwrap_or(0.0) + self.main_end.unwrap_or(0.0)
    }

    fn fixed_cross(self) -> f32 {
        self.cross_start.unwrap_or(0.0) + self.cross_end.unwrap_or(0.0)
    }

    fn main_auto_count(self) -> usize {
        usize::from(self.main_start.is_none()) + usize::from(self.main_end.is_none())
    }
}

struct ItemLayout {
    source: usize,
    base_main: f32,
    target_main: f32,
    natural_cross: f32,
    grow: f32,
    shrink: f32,
    margins: ResolvedMargins,
}

struct Line {
    start: usize,
    end: usize,
    cross: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Flex<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.items
            .iter()
            .map(|item| Tree::new(&item.content))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let children = self
            .items
            .iter()
            .map(|item| &item.content)
            .collect::<Vec<_>>();
        tree.diff_children(&children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let axis = self.direction.axis();
        let limits = limits
            .width(self.width)
            .height(self.height)
            .max_width(self.max_width)
            .max_height(self.max_height);
        let padding_size = Size::new(
            self.padding.left + self.padding.right,
            self.padding.top + self.padding.bottom,
        );
        let inner_limits = limits.shrink(padding_size);
        let max_main = axis.main(inner_limits.max());
        let max_cross = axis.cross(inner_limits.max());
        let main_gap = match axis {
            Axis::Horizontal => self.column_gap,
            Axis::Vertical => self.row_gap,
        };
        let cross_gap = match axis {
            Axis::Horizontal => self.row_gap,
            Axis::Vertical => self.column_gap,
        };
        let (main_length, _) = axis.lengths(Size::new(self.width, self.height));
        let definite_main = definite_length(main_length, max_main);
        let basis_available = definite_main.unwrap_or(f32::INFINITY);

        let mut order = (0..self.items.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| self.items[*index].order);
        let mut measured = Vec::with_capacity(order.len());

        for source in order {
            let item = &mut self.items[source];
            let hint = item.content.as_widget().size();
            let (main_hint, _) = axis.lengths(hint);
            let basis = resolve_basis(item.basis, basis_available, main_hint);
            let measure_limits = child_limits(axis, basis, max_main, max_cross, true, None);
            let node = item.content.as_widget_mut().layout(
                &mut tree.children[source],
                renderer,
                &measure_limits,
            );
            let intrinsic_main = axis.main(node.size());
            let fill = main_hint.fill_factor();
            let grow = item.grow.unwrap_or(fill as f32);
            let content_basis = matches!(item.basis, FlexBasis::Content | FlexBasis::Percent(_));
            let base_main = basis.unwrap_or(if content_basis || fill == 0 {
                intrinsic_main
            } else {
                0.0
            });
            measured.push(ItemLayout {
                source,
                base_main: non_negative(base_main),
                target_main: non_negative(base_main),
                natural_cross: axis.cross(node.size()),
                grow: non_negative(grow),
                shrink: item.shrink,
                margins: resolve_margins(item.margins, axis, inner_limits.max().width),
            });
        }

        let wrap_limit = definite_main.unwrap_or(max_main);
        let mut lines = build_lines(&measured, self.wrap, wrap_limit, main_gap);
        let natural_main = lines
            .iter()
            .map(|line| line_base(&measured[line.start..line.end], main_gap))
            .fold(0.0_f32, f32::max);
        let initial_cross = lines.iter().map(|line| line.cross).sum::<f32>()
            + cross_gap * lines.len().saturating_sub(1) as f32;
        let intrinsic = axis.size(natural_main, initial_cross) + padding_size;
        let mut outer_size = limits.resolve(self.width, self.height, intrinsic);
        let target_main = (axis.main(outer_size) - axis.main(padding_size)).max(0.0);

        let mut nodes = (0..self.items.len())
            .map(|_| layout::Node::default())
            .collect::<Vec<_>>();

        for line in &mut lines {
            resolve_flex_line(&mut measured[line.start..line.end], target_main, main_gap);
            line.cross = 0.0;
            for item in &mut measured[line.start..line.end] {
                let source = item.source;
                let child = &mut self.items[source].content;
                let final_limits = child_limits(
                    axis,
                    Some(item.target_main),
                    item.target_main,
                    max_cross,
                    false,
                    None,
                );
                let node = child.as_widget_mut().layout(
                    &mut tree.children[source],
                    renderer,
                    &final_limits,
                );
                item.natural_cross = axis.cross(node.size());
                line.cross = line
                    .cross
                    .max(item.natural_cross + item.margins.fixed_cross());
                nodes[source] = node;
            }
        }

        let natural_cross = lines.iter().map(|line| line.cross).sum::<f32>()
            + cross_gap * lines.len().saturating_sub(1) as f32;
        let intrinsic = axis.size(target_main, natural_cross) + padding_size;
        outer_size = limits.resolve(self.width, self.height, intrinsic);
        let target_cross = (axis.cross(outer_size) - axis.cross(padding_size)).max(0.0);

        let (line_leading, line_between) = align_lines(
            &mut lines,
            target_cross,
            cross_gap,
            self.align_content,
            self.wrap,
        );
        let mut line_cursor = line_leading;
        for line in &lines {
            let line_cross = line.cross;
            let physical_line_cross = if self.wrap == FlexWrap::WrapReverse {
                target_cross - line_cursor - line_cross
            } else {
                line_cursor
            };
            let line_items = &mut measured[line.start..line.end];
            let used_main = line_items
                .iter()
                .map(|item| item.target_main + item.margins.fixed_main())
                .sum::<f32>()
                + main_gap * line_items.len().saturating_sub(1) as f32;
            let remaining = target_main - used_main;
            let auto_margins = line_items
                .iter()
                .map(|item| item.margins.main_auto_count())
                .sum::<usize>();
            let auto_share = if auto_margins > 0 {
                remaining.max(0.0) / auto_margins as f32
            } else {
                0.0
            };
            let (leading, between) = if auto_margins > 0 {
                (0.0, 0.0)
            } else {
                justify_line(
                    self.justify_content,
                    remaining,
                    line_items.len(),
                    self.direction.is_reverse(),
                )
            };
            let mut main_cursor = leading;

            for item in line_items {
                let source = item.source;
                let main_start = item.margins.main_start.unwrap_or(auto_share);
                let main_end = item.margins.main_end.unwrap_or(auto_share);
                main_cursor += main_start;
                let physical_main = if self.direction.is_reverse() {
                    target_main - main_cursor - item.target_main
                } else {
                    main_cursor
                };

                let align = self.items[source].align_self.unwrap_or(self.align_items);
                let cross_start = item.margins.cross_start;
                let cross_end = item.margins.cross_end;
                let cross_auto_count =
                    usize::from(cross_start.is_none()) + usize::from(cross_end.is_none());
                let mut node_cross = axis.cross(nodes[source].size());
                let hint = self.items[source].content.as_widget().size();
                let (_, cross_hint) = axis.lengths(hint);
                if align == AlignItems::Stretch
                    && cross_auto_count == 0
                    && !matches!(cross_hint, Length::Fixed(_))
                {
                    let stretched =
                        (line_cross - cross_start.unwrap_or(0.0) - cross_end.unwrap_or(0.0))
                            .max(0.0);
                    let stretch_limits = child_limits(
                        axis,
                        Some(item.target_main),
                        item.target_main,
                        stretched,
                        false,
                        Some(stretched),
                    );
                    nodes[source] = self.items[source].content.as_widget_mut().layout(
                        &mut tree.children[source],
                        renderer,
                        &stretch_limits,
                    );
                    node_cross = axis.cross(nodes[source].size());
                }
                let free_cross =
                    line_cross - node_cross - cross_start.unwrap_or(0.0) - cross_end.unwrap_or(0.0);
                let cross_offset = if cross_auto_count > 0 {
                    cross_start.unwrap_or(free_cross.max(0.0) / cross_auto_count as f32)
                } else {
                    cross_start.unwrap_or(0.0)
                        + align_item(
                            align,
                            free_cross,
                            self.wrap == FlexWrap::WrapReverse,
                            matches!(axis, Axis::Horizontal),
                        )
                };
                let physical_cross = physical_line_cross + cross_offset;
                let padding_main = match axis {
                    Axis::Horizontal => self.padding.left,
                    Axis::Vertical => self.padding.top,
                };
                let padding_cross = match axis {
                    Axis::Horizontal => self.padding.top,
                    Axis::Vertical => self.padding.left,
                };
                nodes[source].move_to_mut(
                    axis.point(physical_main + padding_main, physical_cross + padding_cross),
                );
                main_cursor += item.target_main + main_end + main_gap + between;
            }
            line_cursor += line_cross + cross_gap + line_between;
        }

        layout::Node::with_children(outer_size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.items
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((item, state), layout)| {
                    item.content
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((item, tree), layout) in self
            .items
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            item.content.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.items
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((item, tree), layout)| {
                item.content
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clipped_viewport) = layout.bounds().intersection(viewport) {
            let viewport = if self.clip {
                &clipped_viewport
            } else {
                viewport
            };
            for ((item, tree), layout) in self
                .items
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .filter(|(_, layout)| layout.bounds().intersects(viewport))
            {
                item.content
                    .as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, viewport);
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let children = self
            .items
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((item, state), layout)| {
                item.content
                    .as_widget_mut()
                    .overlay(state, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();
        (!children.is_empty()).then(|| overlay::Group::with_children(children).overlay())
    }
}

impl<'a, Message, Theme, Renderer> From<Flex<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(flex: Flex<'a, Message, Theme, Renderer>) -> Self {
        Self::new(flex)
    }
}

fn non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else if value.is_sign_positive() {
        f32::MAX
    } else {
        0.0
    }
}

fn definite_length(length: Length, maximum: f32) -> Option<f32> {
    match length {
        Length::Fixed(value) => Some(value.min(maximum)),
        Length::Fill | Length::FillPortion(_) if maximum.is_finite() => Some(maximum),
        Length::Fill | Length::FillPortion(_) | Length::Shrink => None,
    }
}

fn resolve_basis(basis: FlexBasis, available: f32, hint: Length) -> Option<f32> {
    match basis {
        FlexBasis::Auto => match hint {
            Length::Fixed(value) => Some(value),
            Length::Fill | Length::FillPortion(_) | Length::Shrink => None,
        },
        FlexBasis::Content => None,
        FlexBasis::Fixed(value) => Some(non_negative(value)),
        FlexBasis::Percent(value) if available.is_finite() => Some(non_negative(value) * available),
        FlexBasis::Percent(_) => None,
    }
}

fn resolve_margin(margin: FlexMargin, percentage_base: f32) -> Option<f32> {
    match margin {
        FlexMargin::Zero => Some(0.0),
        FlexMargin::Fixed(value) => Some(if value.is_finite() { value } else { 0.0 }),
        FlexMargin::Percent(value) if percentage_base.is_finite() => Some(value * percentage_base),
        FlexMargin::Percent(_) => Some(0.0),
        FlexMargin::Auto => None,
    }
}

fn resolve_margins(margins: FlexMargins, axis: Axis, percentage_base: f32) -> ResolvedMargins {
    let top = resolve_margin(margins.top, percentage_base);
    let right = resolve_margin(margins.right, percentage_base);
    let bottom = resolve_margin(margins.bottom, percentage_base);
    let left = resolve_margin(margins.left, percentage_base);
    match axis {
        Axis::Horizontal => ResolvedMargins {
            main_start: left,
            main_end: right,
            cross_start: top,
            cross_end: bottom,
        },
        Axis::Vertical => ResolvedMargins {
            main_start: top,
            main_end: bottom,
            cross_start: left,
            cross_end: right,
        },
    }
}

fn child_limits(
    axis: Axis,
    main: Option<f32>,
    max_main: f32,
    max_cross: f32,
    compress: bool,
    cross: Option<f32>,
) -> layout::Limits {
    let main_min = main.unwrap_or(0.0);
    let main_max = main.unwrap_or(max_main);
    let cross_min = cross.unwrap_or(0.0);
    let cross_max = cross.unwrap_or(max_cross);
    layout::Limits::with_compression(
        axis.size(main_min, cross_min),
        axis.size(main_max, cross_max),
        Size::new(compress, compress),
    )
}

fn build_lines(items: &[ItemLayout], wrap: FlexWrap, limit: f32, gap: f32) -> Vec<Line> {
    if items.is_empty() {
        return Vec::new();
    }
    if wrap == FlexWrap::NoWrap || !limit.is_finite() {
        return vec![Line {
            start: 0,
            end: items.len(),
            cross: items
                .iter()
                .map(|item| item.natural_cross + item.margins.fixed_cross())
                .fold(0.0, f32::max),
        }];
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let mut used = 0.0;
    let mut cross = 0.0_f32;
    for (index, item) in items.iter().enumerate() {
        let outer = item.base_main + item.margins.fixed_main();
        let next = if index == start {
            outer
        } else {
            used + gap + outer
        };
        if index > start && next > limit {
            lines.push(Line {
                start,
                end: index,
                cross,
            });
            start = index;
            used = outer;
            cross = item.natural_cross + item.margins.fixed_cross();
        } else {
            used = next;
            cross = cross.max(item.natural_cross + item.margins.fixed_cross());
        }
    }
    lines.push(Line {
        start,
        end: items.len(),
        cross,
    });
    lines
}

fn line_base(items: &[ItemLayout], gap: f32) -> f32 {
    items
        .iter()
        .map(|item| item.base_main + item.margins.fixed_main())
        .sum::<f32>()
        + gap * items.len().saturating_sub(1) as f32
}

fn resolve_flex_line(items: &mut [ItemLayout], target: f32, gap: f32) {
    for item in items.iter_mut() {
        item.target_main = item.base_main;
    }
    let used = line_base(items, gap);
    let free = target - used;
    if free > 0.0 {
        if items.iter().any(|item| item.margins.main_auto_count() > 0) {
            return;
        }
        let grow = items.iter().map(|item| item.grow).sum::<f32>();
        if grow > 0.0 {
            let distributable = free * grow.min(1.0);
            for item in items {
                item.target_main += distributable * item.grow / grow;
            }
        }
    } else if free < 0.0 {
        let shrink = items.iter().map(|item| item.shrink).sum::<f32>();
        let mut remaining = -free * shrink.min(1.0);
        let mut active = (0..items.len()).collect::<Vec<_>>();
        while remaining > f32::EPSILON && !active.is_empty() {
            let weight = active
                .iter()
                .map(|index| items[*index].shrink * items[*index].base_main)
                .sum::<f32>();
            if weight <= f32::EPSILON {
                break;
            }
            let mut clamped = Vec::new();
            for index in &active {
                let item = &items[*index];
                let reduction = remaining * item.shrink * item.base_main / weight;
                if reduction >= item.target_main {
                    clamped.push(*index);
                }
            }
            if clamped.is_empty() {
                for index in active {
                    let item = &mut items[index];
                    let reduction = remaining * item.shrink * item.base_main / weight;
                    item.target_main -= reduction;
                }
                break;
            }
            for index in &clamped {
                remaining -= items[*index].target_main;
                items[*index].target_main = 0.0;
            }
            active.retain(|index| !clamped.contains(index));
        }
    }
}

fn justify_line(justify: JustifyContent, free: f32, count: usize, reverse: bool) -> (f32, f32) {
    let logical_start = match justify {
        JustifyContent::Start => reverse,
        JustifyContent::End => !reverse,
        JustifyContent::FlexStart => false,
        JustifyContent::FlexEnd => true,
        _ => false,
    };
    match justify {
        JustifyContent::Start
        | JustifyContent::End
        | JustifyContent::FlexStart
        | JustifyContent::FlexEnd => (if logical_start { free } else { 0.0 }, 0.0),
        JustifyContent::Center => (free / 2.0, 0.0),
        JustifyContent::Stretch => (0.0, 0.0),
        JustifyContent::SpaceBetween if count > 1 && free > 0.0 => (0.0, free / (count - 1) as f32),
        JustifyContent::SpaceAround if count > 0 && free > 0.0 => {
            let between = free / count as f32;
            (between / 2.0, between)
        }
        JustifyContent::SpaceEvenly if count > 0 && free > 0.0 => {
            let between = free / (count + 1) as f32;
            (between, between)
        }
        JustifyContent::SpaceBetween
        | JustifyContent::SpaceAround
        | JustifyContent::SpaceEvenly => (0.0, 0.0),
    }
}

fn align_lines(
    lines: &mut [Line],
    target: f32,
    gap: f32,
    align: AlignContent,
    wrap: FlexWrap,
) -> (f32, f32) {
    if lines.is_empty() {
        return (0.0, 0.0);
    }
    if wrap == FlexWrap::NoWrap {
        lines[0].cross = lines[0].cross.max(target);
        return (0.0, 0.0);
    }
    let used = lines.iter().map(|line| line.cross).sum::<f32>()
        + gap * lines.len().saturating_sub(1) as f32;
    let free = target - used;
    match align {
        AlignContent::Start => (
            if wrap == FlexWrap::WrapReverse {
                free
            } else {
                0.0
            },
            0.0,
        ),
        AlignContent::End => (
            if wrap == FlexWrap::WrapReverse {
                0.0
            } else {
                free
            },
            0.0,
        ),
        AlignContent::FlexStart => (0.0, 0.0),
        AlignContent::FlexEnd => (free, 0.0),
        AlignContent::Center => (free / 2.0, 0.0),
        AlignContent::Stretch if free > 0.0 => {
            let extra = free / lines.len() as f32;
            for line in lines {
                line.cross += extra;
            }
            (0.0, 0.0)
        }
        AlignContent::SpaceBetween if lines.len() > 1 && free > 0.0 => {
            (0.0, free / (lines.len() - 1) as f32)
        }
        AlignContent::SpaceAround if free > 0.0 => {
            let between = free / lines.len() as f32;
            (between / 2.0, between)
        }
        AlignContent::SpaceEvenly if free > 0.0 => {
            let between = free / (lines.len() + 1) as f32;
            (between, between)
        }
        AlignContent::Stretch
        | AlignContent::SpaceBetween
        | AlignContent::SpaceAround
        | AlignContent::SpaceEvenly => (0.0, 0.0),
    }
}

fn align_item(align: AlignItems, free: f32, wrap_reverse: bool, row: bool) -> f32 {
    match align {
        AlignItems::Start | AlignItems::Stretch => 0.0,
        AlignItems::End => free,
        AlignItems::FlexStart => {
            if wrap_reverse {
                free
            } else {
                0.0
            }
        }
        AlignItems::FlexEnd => {
            if wrap_reverse {
                0.0
            } else {
                free
            }
        }
        AlignItems::Center => free / 2.0,
        // ponytail: Iced exposes no child baseline; bottom alignment is the closest
        // measurable fallback until Widget exposes baseline metrics.
        AlignItems::Baseline if row => free,
        AlignItems::Baseline => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced::{Font, Pixels, Theme};

    type TestRenderer = iced_test::renderer::Renderer;

    fn item(base: f32, grow: f32, shrink: f32) -> ItemLayout {
        ItemLayout {
            source: 0,
            base_main: base,
            target_main: base,
            natural_cross: 10.0,
            grow,
            shrink,
            margins: ResolvedMargins {
                main_start: Some(0.0),
                main_end: Some(0.0),
                cross_start: Some(0.0),
                cross_end: Some(0.0),
            },
        }
    }

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
    }

    fn renderer() -> TestRenderer {
        iced_test::futures::futures::executor::block_on(<TestRenderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            None,
        ))
        .expect("headless renderer")
    }

    #[test]
    fn distributes_grow_shrink_justify_and_wrapped_lines() {
        let mut growing = [item(50.0, 1.0, 1.0), item(50.0, 2.0, 1.0)];
        resolve_flex_line(&mut growing, 300.0, 0.0);
        close(growing[0].target_main, 116.666_67);
        close(growing[1].target_main, 183.333_33);

        let mut shrinking = [item(200.0, 0.0, 1.0), item(200.0, 0.0, 1.0)];
        resolve_flex_line(&mut shrinking, 300.0, 0.0);
        close(shrinking[0].target_main, 150.0);
        close(shrinking[1].target_main, 150.0);

        assert_eq!(
            justify_line(JustifyContent::SpaceBetween, 90.0, 4, false),
            (0.0, 30.0)
        );
        assert_eq!(
            justify_line(JustifyContent::SpaceEvenly, 100.0, 4, false),
            (20.0, 20.0)
        );
        assert_eq!(align_item(AlignItems::Start, 10.0, true, true), 0.0);
        assert_eq!(align_item(AlignItems::FlexStart, 10.0, true, true), 10.0);

        let wrapped = build_lines(
            &[item(60.0, 0.0, 1.0), item(60.0, 0.0, 1.0)],
            FlexWrap::Wrap,
            100.0,
            8.0,
        );
        assert_eq!(wrapped.len(), 2);
    }

    #[test]
    fn lays_out_real_iced_children_with_justify_and_order() {
        let first: Element<'_, (), Theme, TestRenderer> =
            iced::widget::Space::new().width(50.0).height(20.0).into();
        let second: Element<'_, (), Theme, TestRenderer> =
            iced::widget::Space::new().width(50.0).height(20.0).into();
        let flex = flex(vec![flex_item(first).order(1), flex_item(second)])
            .width(300.0)
            .height(40.0)
            .justify_content(JustifyContent::SpaceBetween)
            .align_items(AlignItems::Center);
        let mut element: Element<'_, (), Theme, TestRenderer> = flex.into();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer(),
            &layout::Limits::new(Size::ZERO, Size::new(300.0, 40.0)),
        );

        assert_eq!(node.size(), Size::new(300.0, 40.0));
        assert_eq!(
            node.children()[0].bounds(),
            Rectangle::new(Point::new(250.0, 10.0), Size::new(50.0, 20.0))
        );
        assert_eq!(
            node.children()[1].bounds(),
            Rectangle::new(Point::new(0.0, 10.0), Size::new(50.0, 20.0))
        );
    }

    #[test]
    fn uses_content_for_unresolved_percent_basis() {
        let child: Element<'_, (), Theme, TestRenderer> =
            iced::widget::container(iced::widget::Space::new().width(50.0).height(20.0))
                .width(Length::Fill)
                .into();
        let flex = flex(vec![flex_item(child).basis(FlexBasis::Percent(0.5))]);
        let mut element: Element<'_, (), Theme, TestRenderer> = flex.into();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer(),
            &layout::Limits::new(Size::ZERO, Size::new(300.0, 40.0)),
        );

        assert_eq!(node.size(), Size::new(50.0, 20.0));
    }
}
