app NativeThemeMode

extern crate::backend
  sync theme_mode_round_trip(value:theme-mode) -> theme-mode

theme
  bg #111827
  fg #f9fafb
  primary #60a5fa
  danger #f87171

state
  default_mode:theme-mode = theme_mode.default()
  modes:[theme-mode] = []
  returned:theme-mode = theme_mode.none()
  kind = ""
  values_equal = false

on inspect
  default_mode = theme_mode.default()
  modes = [theme_mode.none(), theme_mode.light(), theme_mode.dark()]
  returned = theme_mode_round_trip(theme_mode.dark())
  kind = returned.kind
  values_equal = returned == theme_mode.dark()

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text kind
