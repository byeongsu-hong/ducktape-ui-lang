extern crate::backend
  themer alternate_panel(active:bool) -> unit

app AlternateThemeApp

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  active = true

view
  themer alternate_panel(active)
