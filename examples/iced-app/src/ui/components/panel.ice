component Panel(title:str)
  col @w-full gap-3 p-4 bg-surface border border-border rounded-lg
    text title @text-lg font-bold text-foreground
    slot
