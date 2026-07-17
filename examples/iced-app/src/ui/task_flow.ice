extern crate::backend
  AppError(message:str)
  stream count_stream(limit:i64) -> i64
  task double_task(value:i64) -> i64
  task optional_task(value:i64) -> i64?
  task fallible_task(value:i64) -> i64 ! AppError

app TaskFlow

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  values:[i64] = []
  value = 0
  error = ""
  planned = 0
  system_theme = ""

on start
  parallel
    flow
      from stream count_stream(3)
      then item -> task double_task(item)
      collect
      done -> collected _
      units -> measured _
    flow
      from task optional_task(2)
      and-then item -> task double_task(item)
      done -> finished _
    flow
      from task fallible_task(2)
      and-then item -> task fallible_task(item)
      done -> finished _
      error -> failed _
    flow
      from stream count_stream(1)
      discard
    flow
      from task system theme
      done -> themed _

on collected(next)
  values = next

on measured(units)
  planned = units

on finished(next)
  value = next

on failed(reason)
  error = reason.message

on themed(next)
  system_theme = next

view
  col
    button "Run task flows" -> start
    text planned
    text value
    text error
    text system_theme
