# House style

CENTRIFUGAL follows the RETRO SPORTS house style (`../retro-sports/docs/house-style.md`):
a 1992 HELICO SOFT cart. Differences and additions:

## Screen
- The frame size follows the window: window size / pixel scale, the
  scale picked so the frame is about 340 px tall (F2 changes it). Lay
  out the HUD from the frame's edges, never from fixed sizes; check at
  640x360 (the `--shots` size) and wider.
- Colors through `rgb()` (15-bit grid). Gradients are bands with a
  dithered seam (`body::band`), not smooth.
- Rockets, trees and buildings are polygons in a local frame with the
  navy outline, so they stay crisp at any angle and zoom.

## Look of space
- Planets are lit globes seen from the pole (ADR 0004), posterized into
  five light levels with ordered dithering, city lights and lava glow on
  the night side, clouds on top, an atmosphere halo of sky bands.
- Close to the ground the same planet becomes a side view: grass, dirt,
  wavy rock layers, sea with a bright surface line, sky by altitude and
  sun angle with sunset colors, puffy cloud band, trees by biome.
- The sky is a colorful dithered nebula with twinkling stars.

## Screens
Title (spinning home world, logo, ticker, (C) 1992 HELICO SOFT), mission
board, briefing card with tips, flight, results with stars. Whimsy: cows
at the pad, silly mission taglines, failure quips, "RAPID UNSCHEDULED
DISASSEMBLY".
