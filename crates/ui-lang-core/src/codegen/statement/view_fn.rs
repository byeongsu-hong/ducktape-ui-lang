use super::*;

pub(in crate::codegen) fn generate_view(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let env = state_env(document, "self");
    let root = render_node(
        &document.view,
        document,
        message,
        &env,
        &rust_string(&document.app),
        None,
    )?;
    writeln!(
        out,
        "fn __view(&self) -> ::iced::Element<'_, {message}> {{ {root} }}"
    )
    .unwrap();
    Ok(())
}
