# CENTRIFUGAL game design

## Pitch
Launch rockets from a procedural home world, plan burns on the live
predicted path, and fly missions that grow from a pad hop to a gas giant.
Quasi-realistic: Newtonian n-body flight, but tiny planets (home is
30 km in radius), short orbits and big time warp, so a launch takes a
minute and a planet trip a few seconds of warp.

## Scale (game numbers)
- Star GM 4e12; planets 1.5-20 Mm from the star; home radius 30 km,
  g 9.8, air to 6 km, low orbit ~9 km at 480 m/s, period ~8 min.
- Home moon at 180 km, radius 11 km, g 2.1.
- Engines have low Isp (140-190 s), so tanks still look rocket-sized.

## What the player gets (from the research)
- Always-on predicted path, colored per dominant body, each part drawn
  around its body; ghosts of bodies at encounter time; Ap/Pe, impact,
  closest-approach markers. Near the ground the path turns with the
  planet, so the impact X is where you really land.
- Maneuver nodes: click the path, drag prograde/radial handles (Shift
  fine, wheel 1 m/s), live path update, N auto-burn (orients, warps
  close, burns, stops).
- Attitude hold: hold, prograde, retrograde, radial in/out, node (keys 1-6,
  buttons). Surface-relative low over a planet, orbital higher up.
- Warp: `,` `.`, G warps to the next event (node, Ap, Pe, encounter,
  closest), auto drop-out before encounters, impact and air.
- Landing helper: impact countdown, suicide-burn countdown with a bar,
  arrow and distance to the pad or landing zone.
- Rewind (R) a few seconds at any time, also after a crash; instant
  restart. Stars come from outcome (fuel left, precision), never from
  attempts.
- Camera: wheel zoom at the cursor from 0.1 m/px to the whole system,
  drag to pan, F cycles focus (rocket, body, target, sun), M map. Near
  the rocket local "up" is up; zoomed out north is up.

## Campaign (all systems; bodies by role)
| Code | Mission | Teaches |
|---|---|---|
| M-01 | GRASSHOPPER: hop to 1 km, land on pad | throttle, landing |
| M-02 | SPACE TOURIST: leave the air, chute down | staging, chute, path |
| M-03 | ROUND AND ROUND: reach orbit | gravity turn, first node |
| M-04 | COMSAT: deploy in 15-20 km orbit | raising orbits |
| M-05 | COMING HOME: deorbit near the pad | impact marker, reentry |
| M-06 | BOOSTER BACK: stage high, land the booster | boostback, suicide burn |
| M-07 | MOON SHOT: orbit the moon | targets, encounters |
| M-08 | ONE SMALL HOP: land on the moon LZ | airless landing |
| M-09 | THERE AND BACK AGAIN: moon to home | escape, aerobraking |
| M-10 | RED PLANET: land on the neighbor | interplanetary |
| M-11 | BIG EYE: orbit the gas giant | long transfers, flybys |

Rocket tanks are sized in `mission.rs` from the delta-v each mission
needs in the current system (Hohmann + escape/capture estimates with a
margin), so "N: NEW SYSTEM" on the board re-rolls the whole galaxy and
the campaign still works.

## Balance (bots, `--bots`, campaign seed 7)
| Mission | Bot result | Fuel left |
|---|---|---|
| M-01 | landed on pad, 3 stars | 28% |
| M-02 | space and back, max alt ~10 km | - |
| M-03 | orbit | 38-49% |
| M-04 | deployed at 17.4 x 18.7 km | ~0% upper stage |
| M-07 | moon orbit after 1 correction | 32% |
| M-11 | giant orbit | 37% |
| M-08 | soft moon landing, but no aiming (14 km off) | 10% (before the 2.0x lander tank) |
Targets: a decent player finishes with the bot's margin roughly halved.
M-05, M-06, M-09, M-10 have no bot yet (issue 0004).
