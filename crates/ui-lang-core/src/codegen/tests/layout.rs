use super::*;

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
  container #card style=dynamic_container(highlight) width=fill height=80.0 max-width=640.0 max-height=120.0 align-x=center align-y=end clip=true padding=8.0 padding-left=12.0 background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true
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
on resize_editor_stack
  pane #work resize editor_stack 0.55
view
  pane-grid #work width=fill height=fill
    split workspace_root vertical ratio=0.7
      pane files
        text "Files"
      split editor_stack horizontal ratio=0.6
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
    assert!(generated.contains("__pane_work_splits: ::std::collections::BTreeMap"));
    assert!(generated.contains("Option::Some(\"workspace_root\")"));
    assert!(generated.contains("Option::Some(\"editor_stack\")"));
    assert!(generated.contains("self.__pane_work_splits.get(\"editor_stack\").copied()"));
    assert!(generated.contains("self.__pane_work.resize(__split, (0.55) as f32)"));
}

#[test]
fn lowers_runtime_pane_templates() {
    let source = r#"app Workspace
extern crate::backend
  Task(id:i64, title:str)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  tasks:[Task] = []
  selected = 7
on open_task
  pane #work split files task(selected) horizontal
on close_task
  pane #work close task(selected)
on clicked(name)
view
  pane-grid #work click=clicked(_)
    pane files maximized=files_maximized
      col
        if files_maximized
          text "Maximized files"
    pane task in tasks by=task.id maximized=task_maximized
      scroll #body
        col
          if task_maximized
            text "Maximized task"
          text task.title
"#;
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("enum __IcePaneWork"));
    assert!(generated.contains("Task(i64)"));
    assert!(generated.contains("State<__IcePaneWork>"));
    assert!(generated.contains("Configuration::Pane(__IcePaneWork::__Static(\"files\"))"));
    assert!(generated.contains("__IcePaneWork::Task(__pane_key)"));
    assert!(
        generated.contains("self.tasks.iter().find(|task| (*task).id == (*__pane_key).clone())")
    );
    assert!(generated.contains("__IcePaneWork::Task(self.selected)"));
    assert!(generated.contains("__pane.__name()"));
    assert!(generated.contains("__pane_maximized"));
    assert!(generated.contains("if __pane_maximized"));
    assert!(generated.contains("format!(\"{}/task({})\""));
}

#[test]
fn lowers_structured_pane_titles_and_dynamic_controls() {
    let source = r#"app Workspace
extern crate::backend
  pane-grid-style dynamic_panes(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  filter = ""
  active = true
on close
view
  pane-grid #work split=vertical style=dynamic_panes(active)
    style
      hovered-region background=linear(0.785, primary/25@0.0, background@0.5, danger@1.0) border=foreground border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
      hovered-split color=primary width=3.0
      picked-split color=danger width=4.0
    pane files background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true
      title padding=4.0 padding-x=8.0 padding-top=6.0 always-controls background=primary/50 text=foreground border=danger border-width=1.0 radius=3.0 shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=false
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
    assert!(generated.contains("__ui_lang_check_pane_grid_style_dynamic_panes"));
    assert!(generated.contains("crate::backend::dynamic_panes(__theme, self.active)"));
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
    assert!(generated.contains("let mut __style = crate::backend::dynamic_panes"));
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
