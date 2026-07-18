daemon BackgroundAgent
  title "Background agent"
  theme "dark"
  scale-factor 1.0
  window dashboard
    size 800 600
    position centered

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

view
  col @p-6 gap-4
    text "Background agent" @text-xl font-bold
    button "Quit" style=danger -> quit
