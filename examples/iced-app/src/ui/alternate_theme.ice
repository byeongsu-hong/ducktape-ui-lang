extern crate::backend
  AlternateTheme()
  sync alternate_theme(active:bool) -> AlternateTheme
  load_alternate_theme() -> AlternateTheme
  themer alternate_panel(active:bool) -> unit

app AlternateThemeApp

theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

state
  active = true
  native_theme:AlternateTheme = alternate_theme(true)

component NativeTheme()
  state
    remembered:AlternateTheme = alternate_theme(true)
  text "Native component state"

on mount
  run load_alternate_theme() -> loaded _

on loaded(next)
  native_theme = next

view
  col
    themer alternate_panel(active)
    NativeTheme
