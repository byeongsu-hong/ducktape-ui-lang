use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_table(
    item: &str,
    rows: &Expr,
    options: &TableOptions,
    columns: &[TableColumn],
    span: &Span,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let Type::List(inner) = expr_type(
        rows,
        &env.iter()
            .map(|(name, binding)| (name.clone(), binding.ty.clone()))
            .collect(),
        document,
        span,
    )?
    else {
        unreachable!("checker validates table rows")
    };
    let rows = expr_code(rows, env, document, ValueMode::Owned)?;
    let row_type = *inner;
    let row_rust = row_type.rust(&document.structs);
    let mut cell_env = env.clone();
    cell_env.insert(
        item.into(),
        Binding {
            code: item.into(),
            ty: row_type,
            local: true,
        },
    );
    let mut column_codes = Vec::with_capacity(columns.len());
    for (index, column) in columns.iter().enumerate() {
        let header_scope = format!("format!(\"{{}}/header({index})\", {scope})");
        let cell_scope = format!("format!(\"{{}}/row({{}})/column({index})\", {scope}, __row)");
        let header = render_node(&column.header, document, message, env, &header_scope, slot)?;
        let cell = render_node(
            &column.cell,
            document,
            message,
            &cell_env,
            &cell_scope,
            slot,
        )?;
        let mut code = format!(
            "{{ let __table_header: ::iced::Element<'_, {message}> = {header}; ::iced::widget::table::column(__table_header, move |(__row, {item}): (usize, {row_rust})| -> ::iced::Element<'_, {message}> {{ {cell} }})"
        );
        if let Some(width) = &column.width {
            write!(code, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(align) = column.align_x {
            let align = match align {
                InputAlignment::Left => "Left",
                InputAlignment::Center => "Center",
                InputAlignment::Right => "Right",
            };
            write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
        }
        if let Some(align) = column.align_y {
            let align = match align {
                VerticalAlignment::Top => "Top",
                VerticalAlignment::Center => "Center",
                VerticalAlignment::Bottom => "Bottom",
            };
            write!(code, ".align_y(::iced::alignment::Vertical::{align})").unwrap();
        }
        code.push_str(" }");
        column_codes.push(code);
    }
    let mut code = format!(
        "::iced::widget::table::table(::std::vec![{}], {rows}.into_iter().enumerate())",
        column_codes.join(", ")
    );
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    for (value, method) in [
        (&options.padding, "padding"),
        (&options.padding_x, "padding_x"),
        (&options.padding_y, "padding_y"),
        (&options.separator, "separator"),
        (&options.separator_x, "separator_x"),
        (&options.separator_y, "separator_y"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into()"))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_keyed_column(
    item: &str,
    items: &Expr,
    key: &Expr,
    options: &LayoutOptions,
    child: &ViewNode,
    span: &Span,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let Type::List(inner) = expr_type(
        items,
        &env.iter()
            .map(|(name, binding)| (name.clone(), binding.ty.clone()))
            .collect(),
        document,
        span,
    )?
    else {
        unreachable!("checker validates keyed lists")
    };
    let items = expr_code(items, env, document, ValueMode::Borrowed)?;
    let mut child_env = env.clone();
    child_env.insert(
        item.into(),
        Binding {
            code: item.into(),
            ty: *inner,
            local: false,
        },
    );
    let key = expr_code(key, &child_env, document, ValueMode::Owned)?;
    let child_scope = format!("format!(\"{{}}/key({{}})\", {scope}, __key)");
    let child = render_node(child, document, message, &child_env, &child_scope, slot)?;
    let mut code = format!(
        "{{ let mut __children: ::std::vec::Vec<_> = ::std::vec::Vec::new(); for {item} in {items}.iter() {{ let __key = {key}; let __child: ::iced::Element<'_, {message}> = {child}; __children.push((__key, __child)); }} let __layout = ::iced::widget::keyed_column(__children)"
    );
    if let Some(spacing) = &options.spacing {
        write!(
            code,
            ".spacing({} as f32)",
            expr_code(spacing, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.height {
        write!(code, ".height({})", length_code(height, env, document)?).unwrap();
    }
    if let Some(max_width) = &options.max_width {
        write!(
            code,
            ".max_width({} as f32)",
            expr_code(max_width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(align) = options.align {
        let align = match align {
            FlexAlignment::Start => "Start",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "End",
        };
        write!(code, ".align_items(::iced::Alignment::{align})").unwrap();
    }
    Ok(format!("{code}; __layout.into() }}"))
}
