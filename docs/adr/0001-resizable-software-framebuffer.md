# 0001. Software framebuffer that follows the window size

## Status
accepted

## Context
RETRO SPORTS draws every pixel into a fixed 256x224 `Frame` (its ADR
0001). CENTRIFUGAL wants the same pixel look on modern, resizable
screens, and a planning game needs room for the map and the HUD.

## Decision
Keep the software framebuffer, but give `Frame` a width and height. The
window layer picks an integer pixel scale (frame about 340 px tall, F2 to
change) and resizes the frame to window / scale. Everything is drawn per
pixel into the frame; macroquad only shows it as a nearest-filtered
texture. Long lines are clipped before drawing (`Frame::line_f`).

## Consequences
- Same pixel-perfect look, any window size; headless shots still match.
- All layout must be relative to the frame edges.
- Per-pixel planets cost CPU on big frames; fine at ~640x360 in wasm.
