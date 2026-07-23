app ResizablePanes

theme
  bg #0f172a
  surface #111827
  fg #f8fafc
  muted #94a3b8
  border #334155
  primary #7c3aed
  danger #dc2626

state
  left_width = 240.0
  dragging = false

on drag_started
  dragging = true

on drag_ended
  dragging = false

on divider_dragged(dx, dy)
  return if dx < 0.0 && left_width + dx < 160.0
  left_width = left_width + dx

view
  row width=fill height=fill
    container width=left_width height=fill bg=surface padding=12.0
      text "Sidebar" size=14.0 @text-fg
    resize-handle drag=divider_dragged press=drag_started release=drag_ended cursor=resize-horizontal
      container width=6.0 height=fill bg=border
        text ""
    container width=fill height=fill bg=bg padding=12.0
      col spacing=8.0
        text "Main" size=14.0 @text-fg
        text left_width size=12.0 @text-muted
        if dragging
          text "Dragging divider" size=12.0 @text-muted
