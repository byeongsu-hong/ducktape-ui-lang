component Panel(title:str)
  col spacing=12.0 padding=16.0 @w-full bg-surface border border-border rounded-lg
    text title size=18.0 @font-bold text-fg
    slot
