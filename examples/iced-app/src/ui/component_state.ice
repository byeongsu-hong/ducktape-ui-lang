app ComponentState

theme
  background #111111
  foreground #eeeeee
  primary #3366ff
  danger #cc3333

component Flag(value:str)
  state
    checked = false
  on changed(next)
    checked = next
  col
    text value
    checkbox "Nested" checked=checked -> changed _

component Counter(label:str)
  state
    count = 0
    draft = ""
    enabled = false
  on increment
    count = count + 1
  on changed(next)
    enabled = next
  col
    text label
    text count
    input "Draft" <-> draft
    checkbox "Enabled" checked=enabled -> changed _
    checkbox "Mirror" checked=enabled -> changed _
    Flag value=draft #flag
    button "Increment" -> increment
    match count
      0
        text "zero"
      _
        text draft

view
  row
    Counter label="First" #first
    Counter label="Second" #second
