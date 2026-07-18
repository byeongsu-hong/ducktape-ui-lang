use super::*;

pub(in crate::codegen) fn render_node(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    if let Some(rendered) = render_foundation(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_controls(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_content(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_media(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_structure(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_documents(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    unreachable!("every view node belongs to a render group")
}

mod container;
mod content;
mod controls;
mod documents;
mod foundation;
mod layout;
mod media;
mod pane;
mod structure;
mod table;

pub(super) use container::*;
pub(super) use content::*;
pub(super) use controls::*;
pub(super) use documents::*;
pub(super) use foundation::*;
pub(super) use layout::*;
pub(super) use media::*;
pub(super) use pane::*;
pub(super) use structure::*;
pub(super) use table::*;
