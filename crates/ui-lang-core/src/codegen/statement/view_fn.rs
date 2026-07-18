use super::*;

pub(in crate::codegen) fn generate_view(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let mut env = state_env(document, "self");
    if document.daemon {
        env.insert(
            "window".into(),
            Binding {
                code: "window".into(),
                ty: Type::WindowId,
                local: true,
            },
        );
    }
    let root = render_node(
        &document.view,
        document,
        message,
        &env,
        &rust_string(&document.app),
        None,
    )?;
    let window_arg = if document.daemon {
        ", window: ::iced::window::Id"
    } else {
        ""
    };
    if document.daemon {
        writeln!(
            out,
            "fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {root} }}"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ let __ice_content: __IceElement<'_, {message}> = {root}; ::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext, {message}::__AccessibilityFocusPrevious).into() }}"
        )
        .unwrap();
    }
    Ok(())
}
