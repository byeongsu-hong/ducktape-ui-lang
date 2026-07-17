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
          toggler "About" checked=about_open disabled=loading size=18.0 spacing=8.0 -> about_toggled _
            active checked background=linear(1.57, primary@0.0, surface@1.0) background-border=primary background-border-width=1.0 foreground=linear(0.0, foreground@0.0, primary@1.0) foreground-border=foreground foreground-border-width=1.0 text=foreground radius=7.0 radius-tl=6.0 radius-tr=7.0 radius-br=8.0 radius-bl=9.0 padding-ratio=0.125
            active unchecked background=surface foreground=foreground text=muted
            hovered checked background=primary foreground=foreground text=foreground
            hovered unchecked background=background foreground=primary text=foreground
            disabled checked background=surface foreground=muted text=muted
            disabled unchecked background=background foreground=muted text=muted
          button "About" style=text -> open_about

        row @w-full items-center gap-3
          input "New task" <-> draft hint="What needs doing?" disabled=loading submit=submit @w-full px-4 py-3 bg-surface border border-border rounded-lg
          button "Add" disabled=(loading || empty(trim(draft))) style=success @px-4 py-3 bg-primary text-white rounded-lg disabled:opacity-50 -> submit
            active background=linear(1.57, primary@0.0, surface@1.0) text=white border=primary border-width=1.0 radius=8.0 shadow=black/25 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
            hovered background=linear(1.57, surface@0.0, primary@1.0) text=white
            pressed background=primary/80 text=white
            disabled background=surface text=muted

        if error != ""
          row @w-full items-center gap-4 p-4 bg-danger rounded-lg
            text error @text-sm text-white
            button "Retry" disabled=loading style=danger @px-4 py-2 bg-white text-danger rounded-md -> retry

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
              button "Inspect" style=secondary -> inspect_adjacent
            compact-controls
              button "?" style=subtle -> inspect_adjacent
            content
              scroll #task-list direction=vertical width=fill height=fill
                keyed task in tasks by=task.id width=fill spacing=8.0
                  TaskRow task=task loading=loading
          pane details @bg-surface border border-border rounded-lg
            title padding=12.0 always-controls @bg-background border border-border
              text "Details" @text-lg font-bold text-foreground
            controls
              button "Maximize" style=background -> maximize_details
            compact-controls
              button "↗" style=warning -> maximize_details
            content
              container width=fill height=fill padding=16.0
                col @gap-3
                  text "Drag, resize, or arrange this pane." @text-sm text-muted
                  row wrap @gap-2
                    radio "Summary" value="summary" selected=(detail_mode == "summary") size=16.0 spacing=6.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word font=default -> detail_mode_changed _
                      active selected background=linear(1.57, primary@0.0, surface@1.0) dot=foreground border=primary border-width=2.0 text=foreground
                      active unselected background=surface dot=primary border=border text=muted
                      hovered selected background=primary dot=foreground border=foreground text=foreground
                      hovered unselected background=background dot=primary border=primary text=foreground
                    radio "Activity" value="activity" selected=(detail_mode == "activity") -> detail_mode_changed _
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
            span ".ice" link="https://github.com/byeongsu-hong/ducktape-ui-lang" background=linear(1.57, primary/20@0.0, surface@1.0) padding=2.0 radius=2.0 underline @font-bold text-primary
            span "."
        Dialog.Actions
          button "Close" style=primary @px-4 py-2 bg-primary text-white rounded-md -> close_about
