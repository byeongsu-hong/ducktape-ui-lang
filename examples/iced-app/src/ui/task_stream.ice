extern crate::backend
  AppError(message:str)
  stream count_stream(limit:i64) -> i64
  stream fallible_stream() -> i64 ! AppError

app TaskStream

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  last = 0
  error = ""

on start
  parallel
    stream count_stream(3) -> counted _
    stream fallible_stream() -> counted _ | failed _

on counted(value)
  last = value

on failed(reason)
  error = reason.message

view
  col
    button "Run streams" -> start
    text last
    text error
