component TaskRow(task:Task, loading:bool)
  row @w-full items-center p-4 bg-surface border border-border rounded-lg
    checkbox task.title checked=task.done disabled=loading -> toggle(task.id, _)
