app WindowEvents

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

state
  listen_frames = true

on frame

on opened(x, y, width, height)

on closed

on moved(x, y)

on resized(width, height)

on rescaled(scale)

on close_requested

on focused

on unfocused

on file_hovered(path)

on file_dropped(path)

on files_hovered_left

subscribe
  window frame when listen_frames -> frame
  window opened -> opened _ _ _ _
  window closed -> closed
  window moved -> moved _ _
  window resized -> resized _ _
  window rescaled -> rescaled _
  window close-request -> close_requested
  window focused -> focused
  window unfocused -> unfocused
  window file-hovered -> file_hovered _
  window file-dropped -> file_dropped _
  window files-hovered-left -> files_hovered_left

view
  text "Window events compile fixture"
