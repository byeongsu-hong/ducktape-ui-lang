extern crate::backend
  themer alternate_panel(active:bool) -> unit

app AlternateThemeApp

theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

state
  active = true

view
  themer alternate_panel(active)
