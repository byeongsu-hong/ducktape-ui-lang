app Showcase

font ui family=sans weight=medium stretch=normal style=normal default=true

extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  list_tasks() -> [Task] ! AppError
  create_task(title:str) -> [Task] ! AppError
  set_task_done(id:i64, done:bool) -> [Task] ! AppError
  component native_help(active:bool) -> bool
  task copy_text(text:str) -> unit ! AppError
  subscription app_events() -> bool

theme
  background #0f172a
  surface    #111827
  foreground #f8fafc
  muted      #94a3b8
  primary    #7c3aed
  danger     #dc2626
  border     #334155

qr project_code "https://github.com/byeongsu-hong/ducktape-ui-lang" correction=high version=normal(8)

state
  tasks:[Task] = []
  draft = ""
  loading = false
  error = ""
  volume = 40.0
  notifications = true
  view_mode = 0
  display_modes = ["List", "Board", "Timeline"]
  searchable_modes:combo[str] = ["List", "Board", "Timeline"]
  display_mode:str? = none
  picker_open = false
  mode_query = ""
  hovered_mode = ""
  observed_width = 0.0
  observed_height = 0.0
  external_hover = false
  event_seen = false
  native_hover = false
  pointer_x = 0.0
  pointer_y = 0.0
  scroll_pixels = false
  scroll_x = 0.0
  scroll_y = 0.0
  scroll_relative_x = 0.0
  scroll_relative_y = 0.0
  help:markdown = "# Ice **renders** [iced docs](https://iced.rs)"
  notes:editor = "fn main() { println!(\"ice\"); }"
  last_key = "none"
  command_down = false
  key_repeat = false
  system_theme = "none"
  cpu_brand = "unknown"
  clipboard_text:str? = none
  primary_text:str? = none
  draft_focused = false
  window_width = 0.0
  window_height = 0.0
  window_maximized = false
  window_minimized:bool? = none
  window_x:f64? = none
  window_y:f64? = none
  window_scale = 1.0
  window_mode = "windowed"
  monitor_width:f64? = none
  monitor_height:f64? = none

component TaskRow(task:Task, loading:bool)
  row #root @w-full items-center p-4 bg-surface border border-border rounded-lg
    checkbox task.title checked=task.done disabled=loading size=18.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=auto wrapping=word-or-glyph font=default icon="✓" icon-size=12.0 icon-line-height=1.0 icon-shaping=basic -> toggle(task.id, _)

on mount
  loading = true
  run list_tasks() -> loaded _ | failed _

on submit
  return if loading || empty(trim(draft))
  loading = true
  error = ""
  run create_task(trim(draft)) -> created _ | failed _

on draft_pasted(next)
  draft = next

on toggle(id, checked)
  return if loading
  loading = true
  error = ""
  run set_task_done(id, checked) -> updated _ | failed _

on retry
  loading = true
  error = ""
  run list_tasks() -> loaded _ | failed _

on loaded(next)
  tasks = next
  loading = false

on created(next)
  tasks = next
  draft = ""
  loading = false

on updated(next)
  tasks = next
  loading = false

on failed(cause)
  loading = false
  error = cause.message

on volume_changed(next)
  volume = next

on volume_committed

on notifications_changed(next)
  notifications = next

on view_mode_changed(next)
  view_mode = next

on display_mode_changed(next)
  display_mode = some(next)

on picker_opened
  picker_open = true

on picker_closed
  picker_open = false

on mode_searched(next)
  mode_query = next

on mode_hovered(next)
  hovered_mode = next

on panel_measured(width, height)
  observed_width = width
  observed_height = height

on panel_hidden
  observed_width = 0.0
  observed_height = 0.0

on copy_draft
  return if empty(trim(draft))
  task clipboard write draft

on copied

on copy_primary
  task clipboard write-primary draft

on read_clipboard
  task clipboard read -> clipboard_read _

on clipboard_read(value)
  clipboard_text = value

on read_primary
  task clipboard read-primary -> primary_read _

on primary_read(value)
  primary_text = value

on focus_draft
  task widget focus #new-task

on check_draft_focus
  task widget focused #new-task -> draft_focus_checked _

on draft_focus_checked(value)
  draft_focused = value

on previous_focus
  task widget focus-previous

on next_focus
  task widget focus-next

on draft_cursor_front
  task widget cursor-front #new-task

on draft_cursor_end
  task widget cursor-end #new-task

on draft_cursor
  task widget cursor #new-task 2

on draft_select_all
  task widget select-all #new-task

on draft_select_range
  task widget select #new-task 0 2

on task_list_snap
  task widget snap #task-list 0.0 0.5

on task_list_snap_end
  task widget snap-end #task-list

on task_list_scroll_to
  task widget scroll-to #task-list 0.0 24.0

on task_list_scroll_by
  task widget scroll-by #task-list 0.0 8.0

on window_close
  task window close

on window_drag
  task window drag

on window_drag_resize
  task window drag-resize south-east

on window_resize
  task window resize 960.0 720.0

on window_resizable
  task window resizable true

on window_min_size
  task window min-size 480.0 360.0

on window_clear_min_size
  task window min-size none

on window_max_size
  task window max-size 1920.0 1080.0

on window_resize_increments
  task window resize-increments 8.0 8.0

on window_read_size
  task window size -> window_size_read _ _

on window_size_read(width, height)
  window_width = width
  window_height = height

on window_read_maximized
  task window maximized -> window_maximized_read _

on window_maximized_read(value)
  window_maximized = value

on window_maximize
  task window maximize true

on window_read_minimized
  task window minimized -> window_minimized_read _

on window_minimized_read(value)
  window_minimized = value

on window_minimize
  task window minimize false

on window_read_position
  task window position -> window_position_read _ _

on window_position_read(x, y)
  window_x = x
  window_y = y

on window_read_scale
  task window scale-factor -> window_scale_read _

on window_scale_read(value)
  window_scale = value

on window_move
  task window move 40.0 40.0

on window_read_mode
  task window mode -> window_mode_read _

on window_mode_read(value)
  window_mode = value

on window_fullscreen
  task window set-mode fullscreen

on window_toggle_maximize
  task window toggle-maximize

on window_toggle_decorations
  task window toggle-decorations

on window_attention
  task window attention informational

on window_clear_attention
  task window attention none

on window_focus
  task window focus

on window_level
  task window level normal

on window_system_menu
  task window system-menu

on window_mouse_passthrough
  task window mouse-passthrough false

on window_read_monitor
  task window monitor-size -> window_monitor_read _ _

on window_monitor_read(width, height)
  monitor_width = width
  monitor_height = height

on window_automatic_tabbing
  task window automatic-tabbing false

on external_hover_changed(next)
  external_hover = next

on external_event(next)
  event_seen = next

on native_enter
  native_hover = true

on native_exit
  native_hover = false

on native_press
  native_hover = !native_hover

on native_move(x, y)
  pointer_x = x
  pointer_y = y

on native_scroll(x, y, pixels)
  pointer_x = x
  pointer_y = y
  scroll_pixels = pixels

on task_list_scrolled(x, y, relative_x, relative_y)
  scroll_x = x
  scroll_y = y
  scroll_relative_x = relative_x
  scroll_relative_y = relative_y

on docs_link(url)

on key_pressed(event)
  last_key = event.key
  command_down = event.modifiers.command
  key_repeat = event.repeat

on key_released(event)
  last_key = event.key
  command_down = event.modifiers.command

on key_modifiers_changed(modifiers)
  command_down = modifiers.command

on inspect_system
  task system info -> system_inspected _

on system_inspected(info)
  cpu_brand = info.cpu_brand

on read_system_theme
  task system theme -> system_theme_changed _

on system_theme_changed(next)
  system_theme = next

on open_nested_preview
  pane #nested_workspace split nested_editor nested_preview horizontal ratio=0.4

on close_nested_preview
  pane #nested_workspace close nested_preview

subscribe
  app_events() -> external_event _
  keyboard press status=ignored -> key_pressed _
  keyboard release -> key_released _
  keyboard modifiers -> key_modifiers_changed _
  system theme -> system_theme_changed _

view
  col @w-full h-full p-6 gap-6 bg-background
    row @w-full items-center gap-3
      text "Tasks" font=ui @text-2xl font-bold text-foreground
      lazy tasks as cached_tasks
        text len(cached_tasks) @text-sm text-muted
      text last_key @text-sm text-muted
      text system_theme @text-sm text-muted
      text cpu_brand @text-sm text-muted
      button "Inspect system" -> inspect_system
      button "Read theme" -> read_system_theme
      button "Copy primary" -> copy_primary
      button "Read clipboard" -> read_clipboard
      button "Read primary" -> read_primary
      button "Focus draft" -> focus_draft
      button "Check draft focus" -> check_draft_focus
      button "Read window size" -> window_read_size
      button "Toggle maximize" -> window_toggle_maximize
      button "Focus window" -> window_focus
      if draft_focused
        text "draft focused" @text-sm text-muted

    row @w-full items-center gap-3
      input "New task" #new-task <-> draft hint="What needs doing?" disabled=loading secure=false submit=submit paste=draft_pasted width=fill text-size=14.0 line-height=1.2 align=left font=ui icon="+" icon-side=left icon-size=14.0 icon-spacing=6.0 @px-4 py-3 bg-surface border border-border rounded-lg focus:border-primary
      button disabled=empty(trim(draft)) height=44.0 padding=8.0 clip=true @bg-surface text-foreground rounded-lg disabled:opacity-50 -> copy_draft
        row @gap-2 items-center
          text "Copy" @text-sm text-foreground
          text "⌘C" @text-xs text-muted
      button "Add" disabled=(loading || empty(trim(draft))) @px-4 py-3 bg-primary text-white rounded-lg hover:bg-primary/90 pressed:bg-primary/70 disabled:opacity-50 -> submit

    if error != ""
      row @w-full items-center gap-4 p-4 bg-danger rounded-lg
        text error @text-sm text-white
        button "Retry" disabled=loading @px-4 py-2 bg-white text-danger rounded-md disabled:opacity-50 -> retry

    if loading
      text "Working..." @text-sm text-muted

    if empty(tasks) && !loading
      col @w-full items-center p-6 bg-surface border border-border rounded-lg
        text "No tasks yet." @text-sm text-muted

    container #summary width=fill height=80.0 max-width=720.0 max-height=120.0 align-x=center align-y=center clip=true padding=8.0 padding-left=12.0 background=linear(1.57, surface@0.0, background@1.0) text=muted border=primary border-width=1.0 radius=8.0 shadow=black/50 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true @w-full bg-surface border border-border rounded-lg
      text "A native container owns one structured child tree." @text-sm text-muted

    rule horizontal thickness=1.0 style=weak fill=pad(12,4) color=border radius=2.0 snap=true

    grid spacing=16.0 width=640.0 height=aspect(16.0,9.0) fluid=280.0 @w-full gap-4
      col @w-full gap-2 p-4 bg-surface rounded-lg
        text "Controls" width=fill height=30.0 size=18.0 line-height-px=22.0 font=default align-x=left align-y=center shaping=advanced wrapping=word @font-bold text-foreground
        theme tokyo-night text=white background=linear(1.57, background@0.0, surface@1.0)
          qr project_code total-size=112.0 cell=foreground background=surface
        toggler "Notifications" checked=notifications size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=auto wrapping=word font=default align=left -> notifications_changed _
        slider volume min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 width=fill(2) height=20.0 release=volume_committed -> volume_changed _
          active rail-start=linear(0.0, primary@0.0, foreground@1.0) rail-end=linear(1.57, border@0.0, background@1.0) rail-width=4.0 rail-radius=2.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, foreground@1.0)
          hovered rail-start=foreground rail-end=border rail-width=5.0 handle=rect(12) handle-color=foreground handle-radius=3.0
          dragged rail-start=danger rail-end=border handle=circle(8.0) handle-color=danger handle-border=foreground handle-border-width=1.0
        slider volume min=0.0 max=100.0 step=5.0 default=50.0 shift-step=1.0 vertical width=20.0 height=120.0 release=volume_committed -> volume_changed _
        progress volume length=fill girth=24.0 style=success background=linear(1.57, background@0.0, surface@1.0) bar=linear(0.0, primary@0.0, foreground@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0
        progress volume vertical length=120.0 girth=20.0 style=warning background=linear(1.57, background@0.0, surface@1.0) bar=linear(0.0, danger@0.0, primary@1.0) radius=3.0
        extern native_help(external_hover) -> external_hover_changed _
        if event_seen
          text "External subscription active" @text-xs text-muted
        row width=fill height=shrink spacing=12.0 padding-y=4.0 align=center clip=false wrap wrap-spacing=8.0 wrap-align=start
          image "examples/iced-app/assets/checker.ppm" width=48.0 height=48.0 fit=cover filter=nearest radius=8.0
          svg "examples/iced-app/assets/ice.svg" width=48.0 height=48.0 fit=contain opacity=0.9
          tooltip position=bottom gap=4.0 padding=8.0 delay=100 snap=true style=rounded background=linear(1.57, surface@0.0, background@1.0) text=foreground border=border border-width=1.0 radius=8.0 radius-tl=4.0 shadow=black/50 shadow-x=0.0 shadow-y=4.0 shadow-blur=12.0 pixel-snap=true
            mouse enter=native_enter exit=native_exit press=native_press move=native_move scroll=native_scroll cursor=pointer
              text "Native pointer area" @text-sm text-foreground
            col @p-2 bg-surface rounded-md
              text "Native tooltip" @text-sm text-foreground
              if native_hover
                text "Pointer is inside" @text-xs text-muted
              text pointer_x @text-xs text-muted
      col width=fill height=shrink spacing=8.0 padding=16.0 max-width=672.0 align=start clip=false wrap wrap-spacing=8.0 wrap-align=start @bg-surface rounded-lg
        text "View mode" @text-lg font-bold text-foreground
        markdown help text-size=14.0 h1-size=28.0 h2-size=24.0 h3-size=20.0 h4-size=18.0 h5-size=16.0 h6-size=14.0 code-size=12.0 spacing=10.0 -> docs_link _
        editor #notes <-> notes placeholder="Write notes" width=640.0 height=120.0 min-height=80.0 max-height=240.0 size=14.0 line-height=1.3 padding=8.0 wrapping=word font=ui highlight="rs" highlight-theme=base16-ocean disabled=loading
        pick display_modes display_mode placeholder="Choose a view" width=fill menu-height=160.0 padding=8.0 text-size=14.0 open=picker_opened close=picker_closed -> display_mode_changed _
        combo searchable_modes display_mode "Search views" width=fill menu-height=160.0 padding=8.0 text-size=14.0 input=mode_searched hover=mode_hovered open=picker_opened close=picker_closed -> display_mode_changed _
        if picker_open
          text "Picker is open" @text-xs text-muted
        if mode_query != ""
          text mode_query @text-xs text-muted
        if hovered_mode != ""
          text hovered_mode @text-xs text-muted
        sensor show=panel_measured resize=panel_measured hide=panel_hidden key=mode_query anticipate=16.0 delay=10
          responsive size=(available_width, available_height) width=fill height=32.0
            col @gap-2
              if available_width < 360.0
                text "Compact responsive view" @text-xs text-muted
              if available_width >= 360.0
                row @gap-2
                  text "Wide" @text-xs text-muted
                  text available_height @text-xs text-muted
                  text observed_width @text-xs text-muted
                  text observed_height @text-xs text-muted
        float scale=1.02 x=0.0 y=-1.0
          text "Floating label" @text-xs text-foreground
        pin width=fill height=28.0 x=4.0 y=4.0
          text "Pinned label" @text-xs text-muted
        pin width=fill(2) height=shrink x=8.0 y=6.0
          text "Pinned with flexible bounds" @text-xs text-muted
        radio "List" value=0 selected=(view_mode == 0) -> view_mode_changed _
        radio "Board" value=1 selected=(view_mode == 1) -> view_mode_changed _
        grid columns=2 height=shrink spacing=4.0 @w-full
          text "Even" @text-xs text-muted
          text "height" @text-xs text-muted
        grid columns=1 height=fill @w-full
          text "Fill height" @text-xs text-muted
        grid columns=1 height=fill(2) @w-full
          text "Fill portion height" @text-xs text-muted
        grid columns=1 height=24.0 @w-full
          text "Fixed height" @text-xs text-muted
        space width=fill(2) height=8.0
        stack clip=true width=fill height=shrink under=1 @p-4 bg-background rounded-lg
          text "Stack underlay" @text-sm text-muted
          text "Stack base" @text-sm text-muted
          text "Stack overlay" @text-sm text-foreground

    pane-grid #nested_workspace width=fill height=180.0 spacing=4.0 resize=4.0 drag
      split vertical ratio=0.65
        pane nested_files
          text "Nested files" @text-sm text-muted
        split horizontal ratio=0.6
          pane nested_editor
            button "Open nested preview" -> open_nested_preview
          pane nested_terminal
            text "Nested terminal" @text-sm text-muted
      pane nested_preview closed
        col @gap-2
          text "Dynamic preview" @text-sm text-foreground
          button "Close nested preview" -> close_nested_preview

    scroll #task-list direction=vertical width=fill height=fill bar=visible bar-width=8.0 bar-margin=2.0 scroller-width=6.0 bar-spacing=2.0 anchor-y=start auto=true scroll=task_list_scrolled
      keyed task in tasks by=task.id width=fill height=shrink spacing=8.0 padding=4.0 padding-left=8.0 max-width=720.0 align=center
        TaskRow task=task loading=loading

    table task in tasks width=fill padding=4.0 padding-x=8.0 padding-y=6.0 separator=1.0 separator-x=2.0 separator-y=1.0
      column width=fill align-x=left align-y=center
        header
          text "Task" @font-bold text-foreground
        cell
          text task.title @text-sm text-foreground
      column width=120.0 align-x=center align-y=center
        header
          text "Done" @font-bold text-foreground
        cell
          checkbox "Complete" checked=task.done disabled=loading -> toggle(task.id, _)
