app Tasks

extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  list_tasks() -> [Task] ! AppError
  create_task(title:str) -> [Task] ! AppError
  set_task_done(id:i64, done:bool) -> [Task] ! AppError

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

view
  col @w-full h-full p-6 gap-6 bg-background
    row @w-full items-center gap-3
      text "Tasks" @text-2xl font-bold text-foreground
      text len(tasks) @text-sm text-muted

    row @w-full items-center gap-3
      input "New task" #new-task <-> draft hint="What needs doing?" disabled=loading @w-full px-4 py-3 bg-surface border border-border rounded-lg focus:border-primary
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

    scroll #task-list @w-full h-full
      col @w-full gap-2
        for task in tasks
          TaskRow(task, loading) #task(task.id)
