app NativeEventStatus

extern crate::backend
  sync status_round_trip(value:event-status) -> event-status

theme
  bg #111827
  fg #f9fafb
  primary #60a5fa
  danger #f87171

state
  ignored:event-status = event_status.ignored()
  captured:event-status = event_status.captured()
  returned:event-status = event_status.ignored()
  ignored_then_ignored:event-status = event_status.captured()
  ignored_then_captured:event-status = event_status.ignored()
  captured_then_ignored:event-status = event_status.ignored()
  captured_then_captured:event-status = event_status.ignored()
  kind = ""
  values_equal = false

on inspect
  ignored = event_status.ignored()
  captured = event_status.captured()
  returned = status_round_trip(event_status.captured())
  ignored_then_ignored = event_status.merge(ignored, ignored)
  ignored_then_captured = event_status.merge(ignored, captured)
  captured_then_ignored = event_status.merge(captured, ignored)
  captured_then_captured = event_status.merge(captured, captured)
  kind = returned.kind
  values_equal = returned == captured

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text kind
    text "Captured wins when statuses merge"
