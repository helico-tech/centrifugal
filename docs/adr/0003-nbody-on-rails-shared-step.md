# 0003. Bodies on rails, n-body rockets, one shared step function

## Status
accepted

## Context
The game must feel like real orbital planning: Newtonian gravity for the
rocket from all bodies, but fixed, predictable planet orbits. Research
(`docs/research/ux-and-mechanics.md`) shows predicted paths that drift
from the real flight ruin planning.

## Decision
- Planets and moons follow Kepler orbits as a function of time.
- Rockets feel the gravity of every body. The acceleration of the frame
  body (the planet whose neighborhood the rocket is in, or the star)
  from the bodies below it is subtracted (the indirect term), because
  rails bodies never feel their moons.
- Flight and prediction use the same RK4 `step`. While coasting, the
  step size comes from `coast_dt(state)` only. The flight keeps its last
  step ahead of the clock and the screen interpolates (Hermite).
- Burns step at 1/60 s. Planned burns in the prediction hold one fixed
  direction from burn start; the node autopilot flies the same.

## Consequences
- The coasting path on screen is the flown path (tested).
- Time warp to 1,000,000x stays cheap: a few hundred steps per orbit.
- Fly-bys are chaotic, so the path fades after the first encounter.
