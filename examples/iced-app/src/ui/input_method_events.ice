app InputMethodEvents

theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

on opened

on preedit(text, start, end)

on commit(text)

on closed

subscribe
  input-method opened -> opened
  input-method preedit status=any -> preedit _ _ _
  input-method commit -> commit _
  input-method closed -> closed

view
  text "Input method events compile fixture"
