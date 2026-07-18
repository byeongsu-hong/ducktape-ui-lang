extern crate::backend
  AppError(message:str)
  stream count_stream(limit:i64) -> i64
  stream range_stream(start:i64, limit:i64) -> i64
  stream fallible_stream() -> i64 ! AppError
  recipe counter_recipe(id:i64) -> i64

app TaskStream

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  last = 0
  error = ""
  start = 10
  limit = 3

on start
  parallel
    stream count_stream(3) -> counted _
    stream fallible_stream() -> counted _ | failed _

on counted(value)
  last = value

on failed(reason)
  error = reason.message

on observed(result)

subscribe
  run fallible_stream() -> observed _
  run count_stream(limit) -> counted _
  run range_stream(start, limit) -> counted _
  recipe counter_recipe(start) -> counted _

view
  col
    button "Run streams" -> start
    text last
    text error
