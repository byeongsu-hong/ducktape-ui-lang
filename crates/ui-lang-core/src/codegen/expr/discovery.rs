use super::*;

pub(in crate::codegen) fn initial_code(expr: &Expr, ty: &Type, document: &Document) -> String {
    match (expr, ty) {
        (Expr::Str(value), Type::Str) => format!("{}.to_owned()", rust_string(value)),
        (Expr::Str(value), Type::Markdown) => format!(
            "::iced::widget::markdown::Content::parse({})",
            rust_string(value)
        ),
        (Expr::Str(value), Type::Editor) => format!(
            "::iced::widget::text_editor::Content::with_text({})",
            rust_string(value)
        ),
        (Expr::EmptyList, Type::List(_)) => "::std::vec::Vec::new()".into(),
        (Expr::EmptyList, Type::Combo(_)) => {
            "::iced::widget::combo_box::State::new(::std::vec::Vec::new())".into()
        }
        (Expr::List(values), Type::Combo(_)) => format!(
            "::iced::widget::combo_box::State::new(::std::vec![{}])",
            values
                .iter()
                .map(|value| {
                    expr_code(value, &HashMap::new(), document, ValueMode::Owned)
                        .unwrap_or_else(|_| "::core::default::Default::default()".into())
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        (Expr::None, Type::Option(_)) => "::std::option::Option::None".into(),
        (Expr::Bool(value), _) => value.to_string(),
        (Expr::I64(value), _) => value.to_string(),
        (Expr::F64(value), _) => rust_f64(*value),
        _ => expr_code(expr, &HashMap::new(), document, ValueMode::Owned)
            .unwrap_or_else(|_| "::core::default::Default::default()".into()),
    }
}

pub(in crate::codegen) fn pane_field(name: &str) -> String {
    format!("__pane_{name}")
}

pub(in crate::codegen) fn pane_configuration_code(configuration: &PaneConfiguration) -> String {
    match configuration {
        PaneConfiguration::Pane(name) => format!(
            "::iced::widget::pane_grid::Configuration::Pane({})",
            rust_string(name)
        ),
        PaneConfiguration::Split { axis, ratio, a, b } => {
            let axis = match axis {
                PaneAxis::Horizontal => "Horizontal",
                PaneAxis::Vertical => "Vertical",
            };
            format!(
                "::iced::widget::pane_grid::Configuration::Split {{ axis: ::iced::widget::pane_grid::Axis::{axis}, ratio: {ratio:?}, a: ::std::boxed::Box::new({}), b: ::std::boxed::Box::new({}) }}",
                pane_configuration_code(a),
                pane_configuration_code(b)
            )
        }
    }
}

pub(in crate::codegen) fn pane_resize_variant(name: &str) -> String {
    format!("__Pane{}Resize", pascal(name))
}

pub(in crate::codegen) fn pane_drag_variant(name: &str) -> String {
    format!("__Pane{}Drag", pascal(name))
}

pub(in crate::codegen) fn pane_grids(root: &ViewNode) -> Vec<&ViewNode> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<&'a ViewNode>) {
        match node {
            ViewNode::PaneGrid { panes, .. } => {
                output.push(node);
                for pane in panes {
                    for node in pane.nodes() {
                        collect(node, output);
                    }
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(root, &mut output);
    output
}

pub(in crate::codegen) fn uses_canvas(document: &Document) -> bool {
    !canvases(document).is_empty()
}

pub(in crate::codegen) fn canvases(document: &Document) -> Vec<(&CanvasOptions, &[CanvasEvent])> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<(&'a CanvasOptions, &'a [CanvasEvent])>) {
        match node {
            ViewNode::Canvas {
                options, events, ..
            } => output.push((options, events)),
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::PaneGrid { panes, .. } => {
                for node in panes.iter().flat_map(PaneView::nodes) {
                    collect(node, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(&document.view, &mut output);
    for component in &document.components {
        collect(&component.root, &mut output);
    }
    output
}

pub(in crate::codegen) fn canvas_cache_groups(document: &Document) -> Vec<&str> {
    let mut groups = Vec::new();
    for group in canvases(document)
        .into_iter()
        .filter_map(|(options, _)| options.cache_group.as_deref())
    {
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    groups
}

pub(in crate::codegen) fn canvas_events(document: &Document) -> Vec<&CanvasEvent> {
    canvases(document)
        .into_iter()
        .flat_map(|(_, events)| events)
        .collect()
}

pub(in crate::codegen) fn canvas_group_symbol(group: &str) -> String {
    format!("__ICE_CANVAS_GROUP_{}", group.to_ascii_uppercase())
}

pub(in crate::codegen) fn needs_extern_noop(document: &Document) -> bool {
    fn contains(node: &ViewNode) -> bool {
        match node {
            ViewNode::ExternComponent { route: None, .. }
            | ViewNode::Themer { route: None, .. }
            | ViewNode::Shader { route: None, .. } => true,
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => children.iter().any(contains),
            ViewNode::Tooltip { content, tip, .. } => contains(content) || contains(tip),
            ViewNode::Overlay { .. } => true,
            ViewNode::PaneGrid { panes, .. } => {
                panes.iter().flat_map(PaneView::nodes).any(contains)
            }
            ViewNode::Table { columns, .. } => columns
                .iter()
                .any(|column| contains(&column.header) || contains(&column.cell)),
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. } => contains(content),
            ViewNode::Component { slots, .. } => slots.iter().any(|slot| contains(&slot.content)),
            ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => contains(child),
            ViewNode::Button {
                content: Some(content),
                ..
            } => contains(content),
            ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => contains(content),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    contains(narrow) || contains(wide)
                }
                ResponsiveContent::Size { content, .. } => contains(content),
            },
            _ => false,
        }
    }
    contains(&document.view) || document.components.iter().any(|item| contains(&item.root))
}
