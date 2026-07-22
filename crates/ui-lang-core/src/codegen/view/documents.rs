use super::*;

pub(in crate::codegen) fn render_documents(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Markdown {
            content,
            options,
            route,
            ..
        } => {
            let mut settings = String::from(
                "let mut __markdown_settings = ::iced::widget::markdown::Settings::from(self.__theme());",
            );
            for (value, field) in [
                (&options.text_size, "text_size"),
                (&options.h1_size, "h1_size"),
                (&options.h2_size, "h2_size"),
                (&options.h3_size, "h3_size"),
                (&options.h4_size, "h4_size"),
                (&options.h5_size, "h5_size"),
                (&options.h6_size, "h6_size"),
                (&options.code_size, "code_size"),
                (&options.spacing, "spacing"),
            ] {
                if let Some(value) = value {
                    write!(
                        settings,
                        " __markdown_settings.{field} = ({} as f32).into();",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                }
            }
            let style = &options.style;
            if let Some(font) = &style.font {
                write!(
                    settings,
                    " __markdown_settings.style.font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(background) = &style.inline_code_background {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.background = {};",
                    background_code(background, env, document)?
                )
                .unwrap();
            }
            if let Some(color) = &style.inline_code_color {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(font) = &style.inline_code_font {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(font) = &style.code_block_font {
                write!(
                    settings,
                    " __markdown_settings.style.code_block_font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(color) = &style.link_color {
                write!(
                    settings,
                    " __markdown_settings.style.link_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(padding) = typed_padding_code(&style.inline_code_padding, env, document)? {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_padding = {padding};"
                )
                .unwrap();
            }
            if let Some(color) = &style.inline_code_border_color {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(width) = &style.inline_code_border_width {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.width = {} as f32;",
                    expr_code(width, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(radius) = radius_code(
                style.inline_code_radius.as_ref(),
                [
                    style.inline_code_radius_top_left.as_ref(),
                    style.inline_code_radius_top_right.as_ref(),
                    style.inline_code_radius_bottom_right.as_ref(),
                    style.inline_code_radius_bottom_left.as_ref(),
                ],
                env,
                document,
            )? {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.radius = {radius};"
                )
                .unwrap();
            }
            let callback =
                route_callback_code(route, "__event", "__event", env, document, message)?;
            let view = if let Some(viewer) = &options.viewer {
                let function = document
                    .functions
                    .iter()
                    .find(|item| {
                        item.name == viewer.function && item.kind == ExternKind::MarkdownViewer
                    })
                    .expect("checker validates markdown viewer");
                let args = viewer
                    .args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!(
                    "let __markdown_viewer = {}({args}); ::iced::widget::markdown::view_with(self.{content}.items(), __markdown_settings, &__markdown_viewer)",
                    function.rust_path
                )
            } else {
                format!(
                    "::iced::widget::markdown::view(self.{content}.items(), __markdown_settings)"
                )
            };
            Ok(format!("{{ {settings} {view}.map({callback}) }}"))
        }
        ViewNode::TextEditor {
            binding,
            id,
            disabled,
            options,
            span,
        } => {
            let state = env.get(binding).ok_or_else(|| {
                Error::new("E150", span, format!("unknown editor state `{binding}`"))
            })?;
            let state_name = controlled_state_name(&state.code, "editor", span)?;
            let mut code = format!("::iced::widget::text_editor(&{})", state.code);
            if let Some(id) = id {
                write!(
                    code,
                    ".id(::iced::widget::Id::from({}))",
                    id_code(id, scope, env, document)?
                )
                .unwrap();
            }
            if let Some(placeholder) = &options.placeholder {
                write!(code, ".placeholder({})", rust_string(placeholder)).unwrap();
            }
            if let Some(width) = &options.width {
                write!(
                    code,
                    ".width({} as f32)",
                    expr_code(width, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            for (value, method) in [
                (&options.min_height, "min_height"),
                (&options.max_height, "max_height"),
                (&options.size, "size"),
                (&options.padding, "padding"),
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
            if let Some(line_height) = &options.line_height {
                match line_height {
                    TextLineHeight::Relative(value) => write!(
                        code,
                        ".line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                    TextLineHeight::Absolute(value) => write!(
                        code,
                        ".line_height(::iced::widget::text::LineHeight::Absolute(({} as f32).into()))",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                }
            }
            if let Some(wrapping) = options.wrapping {
                write!(
                    code,
                    ".wrapping(::iced::widget::text::Wrapping::{})",
                    text_wrapping_code(wrapping)
                )
                .unwrap();
            }
            if let Some(font) = &options.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(syntax) = &options.highlight {
                let theme = match options
                    .highlight_theme
                    .unwrap_or(HighlightTheme::Base16Ocean)
                {
                    HighlightTheme::SolarizedDark => "SolarizedDark",
                    HighlightTheme::Base16Mocha => "Base16Mocha",
                    HighlightTheme::Base16Ocean => "Base16Ocean",
                    HighlightTheme::Base16Eighties => "Base16Eighties",
                    HighlightTheme::InspiredGithub => "InspiredGitHub",
                };
                write!(
                    code,
                    ".highlight({}, ::iced::highlighter::Theme::{theme})",
                    rust_string(syntax)
                )
                .unwrap();
            }
            if let Some(binding) = &options.key_binding {
                let function = document
                    .functions
                    .iter()
                    .find(|item| {
                        item.name == binding.function && item.kind == ExternKind::EditorBinding
                    })
                    .expect("checker validates editor binding");
                let route = options
                    .key_binding_route
                    .as_ref()
                    .expect("parser requires a key-binding route");
                let callback = route_callback_with_code(
                    route,
                    "__key_press",
                    env,
                    document,
                    |callback_env| {
                        let args = binding
                            .args
                            .iter()
                            .map(|arg| expr_code(arg, callback_env, document, ValueMode::Owned))
                            .collect::<Result<Vec<_>, _>>()?;
                        let route = route_code(route, "__value", callback_env, document, message)?;
                        Ok(format!(
                            "{}(__key_press{}).map(|__binding| __ice_map_editor_binding(__binding, &|__value| {route}))",
                            function.rust_path,
                            args.iter()
                                .map(|arg| format!(", {arg}"))
                                .collect::<String>()
                        ))
                    },
                )?;
                write!(code, ".key_binding({callback})").unwrap();
            }
            code.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                None,
                env,
                document,
                "style",
                "text_editor",
            )?);
            let finish = |editor: String| -> Result<String, Error> {
                if let Some(highlighter) = &options.highlighter {
                    let function = document
                        .functions
                        .iter()
                        .find(|item| {
                            item.name == highlighter.function
                                && item.kind == ExternKind::EditorHighlighter
                        })
                        .expect("checker validates editor highlighter");
                    let args = highlighter
                        .args
                        .iter()
                        .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(format!(
                        "{}({editor}{})",
                        function.rust_path,
                        args.iter()
                            .map(|arg| format!(", {arg}"))
                            .collect::<String>()
                    ))
                } else {
                    Ok(editor)
                }
            };
            let variant = editor_variant(&state_name);
            let enabled = format!(
                "{code}.on_action({message}::{variant} as fn(::iced::widget::text_editor::Action) -> {message})"
            );
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                let disabled_editor = finish(code)?;
                let enabled_editor = finish(enabled)?;
                Ok(format!(
                    "if {disabled} {{ {disabled_editor}.into() }} else {{ {enabled_editor}.into() }}"
                ))
            } else {
                Ok(format!("{}.into()", finish(enabled)?))
            }
        }
        ViewNode::Table {
            item,
            rows,
            options,
            columns,
            span,
        } => render_table(
            item, rows, options, columns, span, document, message, env, scope, slot,
        ),
        ViewNode::If { span, .. } | ViewNode::For { span, .. } => Err(Error::new(
            "E170",
            span,
            "if and for must be children of a layout node",
        )),
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
