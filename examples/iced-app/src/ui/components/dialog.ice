component Dialog()
  container width=480.0 height=shrink max-width=720.0 padding=24.0 border-width=1.0 radius=10.0 @bg-surface border-border
    col spacing=16.0 @w-full
      slot Header
      slot Body
      slot Actions

component Dialog.Header()
  row align=center @w-full
    slot

component Dialog.Body()
  container width=fill
    slot

component Dialog.Actions()
  row spacing=8.0 @w-full
    slot
