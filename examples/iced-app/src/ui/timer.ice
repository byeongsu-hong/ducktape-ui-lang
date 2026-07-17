app TimerEvents

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

on tick

subscribe
  every 250ms -> tick

view
  text "Timer compile fixture"
