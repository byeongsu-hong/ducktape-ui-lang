app NativeWindowPosition

extern crate::backend
  sync position_round_trip(value:window-position) -> window-position
  sync responsive_position() -> window-position

theme
  background #111827
  foreground #f9fafb
  primary #60a5fa
  danger #f87171

state
  default_position:window-position = window_position.default()
  centered_position:window-position = window_position.default()
  specific_position:window-position = window_position.default()
  responsive:window-position = window_position.default()
  returned:window-position = window_position.default()
  returned_point:point? = none
  missing_point:point? = none
  default_kind = ""
  centered_kind = ""
  specific_kind = ""
  responsive_kind = ""

on inspect
  default_position = window_position.default()
  centered_position = window_position.centered()
  specific_position = window_position.specific(point(24.0, -12.0))
  responsive = responsive_position()
  returned = position_round_trip(specific_position)
  returned_point = returned.point
  missing_point = responsive.point
  default_kind = default_position.kind
  centered_kind = centered_position.kind
  specific_kind = returned.kind
  responsive_kind = responsive.kind

view
  col spacing=8.0 padding=16.0
    button "Inspect" -> inspect
    text default_kind
    text centered_kind
    text specific_kind
    text responsive_kind
