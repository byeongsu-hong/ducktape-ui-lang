use super::*;

#[path = "widgets/content.rs"]
mod content;
#[path = "widgets/controls.rs"]
mod controls;
#[path = "widgets/layout.rs"]
mod layout;

#[test]
fn rejects_proven_same_builder_style_collisions() {
    let header = r#"app Demo
extern crate::backend
  container-style container_base()
  input-style input_base()
  button-style button_base()
  text-style text_base()
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
on pressed
view
"#;
    for (property, owner, utility, view) in [
        (
            "width",
            "width=",
            "w-full",
            "container width=fill @w-full\n    text \"x\"",
        ),
        (
            "height",
            "height=",
            "h-full",
            "container height=fill @h-full\n    text \"x\"",
        ),
        (
            "max-width",
            "max-width=",
            "max-w-md",
            "container max-width=320.0 @max-w-md\n    text \"x\"",
        ),
        (
            "padding",
            "padding=",
            "p-2",
            "container padding=8.0 @p-2\n    text \"x\"",
        ),
        (
            "background",
            "background=",
            "bg-primary",
            "container background=background @bg-primary\n    text \"x\"",
        ),
        (
            "text color",
            "text=",
            "text-primary",
            "container text=foreground @text-primary\n    text \"x\"",
        ),
        (
            "border width",
            "border-width=",
            "border",
            "container border-width=2.0 @border\n    text \"x\"",
        ),
        (
            "border color",
            "border=",
            "border-danger",
            "container border=primary @border border-danger\n    text \"x\"",
        ),
        (
            "radius",
            "radius=",
            "rounded",
            "container radius=4.0 @border rounded\n    text \"x\"",
        ),
        (
            "width",
            "width=",
            "w-full",
            "scroll width=fill @w-full\n    text \"x\"",
        ),
        (
            "spacing",
            "spacing=",
            "gap-2",
            "col spacing=8.0 @gap-2\n    text \"x\"",
        ),
        (
            "padding",
            "padding=",
            "px-2",
            "row padding=8.0 @px-2\n    text \"x\"",
        ),
        (
            "alignment",
            "align=",
            "items-center",
            "row align=center @items-center\n    text \"x\"",
        ),
        (
            "spacing",
            "spacing=",
            "gap-2",
            "grid columns=1 spacing=8.0 @gap-2\n    text \"x\"",
        ),
        (
            "width",
            "width=",
            "w-full",
            "stack width=100.0 @w-full\n    text \"x\"",
        ),
        (
            "height",
            "height=",
            "h-full",
            "stack height=100.0 @h-full\n    text \"x\"",
        ),
        (
            "text size",
            "size=",
            "text-sm",
            "text \"x\" size=16.0 @text-sm",
        ),
        (
            "text color",
            "color=",
            "text-primary",
            "rich-text color=foreground @text-primary\n    span \"x\"",
        ),
        (
            "text size",
            "size=",
            "text-sm",
            "rich-text\n    span \"x\" size=16.0 @text-sm",
        ),
        (
            "width",
            "width=",
            "w-full",
            "input \"x\" <-> value width=fill @w-full",
        ),
        (
            "padding",
            "padding=",
            "px-2",
            "input \"x\" <-> value padding=8.0 @px-2",
        ),
        (
            "active background",
            "active background=",
            "bg-primary",
            "input \"x\" <-> value @bg-primary\n    active background=background",
        ),
        (
            "focused border color",
            "focused border=",
            "focus:border-danger",
            "input \"x\" <-> value @border focus:border-danger\n    focused border=primary",
        ),
        (
            "padding",
            "padding=",
            "p-2",
            "button \"x\" padding=8.0 @p-2 -> pressed",
        ),
        (
            "hovered background",
            "hovered background=",
            "hover:bg-danger",
            "button \"x\" @bg-primary hover:bg-danger -> pressed\n    hovered background=background",
        ),
        (
            "active text color",
            "active text=",
            "text-primary",
            "button \"x\" @text-primary -> pressed\n    active text=foreground",
        ),
        (
            "pressed radius",
            "pressed radius=",
            "rounded-lg",
            "button \"x\" @rounded-lg -> pressed\n    pressed radius=4.0",
        ),
        (
            "background",
            "background=",
            "bg-primary",
            "pane-grid #work\n    pane files background=background @bg-primary\n      text \"x\"",
        ),
        (
            "background",
            "background=",
            "bg-primary",
            "pane-grid #work\n    pane files\n      title background=background @bg-primary\n        text \"title\"\n      content\n        text \"x\"",
        ),
    ] {
        let source = format!("{header}  {view}\n");
        let error = analyze(&source).unwrap_err();
        assert_eq!(error.code, "E045", "{view}");
        assert!(error.message.contains(&format!("`{property}`")), "{view}");
        assert!(error.message.contains(&format!("`{owner}`")), "{view}");
        assert!(error.message.contains(&format!("`@{utility}`")), "{view}");
        assert!(
            error.hint.is_some_and(|hint| hint.contains(owner)),
            "{view}"
        );
    }

    for view in [
        "row width=fill @w-full\n    text \"x\"",
        "col max-width=320.0 @max-w-md\n    text \"x\"",
        "grid columns=1 width=320.0 @w-full\n    text \"x\"",
        "stack width=fill height=fill\n    text \"x\"",
        "stack @w-full h-full\n    text \"x\"",
        "container style=container_base() @bg-primary\n    text \"x\"",
        "input \"x\" <-> value style=input_base() @bg-primary",
        "button \"x\" style=button_base() @bg-primary -> pressed",
        "button \"x\" @disabled:opacity-50 -> pressed\n    disabled background=background",
        "text \"x\" font=mono style=text_base() @font-bold text-primary",
    ] {
        analyze(&format!("{header}  {view}\n")).unwrap();
    }

    for view in [
        "container border=primary @border-danger\n    text \"x\"",
        "container radius=4.0 @rounded\n    text \"x\"",
    ] {
        let error = analyze(&format!("{header}  {view}\n")).unwrap_err();
        assert_eq!(error.code, "E044", "{view}");
    }
}

#[test]
fn checks_accessibility_names_descriptions_and_icon_buttons() {
    let source = r#"app Accessible
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  name = ""
  checked = false
on press
on toggle(value)
view
  col
    input "Name" label=name description="Profile name" <-> name
    button label="Open help" description="Show help" -> press
      text "?"
    checkbox "Ready" label="Ready state" checked=checked -> toggle _
    image "photo.ppm" label="Portrait" description="Profile portrait"
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("label=name", "label=checked")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`"));

    let error = analyze(&source.replace("label=\"Open help\" ", "")).unwrap_err();
    assert_eq!(error.code, "E105");
    assert!(error.message.contains("child content"));

    let error = analyze(&source.replace(
        "label=\"Portrait\" description=\"Profile portrait\"",
        "description=\"Profile portrait\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E105");
    assert!(error.message.contains("requires an accessibility"));
}
