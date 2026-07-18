
use crate::compile;

#[test]
fn lowers_complete_common_application_and_window_settings() {
    let source = r#"app Configured
  title "Configured app"
  theme "dark"
  background "123456"
  text-color "abcdef"
  id "dev.example.configured"
  executor iced::executor::Default
  font "fonts/Brand.ttf"
  font "fonts/Icons.otf"
  default-text-size 15
  antialiasing false
  vsync false
  scale-factor 1.25
  window
    icon-rgba "assets/app.rgba" 2 1
    size 960 720
    maximized true
    fullscreen false
    position 10 -20
    min-size 480 360
    max-size 1920 1080
    visible true
    resizable false
    closeable false
    minimizable false
    decorations false
    transparent true
    blur true
    level always-on-top
    exit-on-close-request false
    platform linux
      application-id "dev.example.configured"
      override-redirect true
    platform windows
      drag-and-drop false
      skip-taskbar true
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target none
state
  ready = false
extern crate::backend
  task seed() -> bool
preset ready
  state
    ready = true
  boot
    task seed() -> seeded _
on seeded(value)
  ready = value
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  text "Configured"
"#;
    let generated = compile(source, "configured.ice").unwrap();
    for expected in [
        ".title(Self::__title)",
        ".theme(Self::__theme).style(Self::__style)",
        "fn __title(&self) -> ::std::string::String",
        "\"dark\" => ::iced::Theme::Dark",
        "fn __style(&self, __theme: &::iced::Theme)",
        "parse::<::iced::Color>()",
        ".executor::<iced::executor::Default>()",
        ".presets([::iced::Preset::new(\"ready\", Self::__preset_0)])",
        "fn __preset_0()",
        "state.ready = true",
        "crate::backend::seed().map(|value| __ConfiguredMessage::Seeded(value))",
        "id: ::std::option::Option::Some(\"dev.example.configured\".to_owned())",
        ".font(include_bytes!(\"fonts/Brand.ttf\").as_slice())",
        ".font(include_bytes!(\"fonts/Icons.otf\").as_slice())",
        "default_text_size: ::iced::Pixels(15 as f32)",
        "antialiasing: false",
        "vsync: false",
        "size: ::iced::Size::new(960 as f32, 720 as f32)",
        "maximized: true",
        "fullscreen: false",
        "Position::Specific(::iced::Point::new(10 as f32, -20 as f32))",
        "min_size: ::std::option::Option::Some(::iced::Size::new(480 as f32, 360 as f32))",
        "max_size: ::std::option::Option::Some(::iced::Size::new(1920 as f32, 1080 as f32))",
        "visible: true",
        "resizable: false",
        "closeable: false",
        "minimizable: false",
        "decorations: false",
        "transparent: true",
        "blur: true",
        "level: ::iced::window::Level::AlwaysOnTop",
        "const __ICE_RGBA: &[u8] = include_bytes!(\"assets/app.rgba\")",
        "__ICE_RGBA.len() == 8",
        "window::icon::from_rgba(__ICE_RGBA.to_vec(), 2, 1)",
        "exit_on_close_request: false",
        "__platform.application_id = \"dev.example.configured\".to_owned()",
        "__platform.override_redirect = true",
        "__platform.drag_and_drop = false",
        "__platform.skip_taskbar = true",
        "__platform.undecorated_shadow = true",
        "CornerPreference::RoundSmall",
        "__platform.title_hidden = true",
        "__platform.titlebar_transparent = true",
        "__platform.fullsize_content_view = true",
        "__platform.target = ::std::option::Option::None",
        "#[cfg(target_os = \"linux\")]",
        "#[cfg(target_os = \"windows\")]",
        "#[cfg(target_os = \"macos\")]",
        "#[cfg(target_arch = \"wasm32\")]",
        ".scale_factor(Self::__scale_factor)",
        "fn __scale_factor(&self) -> f32",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }

    let error = compile(
        &source.replace("ready = true", "ready = 1"),
        "configured.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");

    for (from, to, expected) in [
        ("title \"Configured app\"", "title ready", "expected `str`"),
        ("theme \"dark\"", "theme \"unknown\"", "unknown iced theme"),
        (
            "background \"123456\"",
            "background \"not-a-color\"",
            "hexadecimal",
        ),
        ("scale-factor 1.25", "scale-factor 0", "greater than zero"),
    ] {
        let error = compile(&source.replace(from, to), "configured.ice").unwrap_err();
        assert!(error.message.contains(expected), "{error:?}");
    }
}

#[test]
fn emits_a_probe_for_every_extern_function() {
    let source = r#"app Demo
extern crate::backend
  Item(id:i64)
  AppError(message:str)
  load(id:i64) -> [Item] ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
on mount
  return if false
  run load(1) -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items)
"#;
    let generated = compile(source, "demo.ice").unwrap();
    assert!(generated.contains("async fn __ui_lang_check_load"));
    assert!(generated.contains("crate::backend::load(arg0).await"));
    assert!(generated.contains("let task = (||"));
}

#[test]
fn lowers_structured_task_groups_to_native_combinators() {
    let source = r#"app Grouped
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _
on theme_read(next)
on clipboard_read(next)
on info_read(info)
view
  text "Tasks"
"#;
    let generated = compile(source, "grouped.ice").unwrap();
    assert!(generated.contains("return ::iced::Task::batch(["));
    assert!(generated.contains("::iced::Task::none().chain({"));
    assert!(generated.contains(".chain({ ::iced::system::information()"));
    assert!(generated.contains("fn __ice_system_info"));
}

#[test]
fn lowers_native_task_cancellation() {
    let source = r#"app Cancel
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  request:task-handle? = none
on start
  abortable request abort-on-drop
    task system theme -> loaded _
on loaded(next)
on cancel
  abort request
view
  col
    if aborted(request)
      text "Canceled"
"#;
    let generated = compile(source, "cancel.ice").unwrap();
    assert!(generated.contains("pub(crate) request: ::std::option::Option<::iced::task::Handle>"));
    assert!(generated.contains("let (__task, __handle) = ({"));
    assert!(generated.contains("}).abortable()"));
    assert!(generated.contains("Some(__handle.abort_on_drop())"));
    assert!(generated.contains("__handle.abort()"));
    assert!(generated.contains("is_some_and(::iced::task::Handle::is_aborted)"));
}

#[test]
fn lowers_typed_task_streams() {
    let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream range(start:i64, limit:i64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(id:i64) -> str
  event-filter raw_event() -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
on number(value)
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(3) -> number _
  run range(1, 3) -> number _
  recipe snapshot(3) -> text _
  events 3 using=raw_event -> text _
view
  text "Streams"
"#;
    let generated = compile(source, "streams.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_stream_numbers"));
    assert!(generated.contains("Task::run(crate::backend::numbers(arg0), |value| value)"));
    assert!(generated.contains("Task::run(crate::backend::numbers(3), |value|"));
    assert!(generated.contains("Task::run(crate::backend::fallible(), |result| match result"));
    assert!(generated.contains("Result::Err(error) => __StreamsMessage::Failed(error)"));
    assert!(generated.contains(
            "Subscription::run(crate::backend::fallible).map(move |__value| __StreamsMessage::Observed(__value))"
        ));
    assert!(generated.contains(
        "Subscription::run_with(3, |__data: &i64| crate::backend::numbers(__data.clone()))"
    ));
    assert!(generated.contains(
            "Subscription::run_with((1, 3,), |__data: &(i64, i64,)| crate::backend::range(__data.0.clone(), __data.1.clone()))"
        ));
    assert!(generated.contains("fn __ui_lang_check_recipe_snapshot"));
    assert!(generated.contains(
            "advanced::subscription::from_recipe(crate::backend::snapshot(3)).map(move |__value| __StreamsMessage::Text(__value))"
        ));
    assert!(generated.contains("fn __ui_lang_check_event_filter_raw_event"));
    assert!(generated.contains(
            "advanced::subscription::from_recipe(__IceEventFilterRawEvent { id: 3 }).map(move |__value| __StreamsMessage::Text(__value))"
        ));
}

#[test]
fn lowers_typed_task_sips() {
    let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip transfer(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    sip transfer(3)
      progress -> advanced _
      done -> downloaded _
    sip fallible()
      progress -> counted _
      done -> finished _
      error -> failed _
on advanced(value)
on downloaded(value)
on counted(value)
on finished(value)
on failed(error)
view
  text "Sips"
"#;
    let generated = compile(source, "sips.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_sip_transfer"));
    assert!(generated.contains("let _: f64 = value"));
    assert!(generated.contains("Task::sip(crate::backend::transfer(3), |value|"));
    assert!(generated.contains("Task::sip(crate::backend::fallible(), |value|"));
    assert!(generated.contains("Result::Err(error) => __SipsMessage::Failed(error)"));
}

#[test]
fn lowers_structured_task_flows() {
    let source = r#"app Flows
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
  task fallible(value:i64) -> i64 ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    flow
      from stream numbers(3)
      map value -> value + 1
      then value -> task double(value)
      collect
      done -> collected _
      units -> planned _
    flow
      from task fallible(2)
      map value -> value + 1
      and-then value -> task fallible(value)
      done -> finished _
      error -> failed _
    flow
      from stream numbers(1)
      discard
on collected(values)
on planned(units)
on finished(value)
on failed(error)
view
  text "Flows"
"#;
    let generated = compile(source, "flows.ice").unwrap();
    assert!(generated.contains("Task::run(crate::backend::numbers(3), |value| value)"));
    assert!(generated.contains(".map(move |value| (value + 1))"));
    assert!(generated.contains(".then(move |value| crate::backend::double(value))"));
    assert!(generated.contains(".map(move |result| result.map(|value| (value + 1)))"));
    assert!(generated.contains(".and_then(move |value| crate::backend::fallible(value))"));
    assert!(generated.contains(".collect()"));
    assert!(generated.contains(".discard::<__FlowsMessage>()"));
    assert!(generated.contains("i64::try_from(__task.units())"));
}

#[test]
fn lowers_task_error_mapping_and_native_sources() {
    let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  results:[result[i64,AppError]] = []
on start
  parallel
    flow
      from task request()
      map-error reason -> normalize(reason)
      collect
      done -> collected _
    flow
      from done 1
      then value -> done value + 1
      done -> finished _
    flow
      from none i64
      done -> finished _
on collected(values)
  results = values
on finished(value)
view
  text len(results)
"#;
    let generated = compile(source, "errors.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_sync_normalize"));
    assert!(
        generated.contains(".map_err(move |reason| crate::backend::normalize(reason.clone()))")
    );
    assert!(generated.contains(".collect()"));
    assert!(generated.contains("Task::done(1)"));
    assert!(generated.contains("Task::done((value + 1))"));
    assert!(generated.contains("Task::<i64>::none()"));
    assert!(generated.contains("Vec<::std::result::Result<i64, crate::backend::AppError>>"));
}

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
            "fn __ui_lang_check_themer_alternate_panel(arg0: bool) { let (__theme, __content, __text_color, __background) = crate::backend::alternate_panel(arg0); fn __accept<T: ::iced::theme::Base>(_: &::std::option::Option<T>, _: &::iced::Element<'static, bool, T>"
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
    assert!(generated.contains(
            "format!(\"{}/name\", format!(\"{}/card\", format!(\"{}/Card\", format!(\"{}/editor\", \"Composition\"))))"
        ));
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
    assert!(generated.contains("format!(\"{}/Dialog.Header\""));
    assert!(generated.contains("format!(\"{}/Dialog.Body\""));
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
    assert!(generated.contains("let __lazy_content: ::iced::Element<'static,"));
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

#[test]
fn lowers_complex_native_controls() {
    let source = r#"app Controls
extern crate::backend
  SliderNumber()
  sync slider_number(value:f64) -> SliderNumber
  slider-style dynamic_slider(active:bool)
  progress-style dynamic_progress(active:bool)
  radio-style dynamic_radio(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  precise:SliderNumber = slider_number(50.0)
  enabled = false
  choice = "first"
on amount_changed(next)
  amount = next
on precise_changed(next)
  precise = next
on released
on enabled_changed(next)
  enabled = next
on choice_changed(next)
  choice = next
view
  col
    grid columns=2 width=640.0 spacing=12.0 height=aspect(16.0,9.0) @gap-2
      toggler "Enabled" checked=enabled -> enabled_changed _
      slider amount min=0.0 max=100.0 step=0.5 default=50.0 shift-step=0.1 vertical width=20.0 height=fill(2) style=dynamic_slider(enabled) release=released -> amount_changed _
        active rail-start=linear(0.0, primary@0.0, danger@1.0) rail-end=linear(1.57, background@0.0, primary/25@1.0) rail-width=4.0 rail-border=transparent rail-border-width=1.0 rail-radius=2.0 rail-radius-tl=1.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, foreground@1.0) handle-border=foreground handle-border-width=1.0
        hovered rail-start=foreground rail-end=background handle=rect(12) handle-color=foreground handle-radius=3.0 handle-radius-tl=1.0
        dragged rail-start=danger handle=circle(8.0) handle-color=danger
      slider amount min=0.0 max=100.0 step=1.0 width=fill height=18.0 style=dynamic_slider(enabled) -> amount_changed _
      slider precise min=slider_number(0.0) max=slider_number(100.0) step=slider_number(5.0) default=slider_number(50.0) shift-step=slider_number(1.0) -> precise_changed _
      progress amount vertical length=fill(2) girth=20.0 style=dynamic_progress(enabled) background=linear(1.57, background@0.0, primary/25@1.0) bar=linear(0.0, primary/75@0.0, danger@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0
      progress amount style=success
      progress amount style=warning
      progress amount style=danger
      radio "First" value="first" selected=(choice == "first") style=dynamic_radio(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono -> choice_changed _
        active selected background=linear(1.57, primary@0.0, background@1.0) dot=foreground border=primary border-width=2.0 text=foreground
        active unselected background=background dot=primary border=foreground text=foreground
        hovered selected background=primary dot=foreground border=foreground text=foreground
        hovered unselected background=foreground dot=background border=primary text=primary
      rule horizontal thickness=2.0 style=weak fill=full color=primary/50 radius=4.0 radius-tl=2.0 snap=false
      rule horizontal fill=percent(75.0)
      rule horizontal fill=pad(4)
      rule horizontal fill=pad(4,8)
      space width=fill(2) height=shrink
      stack clip=true width=fill(2) height=120.0 under=1
        text "base"
        text "overlay"
    grid fluid=240.0 height=fill(2)
      text "fluid"
"#;
    let generated = compile(source, "controls.ice").unwrap();
    assert!(
            generated.contains("::iced::widget::grid(__children).spacing(8).spacing(12.0 as f32).width(640.0 as f32).height(::iced::widget::grid::aspect_ratio(16.0 as f32, 9.0 as f32)).columns(2 as usize)")
        );
    assert!(generated.contains(
            "::iced::widget::grid(__children).height(::iced::Length::FillPortion(2)).fluid(240.0 as f32)"
        ));
    assert!(generated.contains("::iced::widget::vertical_slider"));
    assert!(generated.contains(
        ".default(50.0).shift_step(0.1).width(20.0 as f32).height(::iced::Length::FillPortion(2))"
    ));
    assert!(generated.contains("::iced::widget::slider"));
    assert!(generated.contains(".width(::iced::Fill).height(18.0 as f32)"));
    assert!(generated.contains(".style(move |__theme, __status|"));
    assert!(generated.contains("fn __ui_lang_check_slider_style_dynamic_slider"));
    assert_eq!(
        generated
            .matches("crate::backend::dynamic_slider(__theme, __status, self.enabled)")
            .count(),
        2
    );
    assert!(generated.contains(
            "::iced::widget::slider((crate::backend::slider_number(0.0))..=(crate::backend::slider_number(100.0)), self.precise, move |__value| __ControlsMessage::PreciseChanged(__value)).step(crate::backend::slider_number(5.0))"
        ));
    assert!(!generated.contains("self.precise.clone()"));
    assert!(generated.contains("slider::Status::Active"));
    assert!(generated.contains("slider::Status::Hovered"));
    assert!(generated.contains("slider::Status::Dragged"));
    assert!(generated.contains("slider::HandleShape::Circle"));
    assert!(generated.contains("slider::HandleShape::Rectangle"));
    assert!(generated.contains("__style.rail.backgrounds.0"));
    assert!(generated.contains("__style.rail.backgrounds.0 = ::iced::Background::from"));
    assert!(generated.contains("__style.rail.backgrounds.1 = ::iced::Background::from"));
    assert!(generated.contains("__style.handle.background = ::iced::Background::from"));
    assert!(generated.contains("::iced::widget::progress_bar"));
    assert!(generated.contains(".vertical()"));
    assert!(generated.contains(".length(::iced::Length::FillPortion(2)).girth(20.0 as f32)"));
    assert!(generated.contains("crate::backend::dynamic_progress(__theme, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_progress_style_dynamic_progress"));
    assert!(generated.contains("progress_bar::success(__theme)"));
    assert!(generated.contains("progress_bar::warning(__theme)"));
    assert!(generated.contains("progress_bar::danger(__theme)"));
    assert!(generated.contains("__style.background = ::iced::Background::from"));
    assert!(generated.contains("__style.bar = ::iced::Background::from"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("::iced::gradient::Linear::new(0.0 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("::iced::widget::radio(\"First\".to_owned(), true"));
    assert!(generated.contains("move |_| __ControlsMessage::ChoiceChanged(\"first\".to_owned())"));
    assert!(generated.contains(".size(20.0 as f32).spacing(8.0 as f32)"));
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains("crate::backend::dynamic_radio(__theme, __status, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_radio_style_dynamic_radio"));
    for (status, selected) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
    ] {
        assert!(generated.contains(&format!(
            "radio::Status::{status} {{ is_selected: {selected} }}"
        )));
    }
    assert!(generated.contains("__style.background = ::iced::Background::from"));
    assert!(generated.contains("__style.dot_color ="));
    assert!(generated.contains("__style.border_width = 2.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    let default_radio = compile(
        &source.replace(" style=dynamic_radio(enabled)", ""),
        "controls.ice",
    )
    .unwrap();
    assert!(default_radio.contains("radio::default(__theme, __status)"));
    assert!(generated.contains("::iced::widget::rule::weak(__theme)"));
    assert!(generated.contains("rule::FillMode::Full"));
    assert!(generated.contains("rule::FillMode::Percent(75.0 as f32)"));
    assert!(generated.contains("rule::FillMode::Padded(4)"));
    assert!(generated.contains("rule::FillMode::AsymmetricPadding(4, 8)"));
    assert!(generated.contains("__style.snap = false"));
    assert!(generated.contains(
        "::iced::widget::space().width(::iced::Length::FillPortion(2)).height(::iced::Shrink)"
    ));
    assert!(generated.contains("__children.split_off(__under)"));
    assert!(generated.contains("::iced::widget::Stack::new()"));
    assert!(generated.contains("__stack.push_under(__child)"));
    assert!(
        generated
            .contains(".clip(true).width(::iced::Length::FillPortion(2)).height(120.0 as f32)")
    );
}

#[test]
fn lowers_complete_flex_layouts_and_wrapping() {
    let source = r#"app Layouts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  col width=fill height=shrink spacing=8.0 padding=1.0 padding-x=2.0 padding-y=3.0 padding-top=4.0 padding-right=5.0 padding-bottom=6.0 padding-left=7.0 max-width=640.0 align=center clip=true wrap wrap-spacing=12.0 wrap-align=end
    row width=fill(2) height=48.0 spacing=4.0 padding=2.0 align=end clip=false wrap wrap-spacing=6.0 wrap-align=start
      text "One"
      text "Two"
"#;
    let generated = compile(source, "layouts.ice").unwrap();
    assert!(generated.contains("::iced::widget::column(__children).spacing(8.0 as f32)"));
    assert!(generated.contains("::iced::Padding { top: 4.0 as f32, right: 5.0 as f32, bottom: 6.0 as f32, left: 7.0 as f32 }"));
    assert!(generated.contains(".width(::iced::Fill).height(::iced::Shrink)"));
    assert!(generated.contains(".max_width(640.0 as f32)"));
    assert!(generated.contains(
            ".align_x(::iced::alignment::Horizontal::Center).clip(true).wrap().horizontal_spacing(12.0 as f32).align_x(::iced::alignment::Vertical::Bottom)"
        ));
    assert!(generated.contains(".width(::iced::Length::FillPortion(2)).height(48.0 as f32)"));
    assert!(generated.contains(
            ".align_y(::iced::alignment::Vertical::Bottom).clip(false).wrap().vertical_spacing(6.0 as f32).align_x(::iced::alignment::Horizontal::Left)"
        ));
}

#[test]
fn lowers_complete_container_layout() {
    let source = r#"app Boxed
extern crate::backend
  container-style dynamic_container(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  highlight = false
view
  container #card style=dynamic_container(highlight) width=fill height=80.0 max-width=640.0 max-height=120.0 align-x=center align-y=end clip=true padding=8.0 padding-left=12.0 background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true @w-full bg-background border border-foreground rounded-lg
    text "Card"
"#;
    let generated = compile(source, "boxed.ice").unwrap();
    assert!(generated.contains("::iced::widget::container(__container_content)"));
    assert!(generated.contains(".id(::iced::widget::Id::from("));
    assert!(generated.contains(".width(::iced::Fill).height(80.0 as f32)"));
    assert!(generated.contains(".max_width(640.0 as f32).max_height(120.0 as f32)"));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
    assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
    assert!(generated.contains(".clip(true)"));
    assert!(generated.contains("crate::backend::dynamic_container(__theme, self.highlight)"));
    assert!(generated.contains("fn __ui_lang_check_container_style_dynamic_container"));
    assert!(generated.contains("::iced::widget::container::Style"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("__style.shadow.blur_radius = 6.0 as f32"));
    assert!(generated.contains("__style.snap = true"));
    assert!(generated.contains("__style.border.width = 1.0;"));
    assert!(generated.contains("__style.border.width = 2.0 as f32;"));
}

#[test]
fn lowers_structured_overlays_to_native_overlay_widgets() {
    let source = r#"app Dialog
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  shown = true
on close
  shown = false
view
  overlay when=shown dismiss=close backdrop=black/60 padding=24.0 align-x=center align-y=end
    content
      text "Page"
    layer
      container width=320.0 padding=16.0 @bg-background rounded-lg
        text "Dialog"
"#;
    let generated = compile(source, "dialog.ice").unwrap();
    assert!(generated.contains("if self.shown"));
    assert!(generated.contains("::iced::widget::Stack::new()"));
    assert!(generated.contains("::iced::widget::float(__overlay_surface)"));
    assert!(generated.contains("::core::f32::EPSILON"));
    assert!(generated.contains("::iced::Color::from_rgba8(0, 0, 0, 0.600000)"));
    assert!(generated.contains(".on_press(__DialogMessage::Close)"));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
    assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
    assert!(generated.contains("__DialogMessage::__ExternNoop"));
}

#[test]
fn lowers_persistent_pane_grids() {
    let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on clicked(name)
view
  pane-grid #work split=vertical ratio=0.7 width=fill height=fill spacing=8.0 min-size=120.0 resize=6.0 drag click=clicked(_)
    pane files
      text "Files"
    pane editor
      text "Editor"
"#;
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("__pane_work: ::iced::widget::pane_grid::State"));
    assert!(generated.contains("pane_grid::Configuration::Split"));
    assert!(generated.contains("pane_grid::Axis::Vertical"));
    assert!(generated.contains("Configuration::Pane(\"files\")"));
    assert!(generated.contains("::iced::widget::pane_grid(&self.__pane_work"));
    assert!(generated.contains(".on_resize(6.0 as f32, __WorkspaceMessage::__PaneWorkResize)"));
    assert!(generated.contains(".on_drag(__WorkspaceMessage::__PaneWorkDrag)"));
    assert!(generated.contains("self.__pane_work.resize(__event.split, __event.ratio)"));
    assert!(generated.contains("self.__pane_work.drop(pane, target)"));
    assert!(generated.contains("__WorkspaceMessage::Clicked(__pane_name.to_owned())"));
}

#[test]
fn lowers_nested_pane_configuration_and_closed_templates() {
    let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on open_preview
  pane #work split editor preview horizontal ratio=0.4
view
  pane-grid #work width=fill height=fill
    split vertical ratio=0.7
      pane files
        text "Files"
      split horizontal ratio=0.6
        pane editor
          text "Editor"
        pane terminal
          text "Terminal"
    pane preview closed
      text "Preview"
"#;
    let generated = compile(source, "workspace.ice").unwrap();
    assert_eq!(
        generated.matches("pane_grid::Configuration::Split").count(),
        2
    );
    assert!(generated.contains("pane_grid::Axis::Vertical"));
    assert!(generated.contains("pane_grid::Axis::Horizontal"));
    assert!(generated.contains("Configuration::Pane(\"terminal\")"));
    assert!(!generated.contains("Configuration::Pane(\"preview\")"));
    assert!(generated.contains("\"preview\" =>"));
    assert!(generated.contains(".split(::iced::widget::pane_grid::Axis::Horizontal"));
}

#[test]
fn lowers_structured_pane_titles_and_dynamic_controls() {
    let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  filter = ""
on close
view
  pane-grid #work split=vertical
    style
      hovered-region background=linear(0.785, primary/25@0.0, background@0.5, danger@1.0) border=foreground border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
      hovered-split color=primary width=3.0
      picked-split color=danger width=4.0
    pane files background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true @bg-background border border-primary rounded
      title padding=4.0 padding-x=8.0 padding-top=6.0 always-controls background=primary/50 text=foreground border=danger border-width=1.0 radius=3.0 shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=false @bg-primary text-white
        text "Files"
      controls
        button "Close" -> close
      compact-controls
        button "×" -> close
      content
        input "Filter" #filter <-> filter
    pane editor
      title
        text "Editor"
      controls
        button "Close" -> close
      content
        text "Editor body"
"#;
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("pane_grid::Content::new(__pane_content).style"));
    assert!(generated.contains(".title_bar(::iced::widget::pane_grid::TitleBar::new"));
    assert!(generated.contains("top: 6.0 as f32"));
    assert!(generated.contains("right: 8.0 as f32"));
    assert!(generated.contains("bottom: 4.0 as f32"));
    assert!(generated.contains("pane_grid::Controls::dynamic"));
    assert!(generated.contains("pane_grid::Controls::new"));
    assert!(generated.contains(".always_show_controls().style"));
    assert!(generated.contains("__BindFilter"));
    assert!(generated.contains("format!(\"{}/filter\""));
    assert!(generated.contains("pane_grid::default(__theme)"));
    assert!(generated.contains("__style.hovered_region.background"));
    assert!(generated.contains("::iced::gradient::Linear::new(0.785 as f32)"));
    assert!(generated.contains(".add_stop(0.5 as f32"));
    assert!(generated.contains("__style.hovered_region.border.color"));
    assert!(generated.contains("__style.hovered_region.border.width = 2.0 as f32"));
    assert!(generated.contains("top_left: 1.0 as f32"));
    assert!(generated.contains("top_right: 2.0 as f32"));
    assert!(generated.contains("bottom_right: 3.0 as f32"));
    assert!(generated.contains("bottom_left: 4.0 as f32"));
    assert!(generated.contains("__style.hovered_split.color"));
    assert!(generated.contains("__style.hovered_split.width = 3.0 as f32"));
    assert!(generated.contains("__style.picked_split.color"));
    assert!(generated.contains("__style.picked_split.width = 4.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.shadow.color"));
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.shadow.offset.y = 2.0 as f32"));
    assert!(generated.contains("__style.shadow.blur_radius = 6.0 as f32"));
    assert!(generated.contains("__style.snap = true"));
    assert!(generated.contains("__style.snap = false"));
}

#[test]
fn lowers_pane_state_operations_and_queries() {
    let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on arrange
  pane #work maximize editor
  pane #work restore
  pane #work swap files editor
  pane #work move editor left
  pane #work resize 0.6
  pane #work drop editor files top
  pane #work split editor preview horizontal ratio=0.4
  pane #work close editor
on inspect
  pane #work maximized -> observed _
on inspect_neighbor
  pane #work adjacent files right -> observed _
on observed(name)
view
  pane-grid #work split=vertical
    pane files
      text "Files"
    pane editor
      text "Editor"
    pane preview closed
      text "Preview"
"#;
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("self.__pane_work.maximize(__pane)"));
    assert!(generated.contains("self.__pane_work.restore()"));
    assert!(generated.contains("self.__pane_work.swap(__first, __second)"));
    assert!(generated.contains("move_to_edge(__pane, ::iced::widget::pane_grid::Edge::Left)"));
    assert!(generated.contains("layout().splits().next().copied()"));
    assert!(generated.contains("self.__pane_work.resize(__split, (0.6) as f32)"));
    assert!(generated.contains("pane_grid::Target::Pane(__target"));
    assert!(generated.contains("pane_grid::Region::Edge"));
    assert!(generated.contains(".split(::iced::widget::pane_grid::Axis::Horizontal"));
    assert!(generated.contains("\"preview\""));
    assert!(generated.contains("self.__pane_work.close(__pane)"));
    assert!(generated.contains("self.__pane_work.maximized()"));
    assert!(generated.contains("pane_grid::Direction::Right"));
    assert!(generated.contains("::iced::Task::done(__WorkspaceMessage::Observed(value))"));
}

#[test]
fn lowers_list_literals_options_and_pick_lists() {
    let source = r#"app Selection
extern crate::backend
  pick-list-style dynamic_pick(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on opened
on closed
view
  pick choices selected placeholder="Choose" width=fill menu-height=120.0 padding=8.0 text-size=14.0 line-height=1.2 shaping=advanced font=ui open=opened close=closed style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
    active text=foreground placeholder=danger handle=primary background=background border=foreground border-width=1.0 radius=4.0
    hovered text=foreground
    opened text=foreground
    opened-hovered text=foreground
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground border-width=1.0 radius=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
    handle dynamic
      closed code="⌄" font=ui size=12.0 line-height=1.0 shaping=basic
      open code="⌃" font=ui size=13.0 line-height=1.1 shaping=advanced
"#;
    let generated = compile(source, "selection.ice").unwrap();
    assert!(
        generated.contains("pub(crate) selected: ::std::option::Option<::std::string::String>")
    );
    assert!(generated.contains("::std::vec![\"List\".to_owned(), \"Board\".to_owned()]"));
    assert!(
        generated.contains("::iced::widget::pick_list(self.choices.clone(), self.selected.clone()")
    );
    assert!(generated.contains(".on_open(__SelectionMessage::Opened)"));
    assert!(
        generated
            .contains(".text_line_height(::iced::widget::text::LineHeight::Relative(1.2 as f32))")
    );
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains("::iced::widget::pick_list::Handle::Dynamic"));
    assert!(generated.contains(
            "let mut __style = crate::backend::dynamic_pick(__theme, __status, self.busy); match __status"
        ));
    assert!(
        generated.contains("let mut __style = crate::backend::dynamic_menu(__theme, self.busy);")
    );
    assert!(generated.contains("fn __ui_lang_check_pick_list_style_dynamic_pick"));
    assert!(generated.contains("fn __ui_lang_check_menu_style_dynamic_menu"));
    assert!(generated.contains("Status::Opened { is_hovered: false }"));
    assert!(generated.contains("Status::Opened { is_hovered: true }"));
    assert!(generated.contains(".menu_style(move |__theme|"));
    assert!(generated.contains("__style.selected_background"));
    assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
    assert!(generated.contains("self.selected = ::std::option::Option::Some(next);"));
    let defaults = compile(
        &source.replace(
            " style=dynamic_pick(busy) menu-style=dynamic_menu(busy)",
            "",
        ),
        "selection.ice",
    )
    .unwrap();
    assert!(defaults.contains("pick_list::default(__theme, __status)"));
    assert!(defaults.contains("menu::default(__theme)"));
}

#[test]
fn lowers_searchable_combo_boxes() {
    let source = r#"app Search
extern crate::backend
  input-style dynamic_input(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
  query = ""
on selected(next)
  selected = some(next)
on searched(next)
  query = next
on hovered(next)
on opened
on closed
on reset
  modes = ["Timeline"]
on add
  combo modes push "Calendar"
view
  combo modes selected "Search modes" width=fill menu-height=120.0 padding=8.0 text-size=14.0 line-height=1.2 shaping=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground border-width=1.0 radius=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
    icon code="⌕" font=ui size=12.0 spacing=6.0 side=right
"#;
    let generated = compile(source, "search.ice").unwrap();
    assert!(
        generated
            .contains("pub(crate) modes: ::iced::widget::combo_box::State<::std::string::String>")
    );
    assert!(generated.contains(
            "::iced::widget::combo_box::State::new(::std::vec![\"List\".to_owned(), \"Board\".to_owned()])"
        ));
    assert!(generated.contains(
        "::iced::widget::combo_box(&self.modes, \"Search modes\", __combo_selection.as_ref()"
    ));
    assert!(generated.contains(".on_input(move |__value| __SearchMessage::Searched(__value))"));
    assert!(
        generated.contains(".on_option_hovered(move |__value| __SearchMessage::Hovered(__value))")
    );
    assert!(
        generated.contains(".line_height(::iced::widget::text::LineHeight::Relative(1.2 as f32))")
    );
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains("code_point: '⌕'"));
    assert!(generated.contains("Side::Right"));
    assert!(generated.contains(".input_style(move |__theme, __status|"));
    assert!(generated.contains("crate::backend::dynamic_input(__theme, __status, self.busy)"));
    assert!(generated.contains("crate::backend::dynamic_menu(__theme, self.busy)"));
    assert!(generated.contains("fn __ui_lang_check_input_style_dynamic_input"));
    assert!(generated.contains("fn __ui_lang_check_menu_style_dynamic_menu"));
    assert!(generated.contains("Status::Focused { is_hovered: true }"));
    assert!(generated.contains(".menu_style(move |__theme|"));
    assert!(generated.contains("__style.selected_background"));
    assert!(generated.contains(
        "self.modes = ::iced::widget::combo_box::State::new(::std::vec![\"Timeline\".to_owned()]);"
    ));
    assert!(generated.contains("self.modes.push(\"Calendar\".to_owned());"));
}

#[test]
fn lowers_structural_widgets_and_size_events() {
    let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  sensor_key = 0
  width = 0.0
  height = 0.0
on shown(w, h)
  width = w
  height = h
on resized(w, h)
  width = w
  height = h
on hidden
view
  col
    float scale=1.1 x=(viewport_x + viewport_width - original_x - original_width) y=(viewport_y + viewport_height - original_y - original_height) shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 radius=8.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
      text "Floating"
    pin width=fill height=80.0 x=12.0 y=8.0
      text "Pinned"
    sensor show=shown resize=resized hide=hidden key=sensor_key anticipate=32.0 delay=10
      text "Observed"
    responsive at=600.0 width=fill height=40.0
      text "Narrow"
      text "Wide"
    responsive size=(available_width, available_height) width=fill height=fill
      col
        if available_width < available_height
          text "Portrait"
        if available_width >= available_height
          text "Landscape"
"#;
    let generated = compile(source, "structure.ice").unwrap();
    assert!(generated.contains("::iced::widget::float(__float_content).scale(1.1 as f32)"));
    assert!(generated.contains("translate(move |__original, __viewport|"));
    assert!(generated.contains(
            "(((__viewport.x as f64) + (__viewport.width as f64)) - (__original.x as f64)) - (__original.width as f64)"
        ));
    assert!(generated.contains(
            "(((__viewport.y as f64) + (__viewport.height as f64)) - (__original.y as f64)) - (__original.height as f64)"
        ));
    assert!(generated.contains("::iced::widget::float::Style::default()"));
    assert!(generated.contains("__style.shadow.color = ::iced::Color::from_rgba8"));
    assert!(generated.contains("__style.shadow.offset.x = 1.0 as f32"));
    assert!(generated.contains("__style.shadow.offset.y = 2.0 as f32"));
    assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
    assert!(generated.contains("__style.shadow_border_radius = ::iced::border::Radius"));
    assert!(generated.contains("top_left: 1.0 as f32"));
    assert!(generated.contains("top_right: 2.0 as f32"));
    assert!(generated.contains("bottom_right: 3.0 as f32"));
    assert!(generated.contains("bottom_left: 4.0 as f32"));
    assert!(generated.contains("::iced::widget::pin(__pin_content).x(12.0 as f32)"));
    assert!(generated.contains(
            ".on_show(move |__size| __StructureMessage::Shown(__size.width as f64, __size.height as f64))"
        ));
    assert!(generated.contains(".key(self.sensor_key)"));
    assert!(generated.contains("::iced::widget::responsive(move |__size|"));
    assert!(generated.contains("if __size.width < 600.0 as f32"));
    assert!(generated.contains("if ((__size.width as f64) < (__size.height as f64))"));
    assert!(generated.contains("if ((__size.width as f64) >= (__size.height as f64))"));
}

#[test]
fn lowers_configured_scrollables_and_viewport_events() {
    let source = r#"app Scrolling
extern crate::backend
  scroll-style dynamic_scroll(busy:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  absolute_x = 0.0
  absolute_y = 0.0
  relative_x = 0.0
  relative_y = 0.0
on scrolled(ax, ay, rx, ry)
  absolute_x = ax
  absolute_y = ay
  relative_x = rx
  relative_y = ry
on viewport(ax, ay, reversed_x, reversed_y, rx, ry, bx, by, bw, bh, cx, cy, cw, ch)
view
  col
    scroll #feed direction=both width=fill height=200.0 bar=hidden bar-width=8.0 bar-margin=2.0 scroller-width=6.0 bar-spacing=4.0 anchor-x=end anchor-y=start auto=true scroll=scrolled style=dynamic_scroll(busy)
      text "Legacy offsets"
    scroll direction=both width=fill height=200.0 viewport=viewport style=dynamic_scroll(busy)
      col
        text "Complete viewport"
      active horizontal-disabled=false vertical-disabled=false
        container background=background text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
        horizontal-rail background=background border=primary border-width=1.0 radius=2.0
        horizontal-scroller background=primary border=foreground border-width=1.0 radius=2.0
        vertical-rail background=background border=primary border-width=1.0 radius=2.0
        vertical-scroller background=primary border=foreground border-width=1.0 radius=2.0
        gap background=background
        auto background=background border=primary border-width=1.0 radius=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 icon=foreground
      hovered horizontal-hovered=true vertical-hovered=false horizontal-disabled=false vertical-disabled=false
        horizontal-scroller background=foreground
      dragged horizontal-dragged=false vertical-dragged=true horizontal-disabled=false vertical-disabled=false
        vertical-scroller background=danger
"#;
    let generated = compile(source, "scrolling.ice").unwrap();
    assert!(generated.contains("scrollable::Direction::Both"));
    assert!(generated.contains("scrollable::Scrollbar::hidden().width(8.0 as f32)"));
    assert!(generated.contains(".anchor_x(::iced::widget::scrollable::Anchor::End)"));
    assert!(generated.contains(".auto_scroll(true)"));
    assert!(generated.contains("crate::backend::dynamic_scroll(__theme, __status, self.busy)"));
    assert!(generated.contains(
            ".style(move |__theme, __status| crate::backend::dynamic_scroll(__theme, __status, self.busy))"
        ));
    assert!(generated.contains(
            "let mut __style = crate::backend::dynamic_scroll(__theme, __status, self.busy); match __status"
        ));
    assert!(generated.contains("fn __ui_lang_check_scroll_style_dynamic_scroll"));
    assert!(generated.contains("let __absolute = __viewport.absolute_offset()"));
    assert!(generated.contains(
            "__ScrollingMessage::Scrolled(__absolute.x as f64, __absolute.y as f64, __relative.x as f64, __relative.y as f64)"
        ));
    assert!(generated.contains("absolute_offset_reversed()"));
    assert!(generated.contains("let __bounds = __viewport.bounds()"));
    assert!(generated.contains("let __content_bounds = __viewport.content_bounds()"));
    assert!(generated.contains("scrollable::Status::Hovered"));
    assert!(generated.contains("__horizontal_interaction == true"));
    assert!(generated.contains("let __style = &mut __style.container"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.horizontal_rail.scroller.background"));
    assert!(generated.contains("__style.vertical_rail.scroller.background"));
    assert!(generated.contains("__style.gap = ::std::option::Option::Some"));
    assert!(generated.contains("let __style = &mut __style.auto_scroll"));
    assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
    assert!(generated.contains("__style.auto_scroll.icon"));
    let default_scroll = compile(
        &source.replace(" style=dynamic_scroll(busy)", ""),
        "scrolling.ice",
    )
    .unwrap();
    assert!(default_scroll.contains("scrollable::default(__theme, __status)"));
}

#[test]
fn lowers_extended_text_input_behavior() {
    let source = r#"app Form
extern crate::backend
  input-style dynamic_input(disabled:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
  disabled = false
  secure = true
on submitted
on pasted(next)
  value = next
view
  input "Secret" #secret <-> value hint="Paste token" disabled=disabled secure=secure submit=submitted paste=pasted width=240.0 padding=8.0 text-size=14.0 line-height=1.2 align=center font=mono style=dynamic_input(disabled) @bg-background border border-primary rounded-lg focus:border-danger
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    icon code="•" font=ui size=12.0 spacing=4.0 side=right
"#;
    let generated = compile(source, "form.ice").unwrap();
    assert!(generated.contains(".secure(self.secure)"));
    assert!(generated.contains(".width(240.0 as f32).padding(8.0 as f32).size(14.0 as f32)"));
    assert!(generated.contains("LineHeight::Relative(1.2 as f32)"));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
    assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
    assert!(generated.contains("code_point: '•'"));
    assert!(generated.contains("family: ::iced::font::Family::SansSerif"));
    assert!(generated.contains("Side::Right"));
    assert!(generated.contains(".style(move |__theme, __status|"));
    assert!(generated.contains("crate::backend::dynamic_input(__theme, __status, self.disabled)"));
    assert!(generated.contains("fn __ui_lang_check_input_style_dynamic_input"));
    let custom = generated
        .find("crate::backend::dynamic_input(__theme, __status, self.disabled)")
        .unwrap();
    let utility = custom + generated[custom..].find(" __style.background =").unwrap();
    let statuses = utility + generated[utility..].find(" match __status").unwrap();
    assert!(custom < utility && utility < statuses);
    assert!(generated.contains("Status::Focused { is_hovered: true }"));
    assert!(generated.contains("__style.placeholder ="));
    assert!(generated.contains("__style.selection ="));
    assert!(generated.contains(".on_submit_maybe(if self.disabled"));
    assert!(generated.contains(".on_paste_maybe(if self.disabled"));
    let default_input = compile(
        &source.replace(" style=dynamic_input(disabled)", ""),
        "form.ice",
    )
    .unwrap();
    assert!(default_input.contains("text_input::default(__theme, __status)"));
}

#[test]
fn lowers_button_children_and_typed_properties() {
    let source = r#"app Actions
extern crate::backend
  button-style dynamic_button(disabled:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  disabled = false
on pressed
view
  button #action disabled=disabled width=fill height=48.0 padding=8.0 clip=true style=dynamic_button(disabled) @bg-primary text-white rounded-lg disabled:opacity-50 -> pressed
    row
      text "Save"
      text "⌘S"
    active background=linear(1.57, primary@0.0, background@1.0) text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
    hovered background=foreground text=background
    pressed background=primary
    disabled background=background text=foreground
"#;
    let generated = compile(source, "actions.ice").unwrap();
    assert!(generated.contains("let __button_content: ::iced::Element"));
    assert!(generated.contains("::iced::widget::row(__children)"));
    assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
    assert!(generated.contains(".padding(8.0 as f32).clip(true)"));
    assert!(generated.contains(".on_press_maybe(if self.disabled"));
    assert!(generated.contains("crate::backend::dynamic_button(__theme, __status, self.disabled)"));
    assert!(generated.contains("fn __ui_lang_check_button_style_dynamic_button"));
    assert!(generated.contains("button::Status::Active =>"));
    assert!(generated.contains("button::Status::Hovered =>"));
    assert!(generated.contains("button::Status::Pressed =>"));
    assert!(generated.contains("button::Status::Disabled =>"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.snap = true"));
    for preset in [
        "primary",
        "secondary",
        "success",
        "warning",
        "danger",
        "text",
        "background",
        "subtle",
    ] {
        let generated = compile(
            &source.replace("style=dynamic_button(disabled)", &format!("style={preset}")),
            "actions.ice",
        )
        .unwrap();
        assert!(generated.contains(&format!("button::{preset}(__theme, __status)")));
    }
}

#[test]
fn lowers_complete_boolean_control_styles_and_typography() {
    let source = r#"app Preferences
extern crate::backend
  checkbox-style dynamic_checkbox(disabled:bool)
  toggler-style dynamic_toggler(disabled:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  enabled = false
on changed(next)
  enabled = next
view
  col
    checkbox "Checkbox" checked=enabled style=dynamic_checkbox(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono icon="✓" icon-size=12.0 icon-line-height=1.0 icon-shaping=basic -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) icon=foreground text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0
      active unchecked background=background icon=primary text=foreground border=foreground
      hovered checked background=primary icon=foreground text=foreground border=primary
      hovered unchecked background=foreground icon=background text=primary border=primary
      disabled checked background=background icon=foreground text=foreground border=foreground
      disabled unchecked background=background icon=primary text=foreground border=primary
    toggler "Toggler" checked=enabled style=dynamic_toggler(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=auto wrapping=glyph font=default align=right -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) background-border=primary background-border-width=1.0 foreground=linear(0.0, foreground@0.0, primary@1.0) foreground-border=foreground foreground-border-width=2.0 text=foreground radius=7.0 radius-tl=6.0 radius-tr=7.0 radius-br=8.0 radius-bl=9.0 padding-ratio=0.125
      active unchecked background=background foreground=foreground text=primary
      hovered checked background=primary foreground=foreground text=foreground
      hovered unchecked background=foreground foreground=background text=primary
      disabled checked background=background foreground=foreground text=foreground
      disabled unchecked background=background foreground=primary text=foreground
"#;
    let generated = compile(source, "preferences.ice").unwrap();
    assert!(generated.contains(".size(20.0 as f32).spacing(8.0 as f32)"));
    assert!(generated.contains(".width(::iced::Fill)"));
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
    assert!(generated.contains("checkbox::Icon"));
    assert!(generated.contains("code_point: '✓'"));
    assert!(generated.contains(".text_alignment(::iced::widget::text::Alignment::Right)"));
    assert!(
        generated.contains("crate::backend::dynamic_checkbox(__theme, __status, self.enabled)")
    );
    assert!(generated.contains("fn __ui_lang_check_checkbox_style_dynamic_checkbox"));
    for (status, checked) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
        ("Disabled", true),
        ("Disabled", false),
    ] {
        assert!(generated.contains(&format!(
            "checkbox::Status::{status} {{ is_checked: {checked} }}"
        )));
    }
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.icon_color ="));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.border.width = 1.0 as f32"));
    assert!(generated.contains("top_left: 2.0 as f32"));
    for preset in ["primary", "secondary", "success", "danger"] {
        let generated = compile(
            &source.replace(
                "style=dynamic_checkbox(enabled)",
                &format!("style={preset}"),
            ),
            "preferences.ice",
        )
        .unwrap();
        assert!(generated.contains(&format!("checkbox::{preset}(__theme, __status)")));
    }
    assert!(generated.contains("crate::backend::dynamic_toggler(__theme, __status, self.enabled)"));
    assert!(generated.contains("fn __ui_lang_check_toggler_style_dynamic_toggler"));
    for (status, checked) in [
        ("Active", true),
        ("Active", false),
        ("Hovered", true),
        ("Hovered", false),
        ("Disabled", true),
        ("Disabled", false),
    ] {
        assert!(generated.contains(&format!(
            "toggler::Status::{status} {{ is_toggled: {checked} }}"
        )));
    }
    assert!(generated.contains("__style.background_border_width = 1.0 as f32"));
    assert!(generated.contains("__style.foreground = ::iced::Background"));
    assert!(generated.contains("__style.foreground_border_width = 2.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.border_radius = ::std::option::Option::Some"));
    assert!(generated.contains("top_left: 6.0 as f32"));
    assert!(generated.contains("__style.padding_ratio = 0.125 as f32"));
    let generated = compile(
        &source.replace(" style=dynamic_toggler(enabled)", ""),
        "preferences.ice",
    )
    .unwrap();
    assert!(generated.contains("toggler::default(__theme, __status)"));
}

#[test]
fn lowers_full_text_format() {
    let source = r#"app Typography
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Long text" width=fill height=40.0 size=16.0 line-height-px=20.0 font=mono align-x=justified align-y=center shaping=advanced wrapping=word-or-glyph @font-bold
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".width(::iced::Fill).height(40.0 as f32)"));
    assert!(generated.contains("LineHeight::Absolute((20.0 as f32).into())"));
    assert!(generated.contains("text::Alignment::Justified"));
    assert!(generated.contains("alignment::Vertical::Center"));
    assert!(generated.contains("text::Shaping::Advanced"));
    assert!(generated.contains("text::Wrapping::WordOrGlyph"));
    assert!(generated.contains("..::iced::Font::MONOSPACE"));
}

#[test]
fn lowers_native_text_style_callbacks() {
    let source = r#"app Typography
extern crate::backend
  text-style dynamic_text(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  col
    text "Styled" style=dynamic_text(active)
    rich-text style=dynamic_text(active) color=foreground
      span "Rich"
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(
        generated.contains(
            "fn __ui_lang_check_text_style_dynamic_text(theme: &::iced::Theme, arg0: bool)"
        )
    );
    assert_eq!(
        generated
            .matches(".style(move |__theme| crate::backend::dynamic_text(__theme, self.active))")
            .count(),
        2
    );
    assert!(generated.contains(
        ".style(move |__theme| crate::backend::dynamic_text(__theme, self.active)).color("
    ));
}

#[test]
fn lowers_structured_rich_text_spans() {
    let source = r#"app Typography
font ui family=sans weight=medium stretch=normal style=normal
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
on link(url)
view
  rich-text width=fill height=48.0 size=16.0 line-height=1.2 font=ui align-x=justified align-y=center wrapping=word color=foreground @font-bold -> link _
    span "Ice " size=18.0 line-height-px=22.0 font=ui color=primary background=linear(1.57, background@0.0, primary@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 padding=2.0 padding-left=4.0 underline strike=false
    span "language" link="https://example.com" background=background @text-lg font-bold text-primary
"#;
    let generated = compile(source, "rich.ice").unwrap();
    assert!(generated.contains("::iced::widget::rich_text(__rich_spans)"));
    assert!(generated.contains("::iced::widget::span(\"Ice \".to_owned())"));
    assert!(generated.contains(".size(18.0 as f32)"));
    assert!(generated.contains("LineHeight::Absolute((22.0 as f32).into())"));
    assert!(generated.contains(".background(::iced::Background::Color("));
    assert!(generated.contains(".background(::iced::Background::from(::iced::gradient::Linear::new(1.57 as f32).add_stop(0.0 as f32"));
    assert!(generated.contains(".border(::iced::Border"));
    assert!(generated.contains(".padding(::iced::Padding"));
    assert!(generated.contains(".underline(true).strikethrough(false)"));
    assert!(generated.contains(".link(\"https://example.com\".to_owned())"));
    assert!(generated.contains(".on_link_click(move |__link| __TypographyMessage::Link(__link))"));
    assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
    assert!(generated.contains("text::Wrapping::Word"));
}

#[test]
fn lowers_declared_font_descriptors_and_app_default() {
    let source = r#"app Typography
font brand family="Inter" weight=semibold stretch=semi-expanded style=italic default=true
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Brand" font=brand @font-bold
"#;
    let generated = compile(source, "typography.ice").unwrap();
    assert!(generated.contains(".default_font(::iced::Font"));
    assert!(generated.contains("Family::Name(\"Inter\")"));
    assert!(generated.contains("Weight::Semibold"));
    assert!(generated.contains("Stretch::SemiExpanded"));
    assert!(generated.contains("Style::Italic"));
    assert!(generated.contains("weight: ::iced::font::Weight::Bold, ..::iced::Font"));
}

#[test]
fn lowers_typed_iced_extern_boundaries() {
    let source = r#"app Interop
extern crate::backend
  Failure(code:i64)
  component native_meter(value:f64) -> f64
  component passive() -> unit
  shader native_shader(value:f64) -> bool
  shader passive_shader() -> unit
  task focus_next() -> unit
  task save() -> i64 ! Failure
  subscription events() -> bool
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 1.0
  count = 0
  seen = false
on changed(next)
  amount = next
on focused
on focus
  task focus_next() -> focused
on save
  task save() -> saved _ | failed _
on saved(next)
  count = next
on failed(error)
  count = error.code
on event(next)
  seen = next
on shaded(next)
  seen = next
subscribe
  events() -> event _
view
  col
    extern native_meter(amount) -> changed _
    extern passive()
    shader native_shader(amount) width=fill height=64.0 -> shaded _
    shader passive_shader()
    button "Focus" -> focus
    button "Save" -> save
"#;
    let generated = compile(source, "interop.ice").unwrap();
    assert!(generated.contains("::iced::Element<'static, f64>"));
    assert!(generated.contains("::iced::Task<()>"));
    assert!(generated.contains("::iced::Subscription<bool>"));
    assert!(generated.contains("fn __ui_lang_check_shader_native_shader"));
    assert!(generated.contains("::iced::widget::shader::Program<bool>"));
    assert!(
        generated
            .contains("::iced::widget::Shader::new(crate::backend::native_shader(self.amount))")
    );
    assert!(generated.contains(".width(::iced::Fill).height(64.0 as f32)"));
    assert!(generated.contains(".subscription(Self::__subscription)"));
    assert!(generated.contains("native_meter(self.amount).map"));
    assert!(generated.contains("passive().map(move |__value| __InteropMessage::__ExternNoop)"));
    assert!(generated.contains("focus_next().map(|value| __InteropMessage::Focused)"));
    assert!(generated.contains("save().map(|result| match result"));
    assert!(generated.contains("Result::Err(error) => __InteropMessage::Failed(error)"));
}

#[test]
fn lowers_native_keyboard_subscriptions() {
    let source = include_str!("../../../../examples/iced-app/src/ui/keyboard_values.ice");
    let generated = compile(source, "keyboard_values.ice").unwrap();
    assert!(generated.contains("struct __IceKeyPress"));
    assert!(generated.contains("struct __IceKeyRelease"));
    assert!(generated.contains("key: ::iced::keyboard::Key"));
    assert!(generated.contains("physical_key: ::iced::keyboard::key::Physical"));
    assert!(generated.contains("modifiers: ::iced::keyboard::Modifiers"));
    assert!(generated.contains("::iced::keyboard::listen().filter_map"));
    assert!(generated.contains("::iced::keyboard::Event::KeyPressed"));
    assert!(generated.contains("::iced::keyboard::Event::KeyReleased"));
    assert!(generated.contains("::iced::keyboard::Event::ModifiersChanged"));
    assert!(generated.contains("::iced::keyboard::key::Named::Enter"));
    assert!(generated.contains("::iced::keyboard::key::NativeCode::Windows(42u16)"));
    assert!(generated.contains("<u32>::try_from(42).ok().map"));
    assert!(generated.contains("::iced::keyboard::Location::Standard"));
    assert!(generated.contains("::iced::keyboard::Modifiers::SHIFT"));
    assert!(generated.contains("::iced::keyboard::Modifiers::COMMAND"));
    assert!(generated.contains(".to_latin(event.physical_key)"));
    assert!(generated.contains("::iced::keyboard::Key::Character(value)"));
    assert!(generated.contains("::iced::keyboard::key::Physical::Code(value)"));
    assert!(generated.contains("fn __ui_lang_check_sync_keyboard_value"));
}

#[test]
fn lowers_native_timer_subscription() {
    let source = include_str!("../../../../examples/iced-app/src/ui/timer.ice");
    let generated = compile(source, "timer.ice").unwrap();
    assert!(generated.contains("::iced::time::every(::std::time::Duration::from_millis(250))"));
    assert!(generated.contains("if self.auto_refresh { ::iced::Subscription::batch(["));
    assert!(generated.contains("]) } else { ::iced::Subscription::none() }"));
    assert!(generated.contains("::iced::time::now().map"));
    assert!(generated.contains("__TimerEventsMessage::Tick(__value)"));
    assert!(generated.contains(
            "::iced::time::repeat(crate::backend::refresh_time, ::std::time::Duration::from_millis(1000))"
        ));
    assert!(generated.contains(
        ".filter_map(|__value| crate::backend::even_refresh(__value)).with(self.generation)"
    ));
    assert!(generated.contains(
            ".filter_map(|__value| crate::backend::visible_pointer(__value.0, __value.1)).with(self.generation)"
        ));
    assert!(generated.contains(".filter_map(|_| crate::backend::allow_frame())"));
    assert!(generated.contains("__TimerEventsMessage::Refreshed(__value.0, __value.1)"));
}

#[test]
fn lowers_generic_event_values_to_all_native_listeners() {
    let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
  sync event_label(value:event) -> str?
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on received(value)
on labeled(value)
on identified(id, value)
subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id -> identified _ _
view
  text "Events"
"#;
    let generated = compile(source, "events.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_sync_event_name"));
    assert!(generated.contains("arg0: ::iced::Event"));
    assert!(generated.contains("::iced::event::listen().map"));
    assert!(generated.contains("::iced::event::listen_with"));
    assert!(generated.contains("::iced::event::listen_raw"));
    assert!(generated.contains("::iced::event::Status::Ignored"));
    assert!(generated.contains("::iced::event::Status::Captured"));
    assert!(generated.contains("Some((__id, __event))"));
    assert!(generated.contains("filter_map(|__value| crate::backend::event_label(__value))"));
    assert!(generated.contains("__EventsMessage::Received(__value)"));
    assert!(generated.contains("__EventsMessage::Identified(__value.0, __value.1)"));
}

#[test]
fn lowers_a_condition_around_window_frames() {
    let source = include_str!("../../../../examples/iced-app/src/ui/window_events.ice");
    let generated = compile(source, "window_events.ice").unwrap();
    assert!(
        generated.contains(
            "if self.listen_frames { ::iced::Subscription::batch([::iced::window::frames()"
        )
    );
    assert!(generated.contains("]) } else { ::iced::Subscription::none() }"));
    assert!(generated.contains("::iced::Event::Window(__event)"));
    assert!(generated.contains("::iced::event::Status::Captured"));
    assert!(generated.contains("::iced::window::events().filter_map(|(__id, __event)|"));
    assert!(generated.contains("::iced::event::listen_with(|__event, __status, __id|"));
    assert!(generated.contains("(__id, __value.0, __value.1, __value.2, __value.3)"));
    assert!(generated.contains(".map(|_| __id)"));
    assert!(generated.contains(".map(|__value| (__id, __value))"));
    assert!(generated.contains(
        "__WindowEventsMessage::Opened(__value.0, __value.1, __value.2, __value.3, __value.4)"
    ));

    let legacy = source
        .replace("on focused(id)\n  last_window = some(id)", "on focused")
        .replace(
            "window focused with-id -> focused _",
            "window focused -> focused",
        );
    let generated = compile(&legacy, "window_events.ice").unwrap();
    assert!(generated.contains("map(move |__value| __WindowEventsMessage::Focused)"));
}

#[test]
fn lowers_all_native_input_method_subscriptions() {
    let source = include_str!("../../../../examples/iced-app/src/ui/input_method_events.ice");
    let generated = compile(source, "input_method_events.ice").unwrap();
    assert!(generated.contains("::iced::advanced::input_method::Event::Opened"));
    assert!(generated.contains("::iced::advanced::input_method::Event::Preedit"));
    assert!(generated.contains("::iced::advanced::input_method::Event::Commit"));
    assert!(generated.contains("::iced::advanced::input_method::Event::Closed"));
    assert!(generated.contains("i64::try_from(range.start)"));
    assert!(generated.contains("|__event, _, _|"));
}

#[test]
fn lowers_all_native_mouse_subscriptions() {
    let source = include_str!("../../../../examples/iced-app/src/ui/mouse_events.ice");
    let generated = compile(source, "mouse_events.ice").unwrap();
    assert!(generated.contains("::iced::event::listen_with"));
    assert!(generated.contains("::iced::mouse::Event::CursorEntered"));
    assert!(generated.contains("::iced::mouse::Event::CursorLeft"));
    assert!(generated.contains("::iced::mouse::Event::CursorMoved"));
    assert!(generated.contains("::iced::mouse::Event::ButtonPressed"));
    assert!(generated.contains("::iced::mouse::Event::ButtonReleased"));
    assert!(generated.contains("::iced::mouse::Event::WheelScrolled"));
    assert!(generated.contains("::iced::mouse::ScrollDelta::Pixels"));
    assert!(generated.contains("::std::option::Option::Some(button)"));
    assert!(generated.contains("::iced::event::Status::Captured"));
}

#[test]
fn lowers_all_native_touch_subscriptions() {
    let source = include_str!("../../../../examples/iced-app/src/ui/touch_events.ice");
    let generated = compile(source, "touch_events.ice").unwrap();
    assert!(generated.contains("::iced::touch::Event::FingerPressed"));
    assert!(generated.contains("::iced::touch::Event::FingerMoved"));
    assert!(generated.contains("::iced::touch::Event::FingerLifted"));
    assert!(generated.contains("::iced::touch::Event::FingerLost"));
    assert!(generated.contains("::std::option::Option::Some((id, position.x as f64"));
    assert!(generated.contains("::iced::event::Status::Ignored"));
}

#[test]
fn lowers_typed_pointer_values() {
    let source = include_str!("../../../../examples/iced-app/src/ui/pointer_values.ice");
    let generated = compile(source, "pointer_values.ice").unwrap();
    for expected in [
        "Pressed(::iced::mouse::Button)",
        "Touched(::iced::touch::Finger, f64, f64)",
        "::iced::advanced::mouse::Click::new",
        "::iced::mouse::Cursor::Available",
        "::iced::mouse::Button::Other(9u16)",
        "::iced::touch::Finger(18446744073709551615u64)",
        ".position_over(self.bounds)",
        "fn __ui_lang_check_sync_pointer_click",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_transformations() {
    let source = include_str!("../../../../examples/iced-app/src/ui/transformation_values.ice");
    let generated = compile(source, "transformation_values.ice").unwrap();
    for expected in [
        "identity: ::iced::Transformation",
        "translation: ::iced::Vector",
        "size_value: ::iced::Size",
        "::iced::Transformation::orthographic(640u32, 480u32)",
        "<u32>::try_from((-1))",
        "::iced::Transformation::translate",
        "::iced::Transformation::scale",
        ".inverse()",
        "::std::convert::Into::<[f32; 16]>::into",
        "fn __ui_lang_check_sync_transformation_round_trip",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_geometry_values() {
    let source = include_str!("../../../../examples/iced-app/src/ui/geometry_values.ice");
    let generated = compile(source, "geometry_values.ice").unwrap();
    for expected in [
        "snapped_point: ::iced::Point<u32>",
        "exact_bounds: ::iced::Rectangle<u32>",
        "snapped_bounds: ::std::option::Option<::iced::Rectangle<u32>>",
        "::iced::Point::ORIGIN",
        ".distance(::iced::Point::new",
        ".snap()",
        "::iced::Vector::ZERO",
        "::iced::Size::INFINITE",
        ".rotate(::iced::Radians",
        "::iced::Size::from((640u32, 480u32))",
        "<u32>::try_from((-1))",
        "::iced::Rectangle::with_vertices",
        ".intersection(&(::iced::Rectangle",
        "::iced::Padding { top:",
        "(self.bounds).anchor(::iced::Size::new",
        "::iced::alignment::Horizontal::Right",
        "::iced::alignment::Vertical::Bottom",
        "(2.0) as f32",
        "fn __ui_lang_check_sync_geometry_round_trip",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_padding_and_angles() {
    let source = include_str!("../../../../examples/iced-app/src/ui/padding_angles.ice");
    let generated = compile(source, "padding_angles.ice").unwrap();
    for expected in [
        "pixel_value: ::iced::Pixels",
        "direct_padding: ::iced::Padding",
        "degree_value: ::iced::Degrees",
        "radians_value: ::iced::Radians",
        "::iced::Pixels::from(4294967295u32)",
        ".ok().map(::iced::Pixels::from)",
        "::iced::padding::all((5.0) as f32)",
        "::iced::padding::right(::iced::Pixels",
        "::iced::Padding::from([",
        ".fit(::iced::Size::new",
        "::iced::Degrees::RANGE.contains",
        "::iced::Radians::RANGE.contains",
        "::iced::Radians::from(::iced::Degrees",
        ".to_distance(&(::iced::Rectangle",
        " % ",
        "(2.0) as f32 * ::iced::Radians",
        ".rotate(self.radians_value)",
        "fn __ui_lang_check_sync_unit_round_trip",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_system_tasks_and_subscription() {
    let source = r#"app Diagnostics
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  cpu = ""
  mode = "none"
on inspect
  task system info -> inspected _
on inspected(info)
  cpu = info.cpu_brand
on read_theme
  task system theme -> theme_changed _
on theme_changed(next)
  mode = next
subscribe
  system theme -> theme_changed _
view
  text cpu
"#;
    let generated = compile(source, "diagnostics.ice").unwrap();
    assert!(generated.contains("struct __IceSystemInfo"));
    assert!(generated.contains("fn __ice_system_info(value: ::iced::system::Information)"));
    assert!(generated.contains("::iced::system::information().map(__ice_system_info)"));
    assert!(generated.contains("::iced::system::theme().map(__ice_system_theme)"));
    assert!(generated.contains("::iced::system::theme_changes().map(__ice_system_theme)"));
    assert!(generated.contains("self.cpu = info.cpu_brand.clone()"));
}

#[test]
fn lowers_native_clipboard_tasks() {
    let source = r#"app Clipboard
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value:str? = none
on read
  task clipboard read -> read_done _
on read_done(next)
  value = next
on read_primary
  task clipboard read-primary -> read_done _
on write
  task clipboard write "copied"
on write_primary
  task clipboard write-primary "selected"
view
  text "Clipboard"
"#;
    let generated = compile(source, "clipboard.ice").unwrap();
    assert!(generated.contains("::iced::clipboard::read().map"));
    assert!(generated.contains("::iced::clipboard::read_primary().map"));
    assert!(generated.contains("::iced::clipboard::write::<__ClipboardMessage>"));
    assert!(generated.contains("::iced::clipboard::write_primary::<__ClipboardMessage>"));
}

#[test]
fn lowers_native_runtime_font_loading() {
    let source = r#"app Fonts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  font_bytes:bytes = bytes(00 01)
on load
  task font load font_bytes -> loaded _
on loaded(result)
view
  text "Fonts"
"#;
    let generated = compile(source, "fonts.ice").unwrap();
    assert!(generated.contains("::iced::font::load(self.font_bytes.clone()).map"));
    assert!(generated.contains("Result::Ok(value) => __FontsMessage::Loaded(value)"));
    assert!(generated.contains("Result::Err(error) => match error {}"));
}

#[test]
fn lowers_all_static_widget_operations() {
    let source = r#"app Operations
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
on checked(value)
on previous
  task widget focus-previous
on next
  task widget focus-next
on focus
  task widget focus #field
on check
  task widget focused #field -> checked _
on front
  task widget cursor-front #field
on end
  task widget cursor-end #field
on cursor
  task widget cursor #field 2
on all
  task widget select-all #field
on range
  task widget select #field 1 3
on snap
  task widget snap #list 0.0 1.0
on snap_end
  task widget snap-end #list
on scroll_to
  task widget scroll-to #list 0.0 24.0
on scroll_by
  task widget scroll-by #list -4.0 8.0
view
  col
    input "Value" #field <-> value
    scroll #list
      text "Content"
"#;
    let generated = compile(source, "operations.ice").unwrap();
    for function in [
        "focus_previous",
        "focus_next",
        "focus::<",
        "is_focused",
        "move_cursor_to_front",
        "move_cursor_to_end",
        "move_cursor_to::<",
        "select_all",
        "select_range",
        "snap_to::<",
        "snap_to_end",
        "scroll_to::<",
        "scroll_by::<",
    ] {
        assert!(generated.contains(function), "missing {function}");
    }
    assert!(generated.contains("Id::new(\"Operations/field\")"));
    assert!(generated.contains("Id::new(\"Operations/list\")"));
    assert!(generated.contains("RelativeOffset { x: (0.0) as f32, y: (1.0) as f32 }"));
    assert!(generated.contains("AbsoluteOffset"));
    assert!(generated.contains("(-4.0)"));
}

#[test]
fn lowers_all_dynamic_widget_operations() {
    let source = r#"app DynamicOperations
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  ids = [1, 2]
  selected = 1
  value = ""
on checked(value)
on focus
  task widget focus #field(selected)
on check
  task widget focused #field(selected) -> checked _
on front
  task widget cursor-front #field(selected)
on end
  task widget cursor-end #field(selected)
on cursor
  task widget cursor #field(selected) 2
on all
  task widget select-all #field(selected)
on range
  task widget select #field(selected) 1 3
on snap
  task widget snap #list(selected) 0.0 1.0
on snap_end
  task widget snap-end #list(selected)
on scroll_to
  task widget scroll-to #list(selected) 0.0 24.0
on scroll_by
  task widget scroll-by #list(selected) -4.0 8.0
view
  col
    for id in ids
      input "Value" #field(id) <-> value
      scroll #list(id)
        text id
"#;
    let generated = compile(source, "dynamic_operations.ice").unwrap();
    for function in [
        "focus::<",
        "is_focused",
        "move_cursor_to_front",
        "move_cursor_to_end",
        "move_cursor_to::<",
        "select_all",
        "select_range",
        "snap_to::<",
        "snap_to_end",
        "scroll_to::<",
        "scroll_by::<",
    ] {
        assert!(generated.contains(function), "missing {function}");
    }
    assert!(
        generated
            .contains("Id::from(format!(\"{}/field({})\", \"DynamicOperations\", self.selected))")
    );
    assert!(
        generated
            .contains("Id::from(format!(\"{}/list({})\", \"DynamicOperations\", self.selected))")
    );
    assert!(generated.contains(
        ".id(::iced::widget::Id::from(format!(\"{}/field({})\", \"DynamicOperations\", id)))"
    ));
    assert!(generated.contains(
        ".id(::iced::widget::Id::from(format!(\"{}/list({})\", \"DynamicOperations\", id)))"
    ));
}

#[test]
fn lowers_scoped_widget_operations() {
    let source = include_str!("../../../../examples/iced-app/src/ui/scoped_widget_operations.ice");
    let generated = compile(source, "scoped_widget_operations.ice").unwrap();

    for id in [
        "Id::new(\"ScopedOperations/Field/field\")",
        "Id::new(\"ScopedOperations/frame/inner-frame/slot-field\")",
        "Id::new(\"ScopedOperations/details/list\")",
    ] {
        assert!(generated.contains(id), "missing {id}");
    }
    for path in [
        "format!(\"{}/field\", format!(\"{}/inner\", format!(\"{}/outer({})\", \"ScopedOperations\", self.selected)))",
        "format!(\"{}/field\", format!(\"{}/key({})\", \"ScopedOperations\", self.selected))",
        "format!(\"{}/filter\", format!(\"{}/header({})\", \"ScopedOperations\", self.column_index))",
        "format!(\"{}/cell\", format!(\"{}/column({})\", format!(\"{}/row({})\", \"ScopedOperations\", self.row_index), self.column_index))",
    ] {
        assert!(generated.contains(path), "missing {path}");
    }
}

#[test]
fn lowers_widget_selectors() {
    let source = include_str!("../../../../examples/iced-app/src/ui/widget_selectors.ice");
    let generated = compile(source, "widget_selectors.ice").unwrap();

    for expected in [
        "struct __IceWidgetTarget",
        "fn __ice_widget_target_from_target",
        "fn __ice_widget_target_from_text",
        "::iced::widget::selector::find(::iced::widget::selector::id(",
        "::iced::widget::selector::find(\"Search\".to_owned())",
        "::iced::widget::selector::find(::iced::Point::new(",
        "::iced::widget::selector::is_focused()",
        "::iced::widget::selector::find_all(\"Search\".to_owned())",
        "::iced::widget::selector::find_all(crate::backend::by_kind(",
        "fn __ui_lang_check_selector_by_kind",
        ".as_ref().map(|value| value.kind.clone())",
        ".as_ref().map(|value| value.x.clone())",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn checks_and_lowers_main_window_tasks() {
    let source = r#"app WindowTasks
  window child
    size 640 480
    position centered
extern crate::backend
  window describe_window(prefix:str) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on closed
on size_read(width, height)
on bool_read(value)
on optional_bool_read(value)
on optional_pair_read(x, y)
on scale_read(value)
on mode_read(value)
on raw_id_read(value)
on text_read(value)
on screenshot_read(pixels, width, height, scale)
on opened(id)
  task window size target=id -> size_read _ _
on selected(id)
on close_target(id)
  task window close target=id
on open_child
  task window open child -> opened _
on open_default
  task window open -> close_target _
on read_oldest
  task window oldest -> selected _
on read_latest
  task window latest -> selected _
on close_window
  task window close
on drag_window
  task window drag
on drag_resize_window
  task window drag-resize north-east
on resize_window
  task window resize 800.0 600.0
on resizable_window
  task window resizable true
on min_size_window
  task window min-size 320.0 240.0
on clear_min_size
  task window min-size none
on max_size_window
  task window max-size 1920.0 1080.0
on resize_increments_window
  task window resize-increments 8.0 16.0
on read_size
  task window size -> size_read _ _
on read_maximized
  task window maximized -> bool_read _
on maximize_window
  task window maximize true
on read_minimized
  task window minimized -> optional_bool_read _
on minimize_window
  task window minimize false
on read_position
  task window position -> optional_pair_read _ _
on read_scale
  task window scale-factor -> scale_read _
on move_window
  task window move -10.0 20.0
on read_mode
  task window mode -> mode_read _
on mode_window
  task window set-mode fullscreen
on toggle_maximize_window
  task window toggle-maximize
on toggle_decorations_window
  task window toggle-decorations
on attention_window
  task window attention informational
on clear_attention
  task window attention none
on focus_window
  task window focus
on level_window
  task window level always-on-top
on system_menu_window
  task window system-menu
on read_raw_id
  task window raw-id -> raw_id_read _
on capture_window
  task window screenshot -> screenshot_read _ _ _ _
on passthrough_window
  task window mouse-passthrough false
on read_monitor
  task window monitor-size -> optional_pair_read _ _
on automatic_tabbing
  task window automatic-tabbing false
on change_icon
  task window icon bytes(ff 00 00 ff 00 ff 00 ff) 2 1
on describe_window
  task window describe_window("main") -> text_read _
view
  text "Window"
"#;
    let generated = compile(source, "window_tasks.ice").unwrap();
    for function in [
        "window::open",
        "window::oldest",
        "window::latest",
        "window::close",
        "window::drag",
        "window::drag_resize",
        "window::resize",
        "window::set_resizable",
        "window::set_min_size",
        "window::set_max_size",
        "window::set_resize_increments",
        "window::size",
        "window::is_maximized",
        "window::maximize",
        "window::is_minimized",
        "window::minimize",
        "window::position",
        "window::scale_factor",
        "window::move_to",
        "window::mode",
        "window::set_mode",
        "window::toggle_maximize",
        "window::toggle_decorations",
        "window::request_user_attention",
        "window::gain_focus",
        "window::set_level",
        "window::show_system_menu",
        "window::raw_id",
        "window::screenshot",
        "window::enable_mouse_passthrough",
        "window::disable_mouse_passthrough",
        "window::monitor_size",
        "window::allow_automatic_tabbing",
        "window::set_icon",
        "window::run",
    ] {
        assert!(generated.contains(function), "missing {function}");
    }
    assert!(generated.contains("fn __window_0() -> ::iced::window::Settings"));
    assert!(generated.contains("size: ::iced::Size::new(640 as f32, 480 as f32)"));
    assert!(generated.contains("::iced::window::open(Self::__window_0())"));
    assert!(generated.contains("::iced::window::open(::std::default::Default::default())"));
    assert!(generated.contains("::iced::window::size(id).map"));
    assert!(generated.contains("::iced::window::close::<__WindowTasksMessage>(id)"));
    assert!(generated.contains("value.to_string()"));
    assert!(generated.contains("value.rgba.to_vec()"));
    assert!(generated.contains("value.size.width as i64"));
    assert!(generated.contains("value.scale_factor as f64"));
    assert!(generated.contains("window::oldest().and_then"));
    assert!(generated.contains("crate::backend::describe_window(__window, \"main\".to_owned())"));
    assert!(generated.contains("fn __ui_lang_check_window_describe_window"));
    assert!(generated.contains("window: &dyn ::iced::window::Window"));
    assert!(generated.contains("__width.checked_mul(__height).is_some()"));

    let error = compile(
        &source.replacen("task window close\n", "task window close -> closed\n", 1),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");

    let error = compile(
        &source.replace("resize 800.0 600.0", "resize -1.0 600.0"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E128");

    let error = compile(
        &source.replace("size -> size_read _ _", "size -> size_read _"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E129");

    let error = compile(
        &source.replace("task window open child", "task window open missing"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");

    let error = compile(
        &source.replace("task window oldest", "task window oldest target=id"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");

    let error = compile(
        &source.replace("task window size target=id", "task window size target=true"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = compile(
        &source.replace(
            "task window screenshot -> screenshot_read _ _ _ _",
            "task window screenshot -> screenshot_read _ _ _",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E129");

    let error = compile(
        &source.replace(
            "bytes(ff 00 00 ff 00 ff 00 ff) 2 1",
            "bytes(ff 00 00 ff) 2 1",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");
    assert!(error.message.contains("width × height × 4"));

    let error = compile(
        &source.replace(
            "bytes(ff 00 00 ff 00 ff 00 ff) 2 1",
            "bytes(ff 00 00 ff 00 ff 00 ff) 4294967295 2",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");
    assert!(error.message.contains("dimensions are too large"));

    let error = compile(
        &source.replace("describe_window(\"main\")", "describe_window(true)"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = compile(
        &source.replace(
            "describe_window(\"main\") -> text_read _",
            "missing(\"main\") -> text_read _",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E130");
}

#[test]
fn lowers_native_canvas_geometry_cache_and_events() {
    let source = r#"app Drawing
theme
  background #0f172a
  foreground #f8fafc
  primary #7c3aed
  danger #dc2626
state
  cached = true
  picture = rgba(1, 1, bytes(ff 00 ff ff))
on pressed(x, y)
on released(x, y)
on moved(x, y)
on scrolled(x, y, pixels)
on entered
on exited
view
  canvas width=fill height=240.0 cache=cached cache-group=drawings capture=true cursor=crosshair press=pressed release=released move=moved scroll=scrolled enter=entered exit=exited
    rect x=0.0 y=0.0 width=canvas_width height=canvas_height fill=linear(1.57, background@0.0, primary@1.0) stroke=foreground
    rect x=8.0 y=8.0 width=72.0 height=40.0 radius=8.0 radius-tl=4.0 stroke=foreground stroke-width=2.0 dash=(4.0, 2.0) dash-offset=1 cap=round join=bevel
    circle x=120.0 y=60.0 radius=24.0 fill=primary fill-rule=even-odd stroke=foreground
    line x1=16.0 y1=120.0 x2=200.0 y2=120.0 stroke=foreground stroke-width=3.0 cap=square
    text "Canvas" x=16.0 y=150.0 max-width=180.0 color=foreground size=18.0 line-height=1.2 font=default align-x=left align-y=top shaping=advanced
    image picture x=8.0 y=160.0 width=32.0 height=24.0 filter=nearest rotation=0.2 opacity=0.8 snap=true radius=4.0 radius-tl=2.0
    svg "<svg/>" memory x=48.0 y=160.0 width=24.0 height=24.0 color=foreground rotation=0.1 opacity=0.9
    path fill=primary stroke=foreground stroke-width=1.0
      move x=220.0 y=20.0
      line x=260.0 y=20.0
      arc x=260.0 y=40.0 radius=20.0 start=0.0 end=3.14
      arc-to ax=280.0 ay=60.0 bx=300.0 by=40.0 radius=8.0
      ellipse x=320.0 y=40.0 radius-x=20.0 radius-y=10.0 rotation=0.2 start=0.0 end=6.28
      bezier ax=340.0 ay=10.0 bx=360.0 by=70.0 x=380.0 y=40.0
      quadratic cx=400.0 cy=10.0 x=420.0 y=40.0
      rect x=220.0 y=80.0 width=30.0 height=20.0
      rounded x=260.0 y=80.0 width=30.0 height=20.0 radius=4.0
      circle x=320.0 y=90.0 radius=10.0
      close
    group x=10.0 y=10.0 rotate=0.1 scale=1.1 scale-x=1.0 scale-y=0.9 clip=(0.0, 0.0, 100.0, 100.0)
      circle x=20.0 y=20.0 radius=10.0 fill=foreground
    if cached
      circle x=360.0 y=180.0 radius=12.0 fill=primary
    for value in [12.0, 24.0]
      circle x=value y=210.0 radius=4.0 fill=foreground
"#;
    let generated = compile(source, "drawing.ice").unwrap();
    for expected in [
        "impl<State, Message, Draw, Update, Interaction> ::iced::widget::canvas::Program<Message>",
        "__state.cache.get_or_init",
        "Cache::with_group",
        "__ICE_CANVAS_GROUP_DRAWINGS",
        "::std::hash::Hash::hash",
        "::iced::widget::canvas::Path::rounded_rectangle",
        "__frame.fill_rectangle",
        "__frame.stroke_rectangle",
        "__frame.fill_text",
        "__frame.draw_image",
        "__frame.draw_svg",
        "::iced::advanced::svg::Svg",
        "__path.arc(",
        "__path.arc_to(",
        "__path.ellipse(",
        "__path.bezier_curve_to(",
        "__path.quadratic_curve_to(",
        "__frame.with_save",
        "__frame.with_clip",
        "__frame.scale_nonuniform",
        "::iced::mouse::Interaction::Crosshair",
        "::iced::widget::canvas::Action::publish",
        ".and_capture()",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_canvas_event_and_redraw_action() {
    let source = include_str!("../../../../examples/iced-app/src/ui/canvas_events.ice");
    let generated = compile(source, "canvas_events.ice").unwrap();
    for expected in [
        "Event::InputMethod",
        "Event::Keyboard",
        "Event::Mouse",
        "Event::Touch",
        "Event::Window",
        "struct __IceKeyPress",
        "::iced::mouse::Button",
        "KeyPressed",
        "KeyReleased",
        "ModifiersChanged",
        "CursorEntered",
        "CursorLeft",
        "CursorMoved",
        "ButtonPressed",
        "ButtonReleased",
        "WheelScrolled",
        "FingerPressed",
        "FingerMoved",
        "FingerLifted",
        "FingerLost",
        "RedrawRequested",
        "CloseRequested",
        "FileHovered",
        "FileDropped",
        "FilesHoveredLeft",
        "Action::publish",
        "Action::capture",
        "Action::request_redraw()",
        "Action::request_redraw_at",
        "Duration::from_millis(16)",
        ".and_capture()",
        "move_count: i64",
        "__state.move_count =",
        "fn __ice_canvas_interaction",
        "__ice_canvas_interaction(__interaction.as_str())",
        "__cursor.is_over(__bounds)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_media_tooltip_and_pointer_events() {
    let source = r#"app Media
extern crate::backend
  svg-style dynamic_svg(active:bool)
  container-style dynamic_tooltip(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
  encoded_image = encoded(bytes(50 36 0a))
  rgba_image = rgba(1, 1, bytes(ff 00 00 ff))
on entered
on exited
on pressed
on moved(x, y)
on scrolled(x, y, pixels)
view
  col
    image "photo.ppm" width=fill height=64.0 fit=cover filter=nearest rotation=solid(0.5) opacity=0.8 scale=1.2 expand=true radius=4.0 radius-tl=1.0 radius-br=2.0 crop=(1, 2, 30, 40)
    image encoded_image
    image rgba_image
    viewer encoded_image width=fill(2) height=120.0 fit=contain filter=linear padding=8.0 min-scale=0.5 max-scale=4.0 scale-step=0.25
    viewer "photo.ppm" width=64.0 height=64.0
    svg "icon.svg" width=48.0 height=shrink fit=scale-down rotation=0.1 opacity=0.9 color=foreground hover=primary style=dynamic_svg(active)
    svg "<svg/>" memory width=16.0 color=foreground hover=none
    svg bytes(3c 73 76 67 2f 3e) memory width=16.0
    tooltip position=cursor gap=2.0 padding=5.0 delay=100 snap=false style=dynamic_tooltip(active) background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary/75 border-width=1.0 radius=5.0 radius-tl=2.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true
      mouse enter=entered exit=exited press=pressed move=moved scroll=scrolled cursor=pointer
        text "Hover"
      text "Tip"
"#;
    let generated = compile(source, "media.ice").unwrap();
    assert!(generated.contains("::iced::widget::image(\"photo.ppm\".to_owned())"));
    assert!(generated.contains(".rotation(::iced::Rotation::Solid(::iced::Radians(0.5 as f32)))"));
    assert!(generated.contains(".border_radius(::iced::border::Radius { top_left: 1.0 as f32, top_right: 4.0 as f32, bottom_right: 2.0 as f32, bottom_left: 4.0 as f32 })"));
    assert!(generated.contains("image::Handle::from_bytes(::std::vec![0x50u8, 0x36u8, 0x0au8])"));
    assert!(generated.contains("image::Handle::from_rgba((1).clamp(0, u32::MAX as i64) as u32, (1).clamp(0, u32::MAX as i64) as u32, ::std::vec![0xffu8, 0x00u8, 0x00u8, 0xffu8])"));
    assert!(generated.contains("::iced::widget::image::viewer(self.encoded_image.clone()).width(::iced::Length::FillPortion(2)).height(120.0 as f32).content_fit(::iced::ContentFit::Contain).filter_method(::iced::widget::image::FilterMethod::Linear).padding(8.0 as f32).min_scale(0.5 as f32).max_scale(4.0 as f32).scale_step(0.25 as f32)"));
    assert!(generated.contains("::iced::widget::image::viewer(::iced::widget::image::Handle::from_path(\"photo.ppm\".to_owned()))"));
    assert!(generated.contains(".crop(::iced::Rectangle { x: (1).clamp(0, u32::MAX as i64) as u32, y: (2).clamp(0, u32::MAX as i64) as u32, width: (30).clamp(0, u32::MAX as i64) as u32, height: (40).clamp(0, u32::MAX as i64) as u32 })"));
    assert!(generated.contains(".filter_method(::iced::widget::image::FilterMethod::Nearest)"));
    assert!(generated.contains("::iced::widget::svg(\"icon.svg\".to_owned())"));
    assert!(generated.contains("svg::Handle::from_memory((\"<svg/>\".to_owned()).into_bytes())"));
    assert!(generated.contains(
        "svg::Handle::from_memory(::std::vec![0x3cu8, 0x73u8, 0x76u8, 0x67u8, 0x2fu8, 0x3eu8])"
    ));
    assert!(generated.contains("crate::backend::dynamic_svg(__theme, __status, self.active)"));
    assert!(generated.contains("fn __ui_lang_check_svg_style_dynamic_svg"));
    assert!(generated.contains("svg::Status::Idle => __style.color = Some(::iced::Color"));
    assert!(generated.contains("svg::Status::Hovered => __style.color = Some(::iced::Color"));
    assert!(generated.contains("svg::Status::Hovered => __style.color = None"));
    let default_svg = compile(
        &source.replace(" style=dynamic_svg(active)", ""),
        "media.ice",
    )
    .unwrap();
    assert!(default_svg.contains("let mut __style = ::iced::widget::svg::Style::default()"));
    assert!(generated.contains("tooltip::Position::FollowCursor"));
    assert!(generated.contains(".delay(::std::time::Duration::from_millis(100 as u64))"));
    assert!(generated.contains("crate::backend::dynamic_tooltip(__theme, self.active)"));
    let preset_tooltip = compile(
        &source.replace("style=dynamic_tooltip(active)", "style=success"),
        "media.ice",
    )
    .unwrap();
    assert!(preset_tooltip.contains("container::success(__theme)"));
    assert!(generated.contains("__style.background = Some("));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.shadow.blur_radius = 8.0 as f32"));
    assert!(generated.contains("__style.snap = true"));
    assert!(generated.contains(".on_enter(__MediaMessage::Entered)"));
    assert!(generated.contains(
        ".on_move(move |__point| __MediaMessage::Moved(__point.x as f64, __point.y as f64))"
    ));
    assert!(generated.contains("::iced::mouse::ScrollDelta::Lines"));
    assert!(generated.contains("__MediaMessage::Scrolled(__x as f64, __y as f64, true)"));
    assert!(generated.contains(".interaction(::iced::mouse::Interaction::Pointer)"));
}
