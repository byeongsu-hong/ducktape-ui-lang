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
use "components/dialog.ice"
use "handlers/tasks.ice"

view
  overlay when=about_open dismiss=close_about backdrop=black/60 padding=24.0 align-x=center align-y=center
    content
      col @w-full h-full p-6 gap-6 bg-background
        row @w-full items-center gap-3
          text "Tasks" @text-2xl font-bold text-foreground
          text len(tasks) @text-sm text-muted
          button "About" -> open_about

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

        pane-grid #workspace split=vertical ratio=0.7 width=fill height=fill spacing=8.0 min-size=120.0 resize=8.0 drag click=pane_clicked(_)
          style
            hovered-region background=linear(0.785, primary/10@0.0, primary/40@1.0) border=primary border-width=2.0 radius=8.0
            hovered-split color=primary width=3.0
            picked-split color=foreground width=3.0
          pane tasks background=linear(1.57, surface@0.0, background@1.0) shadow=black/50 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true @bg-surface border border-border rounded-lg
            title padding=12.0 always-controls background=linear(1.57, background@0.0, surface@1.0) border=border border-width=1.0 radius-tl=8.0 radius-tr=8.0 shadow=black/25 shadow-y=1.0 shadow-blur=3.0 pixel-snap=true @bg-background border border-border
              text "Task list" @text-lg font-bold text-foreground
            controls
              button "Inspect" -> inspect_adjacent
            compact-controls
              button "?" -> inspect_adjacent
            content
              scroll #task-list direction=vertical width=fill height=fill
                keyed task in tasks by=task.id width=fill spacing=8.0
                  TaskRow task=task loading=loading
          pane details @bg-surface border border-border rounded-lg
            title padding=12.0 always-controls @bg-background border border-border
              text "Details" @text-lg font-bold text-foreground
            controls
              button "Maximize" -> maximize_details
            compact-controls
              button "↗" -> maximize_details
            content
              container width=fill height=fill padding=16.0
                col @gap-3
                  text "Drag, resize, or arrange this pane." @text-sm text-muted
                  row wrap @gap-2
                    button "Restore" -> restore_workspace
                    button "Swap" -> swap_workspace
                    button "Move left" -> move_details_left
                    button "Open preview" -> open_preview
          pane preview closed @bg-surface border border-border rounded-lg
            title padding=12.0 always-controls @bg-background border border-border
              text "Preview" @text-lg font-bold text-foreground
            controls
              button "Close" -> close_preview
            compact-controls
              button "×" -> close_preview
            content
              container width=fill height=fill padding=16.0
                text "This pane was opened dynamically." @text-sm text-muted
    layer
      Dialog
        Dialog.Header
          text "About Ice Tasks" @text-xl font-bold text-foreground
        Dialog.Body
          rich-text width=fill wrapping=word @text-sm text-muted -> about_link _
            span "This dialog is a structured overlay written entirely in "
            span ".ice" link="https://github.com/byeongsu-hong/ducktape-ui-lang" underline @font-bold text-primary
            span "."
        Dialog.Actions
          button "Close" @px-4 py-2 bg-primary text-white rounded-md -> close_about
