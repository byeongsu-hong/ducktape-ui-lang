use super::*;

pub(in crate::codegen) fn render_children(
    out: &mut String,
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<(), Error> {
    for child in children {
        match child {
            ViewNode::If {
                condition,
                children,
                ..
            } => {
                let condition = expr_code(condition, env, document, ValueMode::Owned)?;
                write!(out, " if {condition} {{").unwrap();
                render_children(out, children, document, message, env, scope, slot)?;
                out.push_str(" }");
            }
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                let Type::List(inner) = expr_type(
                    items,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?
                else {
                    return Err(Error::new("E121", span, "for expects a list"));
                };
                let items = expr_code(items, env, document, ValueMode::Borrowed)?;
                write!(out, " for {item} in {items}.iter() {{").unwrap();
                let mut child_env = env.clone();
                child_env.insert(
                    item.clone(),
                    Binding {
                        code: item.clone(),
                        ty: *inner,
                        local: false,
                    },
                );
                render_children(out, children, document, message, &child_env, scope, slot)?;
                out.push_str(" }");
            }
            _ => {
                let child = render_node(child, document, message, env, scope, slot)?;
                write!(out, " __children.push({child});").unwrap();
            }
        }
    }
    Ok(())
}
