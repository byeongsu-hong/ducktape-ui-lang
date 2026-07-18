use super::*;

#[test]
fn lowers_qr_data_and_widget_options() {
    let source = r#"app Codes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
qr automatic "one"
qr corrected "two" correction=quartile
qr fixed "three" correction=low version=micro(4)
qr binary bytes(00 ff a4)
view
  col
    qr automatic cell-size=5.0
    qr corrected total-size=120.0 cell=primary background=white
    qr fixed
    qr binary
"#;
    let generated = compile(source, "codes.ice").unwrap();
    assert!(generated.contains("qr_code::Data::new(\"one\")"));
    assert!(generated.contains("qr_code::Data::with_error_correction(\"two\", ::iced::widget::qr_code::ErrorCorrection::Quartile)"));
    assert!(generated.contains("qr_code::Data::with_version(\"three\", ::iced::widget::qr_code::Version::Micro(4), ::iced::widget::qr_code::ErrorCorrection::Low)"));
    assert!(generated.contains("qr_code::Data::new(&[0x00u8, 0xffu8, 0xa4u8][..])"));
    assert!(generated.contains("::iced::widget::qr_code(&self.automatic).cell_size(5.0 as f32)"));
    assert!(generated.contains(
        "::iced::widget::qr_code(&self.corrected).total_size(120.0 as f32).style(|theme|"
    ));
    assert!(generated.contains("qr_code::Style { cell: ::iced::Color"));
}
#[test]
fn lowers_nested_iced_themes() {
    let source = r#"app Themes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
  surface #111111
view
  col
    theme app
      text "App theme"
    theme tokyo-night text=foreground background=linear(1.57, surface@0.0, background@1.0)
      text "Built-in theme"
    theme dark background=surface
      text "Solid background"
    theme
      text "Default mode"
"#;
    let generated = compile(source, "themes.ice").unwrap();
    assert!(generated.contains("themer(::std::option::Option::Some(Self::__app_theme())"));
    assert!(generated.contains("themer(::std::option::Option::Some(::iced::Theme::TokyoNight)"));
    assert!(generated.contains(".text_color(|_| ::iced::Color"));
    assert!(generated.contains(".background(|_| ::iced::Background::Color"));
    assert!(generated.contains(".background(|_| ::iced::Background::from(::iced::gradient::Linear::new(1.57 as f32).add_stop(0.0 as f32"));
    assert!(generated.contains("themer(::std::option::Option::None"));
}

#[test]
fn lowers_native_theme_factories() {
    let source = r#"extern crate::backend
  theme native_theme(dark:bool)
app Themes
  theme native_theme(dark)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  dark = true
view
  theme native_theme(!dark)
    text "Nested"
"#;
    let generated = compile(source, "themes.ice").unwrap();
    assert!(generated.contains(
            "fn __ui_lang_check_theme_native_theme(arg0: bool) { let _: ::iced::Theme = crate::backend::native_theme(arg0); }"
        ));
    assert!(
        generated.contains(
            "fn __theme(&self) -> ::iced::Theme {\ncrate::backend::native_theme(self.dark)"
        )
    );
    assert!(generated.contains(
        "themer(::std::option::Option::Some(crate::backend::native_theme((!self.dark)))"
    ));
}

#[test]
fn lowers_alternate_theme_subtrees() {
    let source = r#"extern crate::backend
  themer alternate_panel(active:bool) -> bool
app Themes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
on changed(value)
  active = value
view
  themer alternate_panel(active) -> changed _
"#;
    let generated = compile(source, "themer.ice").unwrap();
    assert!(generated.contains(
            "fn __ui_lang_check_themer_alternate_panel(arg0: bool) { let (__theme, __content, __text_color, __background) = crate::backend::alternate_panel(arg0); fn __accept<T: ::iced::theme::Base>(_: &::std::option::Option<T>, _: &__IceElement<'static, bool, T>"
        ));
    assert!(generated.contains("let mut __themer = ::iced::widget::themer(__theme, __content)"));
    assert!(generated.contains("__themer = __themer.text_color(__text_color)"));
    assert!(generated.contains("__themer = __themer.background(__background)"));
    assert!(generated.contains("__themed.map(move |__value| __ThemesMessage::Changed(__value))"));
}

#[test]
fn lowers_component_children_and_slot_forwarding() {
    let source = r#"app Composition
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
component Card(title:str)
  col #card
    text title
    slot
component Wrapper(title:str)
  Card title=title
    slot
view
  Wrapper title="Editor" #editor
    input "Name" #name <-> draft
"#;
    let generated = compile(source, "composition.ice").unwrap();
    assert!(generated.contains("__BindDraft(::std::string::String)"));
    assert!(generated.contains("::iced::widget::text_input(\"\", &self.draft)"));
    assert!(generated.contains("format!(\"{}/name\""));
    assert!(generated.contains("format!(\"{}/Card@"));
}

#[test]
fn lowers_named_slots_and_named_slot_forwarding() {
    let source = r#"app Composition
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Frame()
  col
    slot heading
    slot body
component Dialog()
  Frame
    heading:
      slot title
    body:
      col
        slot content
        slot actions
on cancel
on delete
view
  Dialog
    title:
      text "Delete task?"
    content:
      text "This cannot be undone."
    actions:
      row
        button "Cancel" -> cancel
        button "Delete" -> delete
"#;
    let generated = compile(source, "composition.ice").unwrap();
    assert!(generated.contains("Delete task?"));
    assert!(generated.contains("This cannot be undone."));
    assert!(generated.contains("Cancel"));
    assert!(generated.contains("Delete"));
}

#[test]
fn lowers_compound_components_into_named_slots() {
    let source = r#"app Composition
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog()
  col
    slot Header
    slot Body
component Dialog.Header()
  container #root
    slot
component Dialog.Body()
  container #root
    slot
view
  Dialog
    Dialog.Header
      text "Compound title"
    Dialog.Body
      text "Structured body"
"#;
    let generated = compile(source, "composition.ice").unwrap();
    assert!(generated.contains("Compound title"));
    assert!(generated.contains("Structured body"));
    assert!(generated.contains("format!(\"{}/Dialog.Header@"));
    assert!(generated.contains("format!(\"{}/Dialog.Body@"));
}

#[test]
fn lowers_fully_configured_keyed_columns() {
    let source = r#"app Keyed
extern crate::backend
  Item(id:i64, name:str)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
view
  keyed item in items by=item.id width=fill(2) height=120.0 spacing=8.0 padding=4.0 padding-left=12.0 max-width=640.0 align=end
    scroll #row
      text item.name
"#;
    let generated = compile(source, "keyed.ice").unwrap();
    assert!(generated.contains("for item in self.items.iter()"));
    assert!(generated.contains("__children.push((__key, __child))"));
    assert!(generated.contains("::iced::widget::keyed_column(__children)"));
    assert!(generated.contains(".spacing(8.0 as f32)"));
    assert!(generated.contains("left: 12.0 as f32"));
    assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));
    assert!(generated.contains(".height(120.0 as f32)"));
    assert!(generated.contains(".max_width(640.0 as f32)"));
    assert!(generated.contains(".align_items(::iced::Alignment::End)"));
    assert!(generated.contains("format!(\"{}/key({})\""));
}

#[test]
fn lowers_lazy_to_an_owned_static_subtree() {
    let source = r#"app LazyDemo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  title = "Hello"
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
    let generated = compile(source, "lazy.ice").unwrap();
    assert!(
        generated.contains("::iced::widget::lazy((self.title.clone(), (\"LazyDemo\").to_owned())")
    );
    assert!(generated.contains("let cached: ::std::string::String = __dependency.0.clone()"));
    assert!(generated.contains("let __lazy_content: __IceElement<'static,"));
    assert!(generated.contains("let __lazy_scope = __dependency.1.clone()"));
}

#[test]
fn lowers_parsed_markdown_with_complete_sizes_and_link_route() {
    let source = r##"app Docs
font ui family=sans
extern crate::backend
  markdown-viewer docs_viewer(prefix:str) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  docs:markdown = "# Hello"
  images:[str] = []
on open(url)
on reset
  docs = markdown("# Reset")
on extend
  markdown docs append "\n![Ice](asset://ice)"
  images = markdown_images(docs)
view
  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 spacing=12.0 viewer=docs_viewer("docs") -> open _
    style font=ui inline-code-background=linear(1.57, background@0.0, primary@1.0) inline-code-color=foreground inline-code-font=mono code-block-font=mono link=primary inline-code-padding=2.0 inline-code-padding-x=3.0 inline-code-padding-y=4.0 inline-code-padding-top=5.0 inline-code-padding-right=6.0 inline-code-padding-bottom=7.0 inline-code-padding-left=8.0 inline-code-border=primary inline-code-border-width=1.0 inline-code-radius=4.0 inline-code-radius-tl=1.0 inline-code-radius-tr=2.0 inline-code-radius-br=3.0 inline-code-radius-bl=4.0
"##;
    let generated = compile(source, "docs.ice").unwrap();
    assert!(generated.contains("docs: ::iced::widget::markdown::Content::parse(\"# Hello\")"));
    assert!(
        generated.contains(
            "self.docs = ::iced::widget::markdown::Content::parse(&\"# Reset\".to_owned())"
        )
    );
    for field in [
        "text_size",
        "h1_size",
        "h2_size",
        "h3_size",
        "h4_size",
        "h5_size",
        "h6_size",
        "code_size",
        "spacing",
    ] {
        assert!(generated.contains(&format!("__markdown_settings.{field} =")));
    }
    assert!(generated.contains("self.docs.push_str(&\"\\n![Ice](asset://ice)\".to_owned())"));
    assert!(generated.contains(".images().iter().cloned().collect"));
    assert!(generated.contains("::iced::widget::markdown::view_with(self.docs.items()"));
    assert!(generated.contains("crate::backend::docs_viewer(\"docs\".to_owned())"));
    assert!(generated.contains("map(move |__event| __DocsMessage::Open(__event))"));
    assert!(generated.contains("fn __ui_lang_check_markdown_viewer_docs_viewer"));
    for field in [
        "style.font",
        "style.inline_code_highlight.background",
        "style.inline_code_color",
        "style.inline_code_font",
        "style.code_block_font",
        "style.link_color",
        "style.inline_code_padding",
        "style.inline_code_highlight.border.color",
        "style.inline_code_highlight.border.width",
        "style.inline_code_highlight.border.radius",
    ] {
        assert!(generated.contains(&format!("__markdown_settings.{field} =")));
    }
}

#[test]
fn lowers_structured_tables_with_complete_native_options() {
    let source = r#"app Rows
extern crate::backend
  Item(name:str, done:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  rows:[Item] = []
view
  table row in rows width=fill padding=4.0 padding-x=8.0 padding-y=6.0 separator=1.0 separator-x=2.0 separator-y=3.0
    column width=fill(2) align-x=right align-y=bottom
      header
        text "Name"
      cell
        scroll #value
          text row.name
"#;
    let generated = compile(source, "rows.ice").unwrap();
    assert!(generated.contains("table::table(::std::vec!["));
    assert!(generated.contains("self.rows.clone().into_iter().enumerate()"));
    assert!(generated.contains("move |(__row, row): (usize, crate::backend::Item)|"));
    assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Right)"));
    assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
    for method in [
        "padding(4.0 as f32)",
        "padding_x(8.0 as f32)",
        "padding_y(6.0 as f32)",
        "separator(1.0 as f32)",
        "separator_x(2.0 as f32)",
        "separator_y(3.0 as f32)",
    ] {
        assert!(generated.contains(method));
    }
    assert!(generated.contains("format!(\"{}/row({})/column(0)\""));
}

#[test]
fn lowers_bound_text_editors_and_internal_actions() {
    let source = r#"app Notes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = "fn main() {}"
  locked = false
view
  editor #body <-> body placeholder="Write" width=640.0 height=fill min-height=80.0 max-height=240.0 size=14.0 line-height-px=18.0 padding=8.0 wrapping=word-or-glyph font=mono highlight="rs" highlight-theme=inspired-github disabled=locked
    active background=background border=foreground border-width=1.0 radius=4.0 placeholder=danger value=foreground selection=primary
    hovered background=background border=primary placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
"#;
    let generated = compile(source, "notes.ice").unwrap();
    assert!(generated.contains("body: ::iced::widget::text_editor::Content::with_text"));
    assert!(generated.contains("__EditBody(::iced::widget::text_editor::Action)"));
    assert!(generated.contains("self.body.perform(action)"));
    assert!(generated.contains("::iced::widget::text_editor(&self.body)"));
    assert!(generated.contains(".width(640.0 as f32)"));
    assert!(generated.contains(".height(::iced::Fill)"));
    assert!(generated.contains(".min_height(80.0 as f32)"));
    assert!(generated.contains(".max_height(240.0 as f32)"));
    assert!(generated.contains("LineHeight::Absolute((18.0 as f32).into())"));
    assert!(generated.contains("Wrapping::WordOrGlyph"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains(".highlight(\"rs\", ::iced::highlighter::Theme::InspiredGitHub)"));
    assert!(generated.contains("::iced::widget::text_editor::default"));
    assert!(generated.contains("text_editor::Status::Focused { is_hovered: true }"));
    assert!(generated.contains("__style.placeholder ="));
    assert!(generated.contains("__style.selection ="));
    assert!(generated.contains("if self.locked"));
    assert!(generated.contains(".on_action(__NotesMessage::__EditBody"));
}

#[test]
fn lowers_component_controls_and_editor_extensions() {
    let source = r#"app Notes
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-style editor_surface(readonly:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = ""
  title = "Notes"
  locked = false
  language = "rs"
component EditorPanel(content:editor, heading:str, readonly:bool, syntax:str)
  col
    input "Title" <-> heading
    editor <-> content highlighter=editor_highlight(syntax) key-binding=editor_keys(readonly) style=editor_surface(readonly) -> command _
on command(value)
view
  EditorPanel(body, title, locked, language)
"#;
    let generated = compile(source, "notes.ice").unwrap();
    assert!(generated.contains("__BindTitle(::std::string::String)"));
    assert!(generated.contains("__EditBody(::iced::widget::text_editor::Action)"));
    assert!(generated.contains("text_input(\"\", &self.title)"));
    assert!(generated.contains("text_editor(&self.body)"));
    assert!(generated.contains("crate::backend::editor_keys(__key_press, self.locked)"));
    assert!(generated.contains("__ice_map_editor_binding"));
    assert!(generated.contains("__NotesMessage::Command(__value)"));
    assert!(generated.contains("crate::backend::editor_highlight("));
    assert!(generated.contains(", self.language.clone())"));
    assert!(generated.contains("fn __ui_lang_check_editor_binding_editor_keys"));
    assert!(generated.contains("fn __ui_lang_check_editor_highlighter_editor_highlight"));
    assert!(generated.contains("fn __ui_lang_check_editor_style_editor_surface"));
    assert!(generated.contains("crate::backend::editor_surface(__theme, __status, self.locked)"));
    assert!(generated.contains("self.title = value"));
    assert!(generated.contains("self.body.perform(action)"));
}
