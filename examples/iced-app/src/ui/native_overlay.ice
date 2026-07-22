extern crate::backend
  component native_overlay(index:f64) -> unit

app NativeOverlay

theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

state
  index = 42.0

view
  extern native_overlay(index)
