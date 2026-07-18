app DebugTiming

theme
  background #111827
  foreground #f9fafb
  primary #60a5fa
  danger #f87171

state
  timer:debug-span? = none
  label = "interaction"
  value = 41
  measured = 0

on begin
  debug start label -> timer

on finish
  debug finish timer

on compute
  measured = debug.time_with("compute", value + 1)

view
  col @p-4 gap-2
    button "Begin" -> begin
    button "Finish" -> finish
    button "Compute" -> compute
    if debug.active(timer)
      text "Timing"
    text measured
