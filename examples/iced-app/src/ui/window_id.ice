app NativeWindowId

extern crate::backend
  sync window_id_round_trip(value:window-id) -> window-id

theme
  background #111827
  foreground #f9fafb
  primary #60a5fa
  danger #f87171

state
  first:window-id = window_id.unique()
  second:window-id = window_id.unique()
  returned:window-id = window_id.unique()
  first_display = ""
  values_differ = false
  values_ordered = false

on inspect
  first = window_id.unique()
  second = window_id.unique()
  returned = window_id_round_trip(first)
  first_display = first.display
  values_differ = first != second
  values_ordered = first < second

view
  col spacing=8.0 padding=16.0
    button "Inspect" -> inspect
    lazy first as cached
      text cached.display
