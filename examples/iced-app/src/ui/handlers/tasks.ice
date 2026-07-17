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

on open_about
  about_open = true

on about_toggled(next)
  about_open = next

on detail_mode_changed(next)
  detail_mode = next

on close_about
  about_open = false

on about_link(url)

on pane_clicked(name)

on maximize_details
  pane #workspace maximize details

on restore_workspace
  pane #workspace restore

on swap_workspace
  pane #workspace swap tasks details

on move_details_left
  pane #workspace move details left

on open_preview
  pane #workspace split details preview horizontal ratio=0.35

on close_preview
  pane #workspace close preview

on resize_workspace
  pane #workspace resize 0.5

on drop_details
  pane #workspace drop details tasks center

on close_details
  pane #workspace close details

on inspect_workspace
  pane #workspace maximized -> pane_observed _

on inspect_adjacent
  pane #workspace adjacent tasks right -> pane_observed _

on pane_observed(name)

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
