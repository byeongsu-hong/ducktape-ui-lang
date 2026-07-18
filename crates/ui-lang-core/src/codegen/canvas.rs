use super::*;

pub(super) fn render_canvas(
    options: &CanvasOptions,
    locals: &[State],
    commands: &[CanvasCommand],
    events: &[CanvasEvent],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let state_fields = locals
        .iter()
        .map(|local| format!("{}: {},", local.name, local.ty.rust(&document.structs)))
        .collect::<Vec<_>>()
        .join(" ");
    let state_initials = locals
        .iter()
        .map(|local| {
            format!(
                "{}: {},",
                local.name,
                initial_code(&local.initial, &local.ty, document)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut canvas_env = env.clone();
    for local in locals {
        canvas_env.insert(
            local.name.clone(),
            Binding {
                code: format!("__state.{}", local.name),
                ty: local.ty.clone(),
                local: false,
            },
        );
    }
    canvas_env.insert(
        "canvas_width".into(),
        Binding {
            code: "(__bounds.width as f64)".into(),
            ty: Type::F64,
            local: true,
        },
    );
    canvas_env.insert(
        "canvas_height".into(),
        Binding {
            code: "(__bounds.height as f64)".into(),
            ty: Type::F64,
            local: true,
        },
    );
    let draw_commands = canvas_commands_code(commands, &canvas_env, document)?;
    let use_cache = options.cache.is_some();
    let cache_key = if let Some(dependency) = &options.cache {
        let dependency = expr_code(dependency, env, document, ValueMode::Owned)?;
        format!(
            "::std::option::Option::Some({{ let mut __hasher = ::std::hash::DefaultHasher::new(); ::std::hash::Hash::hash(&({dependency}), &mut __hasher); ::std::hash::Hasher::finish(&__hasher) }})"
        )
    } else {
        "::std::option::Option::None".into()
    };
    let update = canvas_update_code(
        options,
        events,
        env,
        &canvas_env,
        document,
        message,
        use_cache,
    )?;
    let interaction = if let Some(interaction) = &options.interaction_expr {
        let interaction = expr_code(interaction, &canvas_env, document, ValueMode::Owned)?;
        format!(
            "{{ let __interaction = {interaction}; __ice_canvas_interaction(__interaction.as_str()) }}"
        )
    } else {
        format!(
            "::iced::mouse::Interaction::{}",
            options
                .interaction
                .map(mouse_interaction_code)
                .unwrap_or("None")
        )
    };
    let interaction_outside = options
        .interaction_outside
        .as_ref()
        .map(|outside| expr_code(outside, &canvas_env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let cache_group = options.cache_group.as_ref().map_or_else(
        || "::std::option::Option::None".into(),
        |group| {
            format!(
                "::std::option::Option::Some(*{}.get_or_init(::iced::widget::canvas::Group::unique))",
                canvas_group_symbol(group)
            )
        },
    );
    let cache_setup = if use_cache {
        "let __cache = __state.cache.get_or_init(|| match __cache_group { ::std::option::Option::Some(group) => ::iced::widget::canvas::Cache::with_group(group), ::std::option::Option::None => ::iced::widget::canvas::Cache::new() }); if __state.cache_key.get() != __cache_key { __cache.clear(); __state.cache_key.set(__cache_key); }"
    } else {
        ""
    };
    let geometry = if use_cache {
        "__cache.draw(__renderer, __bounds.size(), __paint)"
    } else {
        "{ let mut __frame = ::iced::widget::canvas::Frame::new(__renderer, __bounds.size()); __paint(&mut __frame); __frame.into_geometry() }"
    };
    let mut code = format!(
        "{{ #[allow(dead_code)] struct __IceCanvasState {{ cache: ::std::cell::OnceCell<::iced::widget::canvas::Cache>, cache_key: ::std::cell::Cell<::std::option::Option<u64>>, inside: bool, {state_fields} }} impl ::std::default::Default for __IceCanvasState {{ fn default() -> Self {{ Self {{ cache: ::std::cell::OnceCell::new(), cache_key: ::std::cell::Cell::new(::std::option::Option::None), inside: false, {state_initials} }} }} }} let __cache_key: ::std::option::Option<u64> = {cache_key}; let __cache_group: ::std::option::Option<::iced::widget::canvas::Group> = {cache_group}; let __program = __IceCanvasProgram::<__IceCanvasState, {message}, _, _, _> {{ draw: move |__state: &__IceCanvasState, __renderer: &::iced::Renderer, __theme: &::iced::Theme, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ let _ = (&__cache_key, &__cache_group); {cache_setup} let __paint = move |__frame: &mut ::iced::widget::canvas::Frame| {{ {draw_commands} }}; let __geometry = {geometry}; ::std::vec![__geometry] }}, update: {update}, interaction: move |__state: &__IceCanvasState, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ if ({interaction_outside}) || __cursor.is_over(__bounds) {{ {interaction} }} else {{ ::iced::mouse::Interaction::default() }} }}, message: ::std::marker::PhantomData }}; let __canvas = ::iced::widget::canvas(__program)"
    );
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.height {
        write!(code, ".height({})", length_code(height, env, document)?).unwrap();
    }
    code.push_str("; __canvas.into() }");
    Ok(code)
}

mod commands;
mod events;
mod path;
mod style;

pub(super) use commands::*;
pub(super) use events::*;
pub(super) use path::*;
pub(super) use style::*;
