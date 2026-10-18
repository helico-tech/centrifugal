---
status: todo
type: debt
created: 2026-10-18
---
# No balance bot for M-05, M-06, M-09, M-10 (M-08 partly)

## Problem
Their tanks come from delta-v estimates only, not a flown check.

## Notes
Add bots: deorbit to pad, booster boostback, moon landing (Transfer + Hop), moon return, interplanetary landing. Tune margins in mission.rs.

M-08 has a bot (Transfer + Descent): it lands softly with 10% fuel but does not aim, so it lands ~14 km from the LZ. After that run the lander got 2.0x landing delta-v and the LZ radius went to 3 km. An aiming descent would confirm the margin.
