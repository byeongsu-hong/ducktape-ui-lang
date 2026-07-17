component Dialog()
  container width=480.0 height=shrink max-width=720.0 padding=24.0 @bg-surface border border-border rounded-lg
    col @w-full gap-4
      slot Header
      slot Body
      slot Actions

component Dialog.Header()
  row @w-full items-center
    slot

component Dialog.Body()
  container width=fill
    slot

component Dialog.Actions()
  row @w-full gap-2
    slot
