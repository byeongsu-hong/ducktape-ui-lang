app NativeRedrawRequest

extern crate::backend
  sync redraw_round_trip(value:redraw-request) -> redraw-request
  sync redraw_now() -> instant

theme
  bg #111827
  fg #f9fafb
  primary #60a5fa
  danger #f87171

state
  next_frame:redraw-request = redraw_request.next_frame()
  at:redraw-request = redraw_request.next_frame()
  wait:redraw-request = redraw_request.wait()
  returned:redraw-request = redraw_request.wait()
  scheduled:instant? = none
  kind = ""
  values_equal = false
  values_ordered = false

on inspect
  next_frame = redraw_request.next_frame()
  at = redraw_request.at(redraw_now())
  wait = redraw_request.wait()
  returned = redraw_round_trip(at)
  scheduled = returned.instant
  kind = returned.kind
  values_equal = returned == at
  values_ordered = (next_frame < at) && (at < wait)

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text kind
    text "Next frame < scheduled < wait"
