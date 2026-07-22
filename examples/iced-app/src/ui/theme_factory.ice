extern crate::backend
  theme native_theme(dark:bool)

app NativeTheme
  theme native_theme(dark)

theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

state
  dark = true

view
  theme native_theme(!dark)
    text "Native nested theme"
