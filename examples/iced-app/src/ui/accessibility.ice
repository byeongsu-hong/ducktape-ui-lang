app Accessibility

theme
  background #111827
  foreground #f9fafb
  primary #60a5fa
  danger #f87171

state
  name = ""
  secret = ""
  accepted = false

on toggle(value)
  accepted = value

on submit

view
  col spacing=12.0 padding=16.0
    text "Accessible form"
    input "Name" #name label="Full name" description="Name used on your profile" hint="Ada" <-> name
    input "Password" #password label="Account password" description="Password text is never exported" secure=true <-> secret
    checkbox "Accept terms" #terms label="Terms consent" description="Required before submission" checked=accepted -> toggle _
    button "Submit" #submit description="Save the accessible form" disabled=empty(trim(name)) -> submit
    button #help label="Open help" description="Show keyboard and screen-reader help" -> submit
      text "?"
    image "assets/demo.ppm" label="Ice accessibility example" description="A decorative sample promoted into the accessibility tree"
