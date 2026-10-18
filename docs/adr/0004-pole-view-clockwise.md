# 0004. Planets seen from the pole; everything turns clockwise

## Status
accepted

## Context
In a 2D world the ground under the rocket is the rim of a disc. A
planet drawn as a normal globe (spinning around a vertical axis) would
not match the ground the rocket stands on.

## Decision
- Draw each planet as seen from above its pole: the disc rim is the
  equator, the center is the pole, and the planet spins in the plane.
  Textures are polar (angle x latitude); the side-view terrain samples
  the same elevation field at the equator, so globe and ground agree.
- All orbits and spins go clockwise, so on the pad prograde ("east") is
  to the right of the screen, as players expect.

## Consequences
- Gas giants show concentric bands and rings as full annuli.
- Polar caps sit in the middle of the disc (kept small).
