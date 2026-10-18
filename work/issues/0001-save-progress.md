---
status: todo
type: idea
created: 2026-10-18
---
# Stars are not saved

## Problem
Mission stars live in memory only; a reload (web) or restart loses them.

## Notes
Native: small file next to the binary or in the user data dir. Web: needs a localStorage plugin for macroquad (quad-storage) or a tiny JS shim.
