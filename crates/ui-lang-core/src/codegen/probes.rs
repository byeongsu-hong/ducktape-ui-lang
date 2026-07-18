use super::*;

pub(super) fn generate_extern_probes(out: &mut String, document: &Document) {
    if document
        .functions
        .iter()
        .any(|item| item.kind == ExternKind::EventFilter)
    {
        writeln!(out, "#[cfg(not(target_arch = \"wasm32\"))] type __IceEventStream<T> = ::iced::futures::stream::BoxStream<'static, T>; #[cfg(target_arch = \"wasm32\")] type __IceEventStream<T> = ::iced::futures::stream::LocalBoxStream<'static, T>;").unwrap();
    }
    for item in &document.structs {
        writeln!(
            out,
            "#[allow(dead_code)] fn __ui_lang_check_{}(value: &{}) {{",
            item.name.to_ascii_lowercase(),
            item.rust_path
        )
        .unwrap();
        for (field, ty) in &item.fields {
            writeln!(
                out,
                "let _: &{} = &value.{field};",
                ty.rust(&document.structs)
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
    }
    for item in &document.functions {
        let params = item
            .params
            .iter()
            .enumerate()
            .map(|(index, (_, ty))| format!("arg{index}: {}", ty.rust(&document.structs)))
            .collect::<Vec<_>>()
            .join(", ");
        let args = (0..item.params.len())
            .map(|index| format!("arg{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let output = item.error.as_ref().map_or_else(
            || item.output.rust(&document.structs),
            |error| {
                format!(
                    "::std::result::Result<{}, {}>",
                    item.output.rust(&document.structs),
                    error.rust(&document.structs)
                )
            },
        );
        match item.kind {
            ExternKind::Future => writeln!(
                out,
                "#[allow(dead_code)] async fn __ui_lang_check_{}({params}) {{ let _: {output} = {}({args}).await; }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Component => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_component_{}({params}) {{ let _: ::iced::Element<'static, {output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Shader => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_shader_{}({params}) {{ let __program = {}({args}); fn __accept<P: ::iced::widget::shader::Program<{output}>>(_: &P) {{}} __accept(&__program); let _: ::iced::Element<'static, {output}> = ::iced::widget::Shader::new(__program).into(); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Task => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_task_{}({params}) {{ let _: ::iced::Task<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Stream => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_stream_{}({params}) {{ let _: ::iced::Task<{output}> = ::iced::Task::run({}({args}), |value| value); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Sip => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_sip_{}({params}) {{ let _: ::iced::Task<()> = ::iced::Task::sip({}({args}), |value| {{ let _: {} = value; }}, |value| {{ let _: {output} = value; }}); }}",
                item.name,
                item.rust_path,
                item.progress
                    .as_ref()
                    .expect("sip extern has a progress type")
                    .rust(&document.structs)
            )
            .unwrap(),
            ExternKind::Recipe => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_recipe_{}({params}) {{ let __recipe = {}({args}); fn __accept<R: ::iced::advanced::subscription::Recipe<Output = {output}>>(_: &R) {{}} __accept(&__recipe); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Selector => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_selector_{}({params}) {{ let _: ::iced::Task<::std::option::Option<{output}>> = ::iced::widget::selector::find({}({args})); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::EventFilter => {
                let recipe = format!("__IceEventFilter{}", pascal(&item.name));
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_event_filter_{}() {{ let _: fn(::iced::advanced::subscription::Event) -> ::std::option::Option<{output}> = {}; }}",
                    item.name, item.rust_path
                )
                .unwrap();
                writeln!(
                    out,
                    "struct {recipe}<I> {{ id: I }} impl<I: ::std::hash::Hash + 'static> ::iced::advanced::subscription::Recipe for {recipe}<I> {{ type Output = {output}; fn hash(&self, state: &mut ::iced::advanced::subscription::Hasher) {{ ::std::hash::Hash::hash(&::std::any::TypeId::of::<Self>(), state); ::std::hash::Hash::hash(&self.id, state); }} fn stream(self: ::std::boxed::Box<Self>, input: ::iced::advanced::subscription::EventStream) -> __IceEventStream<Self::Output> {{ ::std::boxed::Box::pin(::iced::futures::StreamExt::filter_map(input, |event| ::iced::futures::future::ready({}(event)))) }} }}",
                    item.rust_path
                )
                .unwrap();
            }
            ExternKind::Sync => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_sync_{}({params}) {{ let _: {output} = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Subscription => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_subscription_{}({params}) {{ let _: ::iced::Subscription<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Theme => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_theme_{}({params}) {{ let _: ::iced::Theme = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Themer => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_themer_{}({params}) {{ let (__theme, __content, __text_color, __background) = {}({args}); fn __accept<T: ::iced::theme::Base>(_: &::std::option::Option<T>, _: &::iced::Element<'static, {output}, T>, _: &::std::option::Option<fn(&T) -> ::iced::Color>, _: &::std::option::Option<fn(&T) -> ::iced::Background>) {{}} __accept(&__theme, &__content, &__text_color, &__background); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Window => {
                let params = if params.is_empty() {
                    "window: &dyn ::iced::window::Window".into()
                } else {
                    format!("window: &dyn ::iced::window::Window, {params}")
                };
                let args = if args.is_empty() {
                    "window".into()
                } else {
                    format!("window, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_window_{}({params}) {{ let _: {output} = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::MarkdownViewer => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_markdown_viewer_{}({params}) {{ let __viewer = {}({args}); fn __accept<V>(_: &V) where for<'a> V: ::iced::widget::markdown::Viewer<'a, {output}, ::iced::Theme, ::iced::Renderer> {{}} __accept(&__viewer); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::EditorBinding => {
                let callback_params = std::iter::once(
                    "::iced::widget::text_editor::KeyPress".to_owned(),
                )
                .chain(
                    item.params
                        .iter()
                        .map(|(_, ty)| ty.rust(&document.structs)),
                )
                .collect::<Vec<_>>()
                .join(", ");
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_editor_binding_{}() {{ let _: fn({callback_params}) -> ::std::option::Option<::iced::widget::text_editor::Binding<{output}>> = {}; }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::EditorHighlighter => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_editor_highlighter_{}({params}) {{ let __content = ::iced::widget::text_editor::Content::new(); let __editor = ::iced::widget::text_editor(&__content).on_action(|_| ()); let _: ::iced::Element<'_, ()> = {}(__editor{}).into(); }}",
                item.name,
                item.rust_path,
                if args.is_empty() {
                    String::new()
                } else {
                    format!(", {args}")
                }
            )
            .unwrap(),
            ExternKind::EditorStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::text_editor::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::text_editor::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_editor_style_{}({params}) {{ let _: ::iced::widget::text_editor::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::TextStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_text_style_{}({params}) {{ let _: ::iced::widget::text::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::SliderStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::slider::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::slider::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_slider_style_{}({params}) {{ let _: ::iced::widget::slider::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ProgressStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_progress_style_{}({params}) {{ let _: ::iced::widget::progress_bar::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ButtonStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::button::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::button::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_button_style_{}({params}) {{ let _: ::iced::widget::button::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::CheckboxStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::checkbox::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::checkbox::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_checkbox_style_{}({params}) {{ let _: ::iced::widget::checkbox::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::TogglerStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::toggler::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::toggler::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_toggler_style_{}({params}) {{ let _: ::iced::widget::toggler::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::RadioStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::radio::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::radio::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_radio_style_{}({params}) {{ let _: ::iced::widget::radio::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ContainerStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_container_style_{}({params}) {{ let _: ::iced::widget::container::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::SvgStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::svg::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::svg::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_svg_style_{}({params}) {{ let _: ::iced::widget::svg::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::InputStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::text_input::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::text_input::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_input_style_{}({params}) {{ let _: ::iced::widget::text_input::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ScrollStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::scrollable::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::scrollable::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_scroll_style_{}({params}) {{ let _: ::iced::widget::scrollable::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::PickListStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::pick_list::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::pick_list::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_pick_list_style_{}({params}) {{ let _: ::iced::widget::pick_list::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::MenuStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_menu_style_{}({params}) {{ let _: ::iced::overlay::menu::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
        }
    }
}

pub(super) fn generate_editor_binding_mapper(out: &mut String, document: &Document) {
    if !document
        .functions
        .iter()
        .any(|item| item.kind == ExternKind::EditorBinding)
    {
        return;
    }
    writeln!(
        out,
        "fn __ice_map_editor_binding<T, M>(binding: ::iced::widget::text_editor::Binding<T>, custom: &impl Fn(T) -> M) -> ::iced::widget::text_editor::Binding<M> {{ use ::iced::widget::text_editor::Binding; match binding {{ Binding::Unfocus => Binding::Unfocus, Binding::Copy => Binding::Copy, Binding::Cut => Binding::Cut, Binding::Paste => Binding::Paste, Binding::Move(value) => Binding::Move(value), Binding::Select(value) => Binding::Select(value), Binding::SelectWord => Binding::SelectWord, Binding::SelectLine => Binding::SelectLine, Binding::SelectAll => Binding::SelectAll, Binding::Insert(value) => Binding::Insert(value), Binding::Enter => Binding::Enter, Binding::Backspace => Binding::Backspace, Binding::Delete => Binding::Delete, Binding::Sequence(values) => Binding::Sequence(values.into_iter().map(|value| __ice_map_editor_binding(value, custom)).collect()), Binding::Custom(value) => Binding::Custom(custom(value)), }} }}"
    )
    .unwrap();
}
