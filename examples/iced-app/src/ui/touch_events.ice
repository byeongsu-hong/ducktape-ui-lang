app TouchEvents

theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000

on pressed(finger, x, y)

on moved(finger, x, y)

on lifted(finger, x, y)

on lost(finger, x, y)

subscribe
  touch pressed status=ignored -> pressed _ _ _
  touch moved -> moved _ _ _
  touch lifted -> lifted _ _ _
  touch lost -> lost _ _ _

view
  text "Touch events compile fixture"
