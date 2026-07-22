app NativeScrollDelta

extern crate::backend
  sync scroll_delta_round_trip(value:scroll-delta) -> scroll-delta

theme
  bg #111827
  fg #f9fafb
  primary #60a5fa
  danger #f87171

state
  lines:scroll-delta = scroll.lines(0.0, 0.0)
  pixels:scroll-delta = scroll.pixels(0.0, 0.0)
  returned:scroll-delta = scroll.lines(0.0, 0.0)
  line_kind = ""
  pixel_kind = ""
  line_x = 0.0
  line_y = 0.0
  pixel_x = 0.0
  pixel_y = 0.0
  values_equal = false

on inspect
  lines = scroll.lines(1.5, -2.25)
  pixels = scroll.pixels(-3.75, 4.5)
  returned = scroll_delta_round_trip(pixels)
  line_kind = lines.kind
  pixel_kind = pixels.kind
  line_x = lines.x
  line_y = lines.y
  pixel_x = pixels.x
  pixel_y = pixels.y
  values_equal = returned == pixels

view
  col spacing=8.0 padding=16.0
    button "Inspect" -> inspect
    text line_kind
    text pixel_kind
