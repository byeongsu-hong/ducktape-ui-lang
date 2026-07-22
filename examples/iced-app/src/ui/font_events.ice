app FontEvents

theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

state
  font_bytes:bytes = bytes(00 01)

on load
  task font load font_bytes -> loaded _

on loaded(result)

view
  button "Load font bytes" -> load
