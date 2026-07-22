use super::*;

#[test]
fn checks_optional_selection_values() {
    let source = r#"app Demo
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
view
  pick choices selected placeholder="Choose" line-height=1.2 shaping=advanced font=ui open=opened style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
    active text=foreground placeholder=danger handle=primary background=background border=foreground border-width=1.0 radius=4.0
    hovered text=foreground
    opened text=foreground
    opened-hovered text=foreground
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground shadow=danger shadow-y=2.0
    handle dynamic
      closed code="⌄" font=ui size=12.0 line-height=1.0 shaping=basic
      open code="⌃" font=ui size=12.0 line-height=1.0 shaping=advanced
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[1].ty.display(), "[str]");
    assert_eq!(document.states[2].ty.display(), "str?");
    assert_eq!(document.handlers[0].params[0].ty.display(), "str");

    let error = analyze(&source.replace("size=12.0", "size=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("icon size"));

    let error = analyze(&source.replace("dynamic_pick(busy)", "missing(busy)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("pick-list style"));

    let error = analyze(&source.replace("dynamic_menu(busy)", "missing(busy)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("menu style"));

    let error = analyze(&source.replace("dynamic_pick(busy)", "dynamic_pick(1.0)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("style=dynamic_pick(busy)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E087");
    assert!(error.message.contains("declared style call"));
}

#[test]
fn rejects_a_non_optional_pick_selection() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  choices = ["List", "Board"]
  selected = "List"
on selected(next)
  selected = next
view
  pick choices selected -> selected _
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("optional"));
}

#[test]
fn checks_qr_declarations_and_references() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
qr code "hello" version=micro(0)
view
  qr code
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E136");
    assert!(error.message.contains("micro(1..4)"));

    let source = source.replace(
        "qr code \"hello\" version=micro(0)",
        "qr saved \"hello\" version=micro(4)",
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E136");
    assert!(error.message.contains("unknown qr data `code`"));
}

#[test]
fn rejects_unknown_nested_theme_colors() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  theme dark text=missing
    text "Hello"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E137");
    assert!(error.message.contains("missing"));

    let source = source.replace(
        "theme dark text=missing",
        "theme dark background=linear(1.57, background@0.0, missing@1.0)",
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E137");
    assert!(error.message.contains("missing"));
}

#[test]
fn checks_component_slot_contracts() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
component Card(title:str, padded:bool)
  col
    text title
    slot
view
  Card padded=true title="Editor"
    input "Name" <-> draft
"#;
    analyze(source).unwrap();
    analyze(&source.replace(
        "Card padded=true title=\"Editor\"",
        "Card(\"Editor\", true)",
    ))
    .unwrap();

    let error = analyze(&source.replace(
        "  Card padded=true title=\"Editor\"\n    input \"Name\" <-> draft",
        "  Card padded=true title=\"Editor\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("requires slot `children`"));

    let error = analyze(&source.replace("    text title\n    slot", "    text title")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `children`"));

    let error = analyze(&source.replace("padded=true ", "")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("missing prop `padded`"));

    let error = analyze(&source.replace("padded=true", "raised=true")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("no prop `raised`"));

    let error = analyze(&source.replace("padded=true", "title=\"Again\"")).unwrap_err();
    assert_eq!(error.code, "E123");
    assert!(error.message.contains("prop `title` more than once"));

    let error = analyze(&source.replace("title=\"Editor\"", "title=true")).unwrap_err();
    assert!(error.message.contains("expected `str`, got `bool`"));

    let error = analyze(&source.replace("padded:bool", "title:bool")).unwrap_err();
    assert_eq!(error.code, "E100");
    assert!(error.message.contains("duplicate component prop `title`"));
}

#[test]
fn checks_named_component_slots() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog(title:str)
  col
    slot header
    text title
    slot body
    slot actions
on cancel
on delete
view
  Dialog title="Delete task?"
    header:
      text "Danger zone"
    body:
      col
        text "This cannot be undone."
    actions:
      row
        button "Cancel" -> cancel
        button "Delete" -> delete
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace(
            "    actions:\n      row\n        button \"Cancel\" -> cancel\n        button \"Delete\" -> delete\n",
            "",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("requires slot `actions`"));

    let error = analyze(&source.replace("    actions:", "    footer:")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `footer`"));

    let error = analyze(&source.replace(
        "    body:\n      col\n        text \"This cannot be undone.\"",
        "    body:\n      text \"First\"\n      text \"Second\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("slot `body` needs exactly one root"));

    let error = analyze(&source.replace("    slot actions", "    slot body")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(
        error
            .message
            .contains("declares slot `body` more than once")
    );
}

#[test]
fn checks_compound_component_slots() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog()
  col
    slot Header
    slot Body
    slot Actions
component Dialog.Header(title:str)
  col
    text title
    slot
component Dialog.Body()
  container
    slot
component Dialog.Actions()
  row
    slot
on close
view
  Dialog
    Dialog.Header title="About"
      text "Compound title"
    Dialog.Body
      text "Structured body"
    Dialog.Actions
      button "Close" -> close
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("    slot Actions\n", "")).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(error.message.contains("does not declare slot `Actions`"));

    let error = analyze(&source.replace(
        "    Dialog.Actions\n      button \"Close\" -> close",
        "    text \"not compound\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("cannot mix compound components"));

    let error = analyze(&source.replace("Dialog.Header", "Dialog..Header")).unwrap_err();
    assert_eq!(error.code, "E072");
    assert!(error.message.contains("invalid component name"));
}

#[test]
fn checks_keyed_columns_and_copyable_keys() {
    let source = r#"app Demo
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
  keyed item in items by=item.id width=fill height=shrink spacing=8.0 padding=4.0 max-width=640.0 align=center
    text item.name
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("by=item.id", "by=item.name")).unwrap_err();
    assert_eq!(error.code, "E138");
    assert!(error.message.contains("bool, i64, or f64"));

    let error = analyze(&source.replace("spacing=8.0", "spacing=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));
}

#[test]
fn checks_lazy_static_boundaries() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  title = "Hello"
  other = "Outside"
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("text len(cached)", "text other")).unwrap_err();
    assert_eq!(error.code, "E150");
    assert!(error.message.contains("unknown value `other`"));

    let error = analyze(&source.replace("title = \"Hello\"", "title = 1.0")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("stable hashing"));

    let error =
        analyze(&source.replace("text len(cached)", "input \"Edit\" <-> cached")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("borrows app state"));

    let component_source = source.replace(
            "view\n  lazy title as cached\n    col\n      text cached\n      text len(cached)",
            "component Editor(value:str)\n  input \"Edit\" <-> value\nview\n  lazy title as cached\n    Editor(cached)",
        );
    let error = analyze(&component_source).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("borrows app state"));
}

#[test]
fn checks_markdown_content_settings_and_links() {
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
  docs:markdown = "# Hello [world](https://example.com)"
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
    let document = analyze(source).unwrap();
    assert_eq!(document.states[0].ty.display(), "markdown");
    assert_eq!(document.handlers[0].params[0].ty.display(), "str");

    let error = analyze(&source.replace("spacing=12.0", "spacing=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));

    let error = analyze(&source.replace("markdown docs", "markdown missing")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("unknown markdown state"));

    let error =
        analyze(&source.replace("markdown docs append", "markdown missing append")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("unknown markdown state"));

    let error = analyze(&source.replace(
        "markdown docs append \"\\n![Ice](asset://ice)\"",
        "markdown docs append true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("viewer=docs_viewer", "viewer=missing")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("markdown viewer"));

    let error = analyze(&source.replace("link=primary", "link=missing")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("markdown link"));

    let error =
        analyze(&source.replace("markdown_images(docs)", "markdown_images(true)")).unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn checks_structured_tables_and_metrics() {
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
    column width=fill(2) align-x=left align-y=center
      header
        text "Name"
      cell
        text row.name
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("padding=4.0", "padding=-1.0")).unwrap_err();
    assert!(error.message.contains("outside its valid range"));

    let error = analyze(&source.replace("table row in rows", "table row in true")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("list of rows"));
}

#[test]
fn checks_bound_text_editors_and_highlighting() {
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
  editor #body <-> body placeholder="Write" width=640.0 height=fill min-height=80.0 max-height=240.0 size=14.0 line-height=1.3 padding=8.0 wrapping=word-or-glyph font=mono highlight="rs" highlight-theme=solarized-dark disabled=locked
    active background=background border=foreground border-width=1.0 radius=4.0 placeholder=danger value=foreground selection=primary
    hovered background=background border=primary placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[0].ty.display(), "editor");

    let error = analyze(&source.replace("min-height=80.0", "min-height=300.0")).unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(error.message.contains("cannot exceed"));

    let error = analyze(&source.replace("placeholder=danger", "icon=danger")).unwrap_err();
    assert_eq!(error.code, "E099");
    assert!(error.message.contains("unknown editor style property"));
}

#[test]
fn checks_component_controlled_state_origins() {
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
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[0].params[0].ty.display(), "EditorCommand");

    let error = analyze(&source.replace(
        "EditorPanel(body, title, locked, language)",
        "EditorPanel(editor(\"scratch\"), title, locked, language)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E139");
    assert!(
        error
            .message
            .contains("editor binding must resolve to an app state")
    );

    let error = analyze(&source.replace("editor_keys(readonly)", "missing(readonly)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor binding"));

    let error =
        analyze(&source.replace("editor_highlight(syntax)", "missing(syntax)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor highlighter"));

    let error =
        analyze(&source.replace("editor_surface(readonly)", "missing(readonly)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("editor style"));
}

#[test]
fn checks_component_scoped_state_and_handlers() {
    let source = r#"app Local
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Toggle()
  state
    enabled = false
  on changed(next)
    enabled = next
  col
    checkbox "Enabled" checked=enabled -> changed _
view
  Toggle #first
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.components[0].states[0].ty, Type::Bool);
    assert_eq!(document.components[0].handlers[0].params[0].ty, Type::Bool);

    let error = analyze(&source.replace("enabled = false", "enabled = missing")).unwrap_err();
    assert_eq!(error.code, "E031");

    let nested_owned = source.replace(
        "enabled = false",
        "enabled = false\n    handles:[task-handle?] = []",
    );
    let error = analyze(&nested_owned).unwrap_err();
    assert_eq!(error.code, "E103");
    assert!(error.message.contains("cloneable values"));

    let error = analyze(&source.replace("    enabled = false\n", "")).unwrap_err();
    assert_eq!(error.code, "E040");
    assert!(error.message.contains("state cannot be empty"));

    let error =
        analyze(&source.replace("enabled = next", "task system theme -> changed _")).unwrap_err();
    assert_eq!(error.code, "E140");
}

#[test]
fn rejects_slots_outside_components_and_duplicate_slots() {
    let outside = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  slot
"#;
    let error = analyze(outside).unwrap_err();
    assert_eq!(error.code, "E124");
    assert_eq!(error.line, 8);

    let duplicate = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Card()
  col
    slot
    slot
view
  text "Hello"
"#;
    let error = analyze(duplicate).unwrap_err();
    assert_eq!(error.code, "E124");
    assert!(
        error
            .message
            .contains("declares slot `children` more than once")
    );
}

#[test]
fn checks_combo_search_state_and_routes() {
    let source = r#"app Demo
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
on add
  combo modes push "Timeline"
view
  combo modes selected "Search modes" line-height=1.2 shaping=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground shadow=danger shadow-y=2.0
    icon code="⌕" font=ui size=12.0 spacing=6.0 side=right
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.states[1].ty.display(), "combo[str]");
    assert_eq!(document.handlers[0].params[0].ty.display(), "str");
    assert_eq!(document.handlers[1].params[0].ty.display(), "str");
    assert_eq!(document.handlers[2].params[0].ty.display(), "str");

    let error = analyze(&source.replace("spacing=6.0", "spacing=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("icon spacing"));

    let error = analyze(&source.replace("combo modes push", "combo missing push")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("unknown combo state"));

    let error = analyze(&source.replace("combo modes push", "combo selected push")).unwrap_err();
    assert_eq!(error.code, "E140");
    assert!(error.message.contains("not combo state"));

    let error = analyze(&source.replace("push \"Timeline\"", "push 1")).unwrap_err();
    assert_eq!(error.code, "E101");
}

#[test]
fn replaces_combo_search_options_with_a_typed_list() {
    let source = r#"app Demo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
on reset
  modes = ["Timeline"]
on selected(next)
  selected = some(next)
view
  combo modes selected "Search modes" -> selected _
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("[\"Timeline\"]", "[1]")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `[str]`, got `[i64]`"));
}

#[test]
fn checks_structural_widget_routes_and_ranges() {
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
    stack width=fill(2) height=120.0 clip=true under=1
      text "Base"
      text "Overlay"
    rule horizontal thickness=2.0 style=weak fill=percent(75.0) color=primary/50 radius=4.0 radius-tl=2.0 snap=false
    space width=fill(2) height=shrink
"#;
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[0].params[0].ty.display(), "f64");
    assert_eq!(document.handlers[0].params[1].ty.display(), "f64");
    assert_eq!(document.handlers[1].params[0].ty.display(), "f64");

    let bad_float_translation = source.replace(
        "x=(viewport_x + viewport_width - original_x - original_width)",
        "x=true",
    );
    let error = analyze(&bad_float_translation).unwrap_err();
    assert!(error.message.contains("expected `f64`, got `bool`"));

    let bad_float_blur = source.replace("shadow-blur=4.0", "shadow-blur=-1.0");
    let error = analyze(&bad_float_blur).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("float style metric"));

    let bad_float_color = source.replace("shadow=black/50", "shadow=missing");
    let error = analyze(&bad_float_color).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("unknown float shadow color"));

    let bad_stack = source.replace("height=120.0 clip=true", "height=-1.0 clip=true");
    let error = analyze(&bad_stack).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("stack size"));

    let bad_under = source.replace("under=1", "under=70000");
    let error = analyze(&bad_under).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("stack under"));

    let duplicate_size_name = source.replace(
        "size=(available_width, available_height)",
        "size=(available_width, available_width)",
    );
    let error = analyze(&duplicate_size_name).unwrap_err();
    assert_eq!(error.code, "E092");
    assert!(error.message.contains("different names"));

    let conflicting_responsive = source.replace(
        "responsive size=(available_width, available_height)",
        "responsive at=600.0 size=(available_width, available_height)",
    );
    let error = analyze(&conflicting_responsive).unwrap_err();
    assert_eq!(error.code, "E092");
    assert!(error.message.contains("either `at=` or `size=`"));
}
