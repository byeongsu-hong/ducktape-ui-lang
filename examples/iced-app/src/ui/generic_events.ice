app GenericEvents

extern crate::backend
  sync event_name(value:event) -> str
  sync event_label(value:event) -> str?

theme
  bg #111827
  fg #f9fafb
  primary #60a5fa
  danger #f87171

state
  last = "none"
  last_window:window-id? = none

on received(value)
  last = event_name(value)

on labeled(value)
  last = value

on identified(id, value)
  last_window = some(id)
  last = event_name(value)

subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id -> identified _ _

view
  text last
