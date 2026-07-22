extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  create_task(title:str) -> [Task] ! AppError

app TaskGroups

theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _

on theme_read(next)

on clipboard_read(next)

on info_read(info)

on create_twice(title)
  parallel
    run create_task(title) -> tasks_read _ | create_failed _
    run create_task(title) -> tasks_read _ | create_failed _

on tasks_read(tasks)

on create_failed(error)

view
  col
    button "Run grouped tasks" -> start
    button "Clone captured input" -> create_twice("copy")
