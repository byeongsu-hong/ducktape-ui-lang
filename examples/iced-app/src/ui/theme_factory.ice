extern crate::backend
  theme native_theme(dark:bool)

app NativeTheme
  theme native_theme(dark)

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  dark = true

view
  theme native_theme(!dark)
    text "Native nested theme"
