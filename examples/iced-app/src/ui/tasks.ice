app Tasks

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

state
  tasks:[Task] = []
  draft = ""
  loading = false
  error = ""
  volume = 40.0
  notifications = true
  view_mode = 0
  external_hover = false
  event_seen = false

component TaskRow(task:Task, loading:bool)
  row #root @w-full items-center p-4 bg-surface border border-border rounded-lg
    checkbox task.title checked=task.done disabled=loading -> toggle(task.id, _)

on mount
  loading = true
  run list_tasks() -> loaded _ | failed _

on submit
  return if loading || empty(trim(draft))
  loading = true
  error = ""
  run create_task(trim(draft)) -> created _ | failed _

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

on copy_draft
  return if empty(trim(draft))
  task copy_text(draft) -> copied | failed _

on copied

on external_hover_changed(next)
  external_hover = next

on external_event(next)
  event_seen = next

subscribe
  app_events() -> external_event _

view
  col @w-full h-full p-6 gap-6 bg-background
    row @w-full items-center gap-3
      text "Tasks" @text-2xl font-bold text-foreground
      text len(tasks) @text-sm text-muted

    row @w-full items-center gap-3
      input "New task" #new-task <-> draft hint="What needs doing?" disabled=loading @w-full px-4 py-3 bg-surface border border-border rounded-lg focus:border-primary
      button "Copy" disabled=empty(trim(draft)) @px-4 py-3 bg-surface text-foreground rounded-lg disabled:opacity-50 -> copy_draft
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

    rule horizontal thickness=1.0

    grid columns=2 @w-full gap-4
      col @w-full gap-2 p-4 bg-surface rounded-lg
        text "Controls" @text-lg font-bold text-foreground
        toggler "Notifications" checked=notifications -> notifications_changed _
        slider volume min=0.0 max=100.0 step=5.0 release=volume_committed -> volume_changed _
        progress volume
        extern native_help(external_hover) -> external_hover_changed _
        if event_seen
          text "External subscription active" @text-xs text-muted
      col @w-full gap-2 p-4 bg-surface rounded-lg
        text "View mode" @text-lg font-bold text-foreground
        radio "List" value=0 selected=(view_mode == 0) -> view_mode_changed _
        radio "Board" value=1 selected=(view_mode == 1) -> view_mode_changed _
        space height=8.0
        stack clip=true @w-full p-4 bg-background rounded-lg
          text "Stack base" @text-sm text-muted
          text "Stack overlay" @text-sm text-foreground

    scroll #task-list @w-full h-full
      col @w-full gap-2
        for task in tasks
          TaskRow(task, loading) #task(task.id)
