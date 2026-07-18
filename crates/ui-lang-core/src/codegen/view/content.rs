use super::*;

pub(in crate::codegen) fn render_content(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Rule {
            axis,
            thickness,
            options,
            ..
        } => {
            let thickness = expr_code(thickness, env, document, ValueMode::Owned)?;
            let axis = match axis {
                Axis::Horizontal => "horizontal",
                Axis::Vertical => "vertical",
            };
            let mut code = format!("::iced::widget::rule::{axis}({thickness} as f32)");
            append_rule_options(&mut code, options, env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::QrCode {
            data,
            cell_size,
            total_size,
            cell,
            background,
            ..
        } => {
            let mut code = format!("::iced::widget::qr_code(&self.{data})");
            if let Some(value) = cell_size {
                write!(
                    code,
                    ".cell_size({} as f32)",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(value) = total_size {
                write!(
                    code,
                    ".total_size({} as f32)",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if cell.is_some() || background.is_some() {
                let cell = cell.as_deref().map(|value| theme_color(document, value));
                let background = background
                    .as_deref()
                    .map(|value| theme_color(document, value));
                write!(
                    code,
                    ".style(|theme| {{ let default = ::iced::widget::qr_code::default(theme); ::iced::widget::qr_code::Style {{ cell: {}, background: {} }} }})",
                    cell.unwrap_or_else(|| "default.cell".into()),
                    background.unwrap_or_else(|| "default.background".into())
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Space { width, height, .. } => {
            let mut code = String::from("::iced::widget::space()");
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Component {
            name,
            args,
            id,
            slots,
            span,
        } => {
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            let mut component_env = HashMap::new();
            for (index, (param, ty)) in component.params.iter().enumerate() {
                let arg = if args.iter().any(|arg| arg.name.is_some()) {
                    args.iter()
                        .find(|arg| arg.name.as_ref() == Some(param))
                        .expect("checker requires every named component prop")
                } else {
                    &args[index]
                };
                component_env.insert(
                    param.clone(),
                    Binding {
                        code: expr_code(&arg.value, env, document, ValueMode::Borrowed)?,
                        ty: ty.clone(),
                        local: false,
                    },
                );
            }
            let component_scope = id.as_ref().map_or_else(
                || format!("format!(\"{{}}/{}\", {scope})", name),
                |id| id_code(id, scope, env, document).unwrap_or_else(|_| scope.into()),
            );
            let component_slots = (!slots.is_empty()).then(|| SlotContext {
                entries: slots
                    .iter()
                    .map(|component_slot| SlotContent {
                        name: component_slot.name.clone(),
                        node: (*component_slot.content).clone(),
                        env: env.clone(),
                    })
                    .collect(),
                parent: slot.cloned().map(Box::new),
            });
            render_node(
                &component.root,
                document,
                message,
                &component_env,
                &component_scope,
                component_slots.as_ref(),
            )
        }
        ViewNode::Slot { name, span } => {
            let slot = slot.ok_or_else(|| {
                Error::new(
                    "E170",
                    span,
                    "slot reached codegen without component content",
                )
            })?;
            let content = slot
                .entries
                .iter()
                .find(|entry| entry.name == *name)
                .ok_or_else(|| {
                    Error::new(
                        "E170",
                        span,
                        format!("slot `{name}` reached codegen without component content"),
                    )
                })?;
            render_node(
                &content.node,
                document,
                message,
                &content.env,
                scope,
                slot.parent.as_deref(),
            )
        }
        ViewNode::ExternComponent {
            function,
            args,
            route,
            span,
        } => {
            let component = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Component)
                .ok_or_else(|| {
                    Error::new(
                        "E130",
                        span,
                        format!("unknown extern component `{function}`"),
                    )
                })?;
            let args = args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mapped = if let Some(route) = route {
                route_code(route, "__value", env, document, message)?
            } else {
                format!("{message}::__ExternNoop")
            };
            Ok(format!(
                "{}({args}).map(move |__value| {mapped}).into()",
                component.rust_path
            ))
        }
        ViewNode::Themer {
            function,
            args,
            route,
            span,
        } => {
            let themer = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Themer)
                .ok_or_else(|| {
                    Error::new("E130", span, format!("unknown extern themer `{function}`"))
                })?;
            let args = args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mapped = if let Some(route) = route {
                route_code(route, "__value", env, document, message)?
            } else {
                format!("{message}::__ExternNoop")
            };
            Ok(format!(
                "{{ let (__theme, __content, __text_color, __background) = {}({args}); let mut __themer = ::iced::widget::themer(__theme, __content); if let ::std::option::Option::Some(__text_color) = __text_color {{ __themer = __themer.text_color(__text_color); }} if let ::std::option::Option::Some(__background) = __background {{ __themer = __themer.background(__background); }} let __themed: ::iced::Element<'_, {}> = __themer.into(); __themed.map(move |__value| {mapped}).into() }}",
                themer.rust_path,
                themer.output.rust(&document.structs)
            ))
        }
        ViewNode::Shader {
            function,
            args,
            width,
            height,
            route,
            span,
        } => {
            let shader = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Shader)
                .ok_or_else(|| Error::new("E191", span, format!("unknown shader `{function}`")))?;
            let args = args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mut code = format!("::iced::widget::Shader::new({}({args}))", shader.rust_path);
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            let output = shader.output.rust(&document.structs);
            let mapped = if let Some(route) = route {
                route_code(route, "__value", env, document, message)?
            } else {
                format!("{message}::__ExternNoop")
            };
            Ok(format!(
                "{{ let __shader: ::iced::Element<'_, {output}> = {code}.into(); __shader.map(move |__value| {mapped}).into() }}"
            ))
        }
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
