extern crate::backend
  refresh_time() -> i64

app TimerEvents

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  auto_refresh = true
  last:instant? = none
  refreshes = 0

on start
  task time now -> tick _

on tick(now)
  last = some(now)

on refreshed(count)
  refreshes = count

subscribe
  every 250ms when auto_refresh -> tick _
  repeat refresh_time() every 1s when auto_refresh -> refreshed _

view
  col
    button "Read time" -> start
    text refreshes
