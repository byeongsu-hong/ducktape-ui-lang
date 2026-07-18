app NativeRotation

extern crate::backend
  sync rotation_round_trip(value:rotation) -> rotation

theme
  background #111827
  foreground #f9fafb
  primary #60a5fa
  danger #f87171

state
  pixel:image = rgba(1, 1, bytes(ff 00 ff ff))
  default_rotation:rotation = rotation.default()
  floating_rotation:rotation = rotation.default()
  solid_rotation:rotation = rotation.default()
  adjusted_rotation:rotation = rotation.default()
  round_trip:rotation = rotation.default()
  applied_size:size = size.zero()
  radians_value:radians = radians(0.0)
  degrees_value:degrees = degrees(0.0)
  kind = ""
  equal = false

on inspect
  default_rotation = rotation.default()
  floating_rotation = rotation.floating(radians(0.25))
  solid_rotation = rotation.solid(radians(0.5))
  adjusted_rotation = rotation.with_radians(floating_rotation, radians(0.75))
  round_trip = rotation_round_trip(rotation.from(0.2))
  applied_size = rotation.apply(solid_rotation, size(10.0, 20.0))
  radians_value = adjusted_rotation.radians
  degrees_value = adjusted_rotation.degrees
  kind = solid_rotation.kind
  equal = default_rotation == rotation.floating(radians(0.0))

view
  col spacing=8.0 padding=16.0
    button "Inspect" -> inspect
    image pixel width=48.0 height=48.0 rotation=solid_rotation
    svg "<svg/>" memory width=48.0 height=48.0 rotation=adjusted_rotation
    text kind
    text radians_value.value
    text degrees_value.value
