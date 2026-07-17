component Dialog()
  container width=480.0 height=shrink max-width=720.0 padding=24.0 @bg-surface border border-border rounded-lg
    col @w-full gap-4
      slot header
      slot body
      slot actions
