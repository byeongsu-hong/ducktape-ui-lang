app Tasks
  title "Ice Tasks"
  id "dev.ducktape.ice.tasks"
  default-text-size 16
  antialiasing true
  vsync true
  scale-factor 1
  window
    size 960 720
    min-size 480 360
    position centered

use "backend.ice"
use "theme.ice"
use "state.ice"
use "components/task_row.ice"
use "components/panel.ice"
use "handlers/tasks.ice"

view
  col @w-full h-full p-6 gap-6 bg-background
    row @w-full items-center gap-3
      text "Tasks" @text-2xl font-bold text-foreground
      text len(tasks) @text-sm text-muted

    row @w-full items-center gap-3
      input "New task" <-> draft hint="What needs doing?" disabled=loading submit=submit @w-full px-4 py-3 bg-surface border border-border rounded-lg
      button "Add" disabled=(loading || empty(trim(draft))) @px-4 py-3 bg-primary text-white rounded-lg disabled:opacity-50 -> submit

    if error != ""
      row @w-full items-center gap-4 p-4 bg-danger rounded-lg
        text error @text-sm text-white
        button "Retry" disabled=loading @px-4 py-2 bg-white text-danger rounded-md -> retry

    lazy loading as busy
      col
        if busy
          text "Working..." @text-sm text-muted

    if empty(tasks) && !loading
      text "No tasks yet." @text-sm text-muted

    Panel title="Task list" #tasks-panel
      scroll #task-list direction=vertical width=fill height=fill
        keyed task in tasks by=task.id width=fill spacing=8.0
          TaskRow task=task loading=loading
