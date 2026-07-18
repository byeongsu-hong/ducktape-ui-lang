use super::*;

#[test]
fn checks_complete_flex_layout_options() {
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
    analyze(source).unwrap();

    let bad_metric = source.replace("spacing=8.0", "spacing=-1.0");
    let error = analyze(&bad_metric).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("column metric"));

    let missing_wrap = source.replace("clip=true wrap wrap-spacing", "clip=true wrap-spacing");
    let error = analyze(&missing_wrap).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("require `wrap`"));

    let wrong_property = source.replace("row width=", "row max-width=100.0 width=");
    let error = analyze(&wrong_property).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("unknown layout property"));
}

#[test]
fn checks_complete_container_layout() {
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
    analyze(source).unwrap();

    let bad_metric = source.replace("max-height=120.0", "max-height=-1.0");
    let error = analyze(&bad_metric).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("container metric"));

    let bad_clip = source.replace("clip=true", "clip=1");
    let error = analyze(&bad_clip).unwrap_err();
    assert_eq!(error.code, "E101");

    let bad_style = source.replace("shadow-blur=6.0", "shadow-blur=-1.0");
    let error = analyze(&bad_style).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("surface style metric"));

    let error = analyze(&source.replace("style=dynamic_container(highlight)", "style=rounded"))
        .unwrap_err();
    assert_eq!(error.code, "E184");
    assert!(error.message.contains("container style must be"));

    let error = analyze(&source.replace(
        "dynamic_container(highlight)",
        "missing_container(highlight)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("container style"));

    let error = analyze(&source.replace("dynamic_container(highlight)", "dynamic_container(1.0)"))
        .unwrap_err();
    assert_eq!(error.code, "E101");

    let unknown = source.replace("clip=true", "opaque=true");
    let error = analyze(&unknown).unwrap_err();
    assert_eq!(error.code, "E184");
    assert!(error.message.contains("unknown container property"));
}

#[test]
fn checks_structured_overlays() {
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
    analyze(source).unwrap();

    let wrong_condition = source.replace("when=shown", "when=1");
    let error = analyze(&wrong_condition).unwrap_err();
    assert_eq!(error.code, "E101");

    let bad_padding = source.replace("padding=24.0", "padding=-1.0");
    let error = analyze(&bad_padding).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("overlay padding"));

    let bad_color = source.replace("black/60", "missing/60");
    let error = analyze(&bad_color).unwrap_err();
    assert_eq!(error.code, "E185");
    assert!(error.message.contains("backdrop color"));

    let unnamed_section = source.replace("    content\n", "    page\n");
    let error = analyze(&unnamed_section).unwrap_err();
    assert_eq!(error.code, "E185");
    assert!(error.message.contains("`content` then `layer`"));
}

#[test]
fn checks_persistent_pane_grids() {
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
    analyze(source).unwrap();

    let bad_ratio = source.replace("ratio=0.7", "ratio=2.0");
    let error = analyze(&bad_ratio).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("ratio"));

    let bad_metric = source.replace("min-size=120.0", "min-size=-1.0");
    let error = analyze(&bad_metric).unwrap_err();
    assert_eq!(error.code, "E128");

    let bad_panes = source.replace("pane editor", "panel editor");
    let error = analyze(&bad_panes).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("pane configuration"));
}

#[test]
fn checks_nested_pane_configurations_and_closed_templates() {
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
    let document = analyze(source).unwrap();
    let ViewNode::PaneGrid {
        configuration,
        panes,
        ..
    } = &document.view
    else {
        panic!("pane-grid view")
    };
    assert_eq!(panes.len(), 4);
    assert!(matches!(configuration, PaneConfiguration::Split { .. }));

    let error = analyze(&source.replace("ratio=0.6", "ratio=1.1")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("ratio"));

    let error = analyze(&source.replace("pane terminal", "pane editor")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("duplicate pane `editor`"));

    let error = analyze(&source.replace("pane preview closed", "pane preview hidden")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("pane name closed"));
}

#[test]
fn checks_structured_pane_titles_and_controls() {
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
    analyze(source).unwrap();

    let error = analyze(&source.replace("padding-top=6.0", "padding-top=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("pane title padding"));

    let error = analyze(&source.replace("      controls\n        button \"Close\" -> close\n", ""))
        .unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(
        error
            .message
            .contains("compact-controls require a `controls`")
    );

    let error = analyze(&source.replace("      content\n", "      body\n")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(
        error
            .message
            .contains("title, controls, compact-controls, or content")
    );

    let error = analyze(&source.replace("@bg-background", "@p-4 bg-background")).unwrap_err();
    assert_eq!(error.code, "E042");
    assert!(error.message.contains("has no effect on `pane`"));

    let error = analyze(&source.replace("primary/25@0.0", "missing@0.0")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("unknown pane-grid background color"));

    let error = analyze(&source.replace("danger@1.0", "danger@1.1")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("gradient stop"));

    let error = analyze(&source.replace("danger@1.0", "danger")).unwrap_err();
    assert_eq!(error.code, "E189");
    assert!(error.message.contains("color@offset"));

    let error = analyze(&source.replace(
            "linear(0.785, primary/25@0.0, background@0.5, danger@1.0)",
            "linear(0.785, primary@0.0, primary@0.1, primary@0.2, primary@0.3, primary@0.4, primary@0.5, primary@0.6, primary@0.7, primary@1.0)",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E189");
    assert!(error.message.contains("at most 8 color stops"));

    let error = analyze(&source.replace("shadow-blur=6.0", "shadow-blur=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("surface style metric"));

    let error = analyze(&source.replace("pixel-snap=true", "pixel-snap=1.0")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("width=3.0", "width=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("pane-grid style metric"));

    let error = analyze(&source.replace("hovered-split color", "active-split color")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(
        error
            .message
            .contains("hovered-region, hovered-split, or picked-split")
    );
}

#[test]
fn checks_pane_state_operations_and_queries() {
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
  pane #work drop editor files center
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
    let document = analyze(source).unwrap();
    assert_eq!(document.handlers[3].params[0].ty.display(), "str?");

    let error = analyze(&source.replace("#work maximize", "#missing maximize")).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("unknown pane-grid"));

    let error = analyze(&source.replace("maximize editor", "maximize missing")).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("has no pane `missing`"));

    let error = analyze(&source.replace("swap files editor", "swap files files")).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("different panes"));

    let error = analyze(&source.replace("resize 0.6", "resize 1.1")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("pane split ratio"));

    let error =
        analyze(&source.replace("pane #work maximized -> observed _", "pane #work maximized"))
            .unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("query requires a route"));

    let duplicate = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Frame()
  row
    slot left
    slot right
view
  Frame
    left:
      pane-grid #work split=vertical
        pane a
          text "A"
        pane b
          text "B"
    right:
      pane-grid #work split=horizontal
        pane c
          text "C"
        pane d
          text "D"
"#;
    let error = analyze(duplicate).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("duplicate persistent pane-grid"));
}

#[test]
fn checks_complete_grid_sizing() {
    let source = r#"app Layouts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  col
    grid columns=2 width=640.0 spacing=12.0 height=aspect(16.0,9.0)
      text "Fixed"
    grid fluid=240.0 height=fill(2)
      text "Fluid"
"#;
    analyze(source).unwrap();

    let conflicting = source.replace("columns=2", "columns=2 fluid=240.0");
    let error = analyze(&conflicting).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("mutually exclusive"));

    let zero_fluid = source.replace("fluid=240.0", "fluid=0.0");
    let error = analyze(&zero_fluid).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("grid fluid width"));

    let zero_aspect = source.replace("aspect(16.0,9.0)", "aspect(16.0,0.0)");
    let error = analyze(&zero_aspect).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("grid aspect height"));
}

#[test]
fn rejects_invalid_rule_style_values() {
    let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  rule horizontal fill=percent(101.0)
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("rule percent"));

    let unknown_color = source.replace("fill=percent(101.0)", "color=missing");
    let error = analyze(&unknown_color).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("unknown rule color"));
}
