daemon BackgroundAgent
  title daemon_title(window)
  theme daemon_theme(window)
  scale-factor daemon_scale(window)
  renderer crate::backend::AppRenderer
  window dashboard
    size 800 600
    position centered

extern crate::backend
  sync daemon_title(id:window-id) -> str
  theme daemon_theme(id:window-id)
  sync daemon_scale(id:window-id) -> f64

theme
  background #0f172a
  foreground #f8fafc
  primary #7c3aed
  danger #dc2626

state
  dashboard:window-id? = none

on mount
  task window open dashboard -> opened _

on opened(id)
  dashboard = some(id)

on quit
  exit

component AgentWindow(id:window-id)
  col spacing=16.0 padding=24.0
    text daemon_title(id) size=20.0 @font-bold
    button "Quit" style=danger -> quit

view
  AgentWindow id=window
