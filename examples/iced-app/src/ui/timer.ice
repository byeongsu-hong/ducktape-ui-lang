app TimerEvents

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  auto_refresh = true

on tick

subscribe
  every 250ms when auto_refresh -> tick

view
  text "Timer compile fixture"
