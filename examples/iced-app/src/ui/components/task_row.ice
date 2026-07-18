component TaskRow(task:Task, loading:bool)
  row padding=16.0 align=center @w-full bg-surface border border-border rounded-lg
    checkbox task.title checked=task.done disabled=loading style=success -> toggle(task.id, _)
      active checked background=linear(1.57, primary@0.0, surface@1.0) icon=foreground text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=3.0 radius-tr=4.0 radius-br=5.0 radius-bl=6.0
      active unchecked background=surface icon=primary text=foreground border=border
      hovered checked background=primary icon=foreground text=foreground border=foreground
      hovered unchecked background=background icon=primary text=foreground border=primary
      disabled checked background=surface icon=muted text=muted border=border
      disabled unchecked background=background icon=muted text=muted border=border
