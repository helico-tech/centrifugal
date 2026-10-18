# CENTRIFUGAL: research on orbital-game UX and mechanics

Date: 2026-10-18. Scope: what other orbital games do well and badly, what makes
orbital missions good levels, and the traps of n-body trajectory prediction.
Points marked *(own analysis)* are our own reasoning, not taken from a source.

---

## 1. Game UX in orbital games

### 1.1 Trajectory prediction display
- **KSP 1** draws "patched conics": one ellipse per sphere of influence (SOI,
  the zone where one body's gravity counts). The leg after an encounter gets its
  own colour and an encounter node. When you reach that node, the game switches
  the reference body ([KSP wiki: Encounter](https://wiki.kerbalspaceprogram.com/wiki/Encounter)).
  How the legs are drawn depends on the chosen viewpoint. A free-return
  figure-eight only looks right in the moon-relative mode, and to get that you
  have to edit settings.cfg ([forum](https://forum.kerbalspaceprogram.com/topic/13902-apollo-13-movie-orbit),
  [forum](https://forum.kerbalspaceprogram.com/topic/127379-2-questions)).
- **KSP 2** players say its map is harder to read than KSP 1's. Encounter
  markers only show up once an encounter exists. Pe altitude only shows on
  hover. The SOI rings are visual noise
  ([forum](https://forum.kerbalspaceprogram.com/topic/220522-suggestion-improvements-to-map-view-and-orbital-planning)).
- **Principia** (the n-body mod for KSP) lets the player choose a **plotting
  frame**: body-surface-fixed (for launch and landing), non-rotating (for
  orbit work), body-pair rotating (for moon transfers), Lagrange (pulsating,
  shows equipotentials), and target-centred (for rendezvous, with
  closest-approach markers). It draws three lines: history, prediction without
  burns, and the flight plan. Burns are solid and coasts are dashed. A
  periapsis below the surface gets an impact marker
  ([Principia Concepts](https://github.com/mockingbirdnest/Principia/wiki/Concepts)).
  One flaw: in the target frame you cannot see whether you will hit a body on
  the way ([forum](https://forum.kerbalspaceprogram.com/topic/202583-surface-fixed-and-rendezvous-reference-frames)).
- **Puzzle games** always show the future path before you commit. In *Orbit:
  Playing with Gravity* you see a planet's path before you launch it, and every
  planet leaves a coloured trail ([App Store](https://apps.apple.com/us/app/orbit-playing-with-gravity/id991457817)).
  *Gravity Assist* (osushi) shows a dotted preview line while you aim
  ([itch.io](https://osushi.itch.io/gravity-assist)).
- **Takeaway** *(own analysis)*: draw each part of the path relative to the
  body that dominates it, and colour it by that body. At a closest approach,
  draw a "ghost" of the body where it will be at that moment. Mark impacts
  clearly.

### 1.2 Maneuver nodes and editing
- KSP's maneuver node (a planned burn at a point on the path, shown with
  drag handles) is powerful, but people struggle with it:
  - One player took 40+ hours to learn that nodes go on the orbit line
    ([forum](https://forum.kerbalspaceprogram.com/topic/223257-feedback-on-orbital-maneuvers-and-ui)).
  - You often have to edit a node while looking at a far-away target, so you
    fight the camera to keep both on screen. While dragging you cannot see
    the new Ap/Pe. The burn bar is too coarse to stop the burn on time (same thread).
  - In KSP 1 the handles point along the post-burn orbit but change the
    pre-burn orbit, so normal/anti-normal feels backwards
    ([bug 3953](https://bugs.kerbalspaceprogram.com/issues/3953)).
  - KSP 2 uses finite burns (burns that take time) but kept KSP 1's
    "start half the burn early" timer. Players overshot their burns or saw a
    0 s timer
    ([forum](https://forum.kerbalspaceprogram.com/topic/222273-the-way-that-maneuver-nodes-seemingly-work-is-quite-silly)).
  - Players asked for scroll-wheel fine tuning and number entry. Mods like
    PreciseNode fill that gap ([PreciseNode](https://github.com/blizzy78/ksp-precisenode),
    [forum](https://forum.kerbalspaceprogram.com/topic/219842-precise-maneuver-node-editor-suggestion)).
- Principia models each node as a finite burn with a Δv and a chosen frame.
  You can enter it as prograde/normal/radial parts. It warns that tiny timing
  or Δv errors grow large later, so it expects you to make correction burns. It
  also has a local optimiser that can fail or return a silly high-Δv answer
  ([Concepts](https://github.com/mockingbirdnest/Principia/wiki/Concepts)).
- Children of a Dead Earth automates most burns and auto-intercepts. Players
  liked the planning, but still found the UI fiddly with too much
  micromanagement ([Steam discussion](https://steamcommunity.com/app/476530/discussions/0/343788552534266122),
  [Orbiter forum](https://orbiter-forum.com/goto/post?id=544331)).

### 1.3 Time warp
- KSP: "Warp to next maneuver" runs at full speed and stops about 1 minute
  before the burn ([TrueAchievements guide](https://www.trueachievements.com/game/Kerbal-Space-Program/walkthrough/2)).
  Auto-warp refused to cross SOI borders, and high warp could overshoot them
  and change the orbit ([forum](https://forum.kerbalspaceprogram.com/topic/121210-cannot-auto-warp-so-far),
  [bug 1043](https://bugs.kerbalspaceprogram.com/issues/1043)). Players
  install *Kerbal Alarm Clock* to set alarms for events.
- Spaceflight Simulator (SFS) has "Timewarp Here": click a point on the orbit
  line to jump there. A click near an encounter or rendezvous marker snaps to
  that marker. You cannot steer or burn during warp, and warp is limited by
  altitude ([SFS wiki: Time warp](https://spaceflight-simulator.fandom.com/wiki/Time_warp)).
- Children of a Dead Earth is close to turn-based: plan, then advance time in
  chunks from 1 minute up to days ([Steam discussion](https://steamcommunity.com/app/476530/discussions/0/343788552534266122)).

### 1.4 Assists
- KSP SAS (the attitude-hold autopilot) can hold prograde, retrograde,
  normal, radial, target, or the maneuver direction. Its navball switches
  between Surface, Orbit and Target speed. Below about 1 m/s the hold modes
  wobble, so SAS falls back to plain stability hold
  ([KSP wiki: SAS](https://wiki.kerbalspaceprogram.com/wiki/Stability_Augmentation_System),
  [Navball](https://wiki.kerbalspaceprogram.com/wiki/Navball),
  [forum](https://forum.kerbalspaceprogram.com/topic/141752-why-does-the-sas-target-hold-not-always-point-to-the-target)).
- A "suicide burn" (hoverslam) is a full-power braking burn timed to reach
  zero speed just at the ground. The MechJeb and KER mods show a countdown
  to it, and players start the burn when it reaches zero. KER's number is
  only right for vertical falls. MechJeb's corrects for the flight-path angle
  ([forum](https://forum.kerbalspaceprogram.com/topic/155599-suicide-burn-code),
  [forum](https://forum.kerbalspaceprogram.com/topic/177136-how-do-i-know-the-suicide-altitude)).
- The *Trajectories* mod shows the predicted impact point on the ground for
  aerobraking and precise landings. It is good to a few km, and less good
  if you change attitude ([forum](https://forum.kerbalspaceprogram.com/topic/162324-112x-trajectories-v245-2023-08-22-atmospheric-predictions)).
- Outer Wilds keeps piloting manual but gives you lock-on, match velocity,
  a landing camera and a basic autopilot. Players love match velocity. The
  autopilot flies straight, even into the sun, so it is clearly a tool with
  limits ([Steam discussion](https://steamcommunity.com/app/753640/discussions/0/3110266679785540459)).
- ΔV: Rings of Saturn lets you choose between autopilot help and manual
  thrusters that save fuel. The trade is fuel against convenience
  ([GamingOnLinux](https://www.gamingonlinux.com/2019/08/hard-sci-fi-space-sim-rings-of-saturn-has-entered-early-access-on-steam)).

### 1.5 Onboarding
- In KSP 1 the tutorials came late and the first hours are "brutal". Players
  also had to budget Δv by hand. Funny Kerbals made failure feel less bad
  ([GameGeeker review](https://gamegeeker.com/games/kerbal-space-program/review?market=JP)).
- KSP 2 added animated videos plus hands-on lessons (Missing the Ground,
  Orbits are Weird, Orbital Transfers). One reviewer liked that they explain
  the jargon ([PCGamesN](https://www.pcgamesn.com/kerbal-space-program-2/review)).
  Another had to replay most of them to get it ([GodIsAGeek](https://www.godisageek.com/2023/02/kerbal-space-program-2-is-more-ambitious-than-ever-before-hands-on-preview/)).
- Juno: New Origins has interactive tutorials, yet players still call it
  less intuitive and say its realistic physics is steep for beginners
  ([Steam](https://store.steampowered.com/app/870200/Juno_New_Origins/),
  [space.com](https://www.space.com/juno-new-origins-game-launch-rockets-build-planets)).
- *Orbit* adds one new mechanic at a time (repulsive black holes, planets
  that pull on each other) ([App Store](https://apps.apple.com/us/app/orbit-playing-with-gravity/id991457817)).
  *Gravity Ghost* has no fail state ([PC Gamer](https://pcgamer.com/gravity-ghost-preview),
  [GDC postmortem](https://gdcvault.com/play/1022023/Gravity-Ghost-A)).

### 1.6 Failure, retry, camera, HUD
- The KSP family has quicksave and "revert to launch". Many web landers just
  restart the level in seconds. The Falcon-landing Scratch game was called
  "infuriating" because it chose realism over fun ([TNW](https://thenextweb.com/news/its-not-as-easy-as-it-looks),
  [Vice](https://motherboard.vice.com/read/just-try-to-land-a-spacex-rocket-on-a-drone-boat-in-this-infuriating-game)).
- Camera *(own analysis, based on the games above)*: KSP and SFS switch
  between a flight view and a map view. Players say editing a node in the
  map means fighting the camera (see 1.2). Our continuous zoom removes that
  switch. Zoom still needs "focus" targets (rocket, body, target) and icons
  that never shrink below a minimum size.
- HUD values that matter, going by what the mods add
  ([KER/MechJeb threads above](https://forum.kerbalspaceprogram.com/topic/155599-suicide-burn-code)):
  altitude above the ground, vertical and horizontal speed, Ap/Pe, Δv left,
  TWR (thrust-to-weight ratio), burn time and time to the node, time to
  impact, and the suicide-burn countdown.

## 2. Mechanics that make orbital missions good levels

- **Pick one constraint per level**: fuel, time, parts, or a surprise event.
  This is what KSP Making History's mission builder offers. Its built-in
  missions ("Land on the Mun") felt too simple, so add a twist or a scoring
  axis ([PCWorld](https://www.pcworld.com/article/401868/kerbal-space-program-making-history-expansion-review.html),
  [Neowin](https://www.neowin.net/news/kerbal-space-program-making-history-expansion-announced---free-for-early-supporters)).
- **Score in layers**: the classic *Lunar Lander* (1979) has landing pads
  with 2x-5x multipliers, and the smaller pads pay more. It adds a fuel bonus
  on top, and fuel is also spent on rotation
  ([Wikipedia](https://en.wikipedia.org/wiki/Lunar_Lander_(1979_video_game))).
  This is the same idea as stars for fuel left and precision.
- **Stars**: *Gravity Assist* gives up to 3 stars based on how few attempts
  you needed ([itch.io](https://osushi.itch.io/gravity-assist)). *(Own
  analysis)*: attempt-based stars punish trying things out, which clashes
  with planning. Prefer outcome-based stars (fuel left, accuracy, time).
- **Gravity assists as puzzles**: several indie games build levels around
  slingshots with limited fuel. Later levels chain several flybys ([osushi
  Gravity Assist](https://osushi.itch.io/gravity-assist),
  [Slingshot](https://hoowa.itch.io/slingshot),
  [Gravity Matters](https://rawg.io/games/gravity-matters)). Matching orbits
  to refuel satellites also works as a puzzle ([GravAssist](https://sgykwi.itch.io/gravassist)).
- **Emergent mechanics** *(own analysis)*: our bodies move on rails and the
  rocket feels n-body gravity. So gravity assists, free returns, Lagrange
  points (in the rocket's frame) and moon perturbations all happen for free,
  with no special code. Children of a Dead Earth sells this as a feature
  ([Steam](https://store.steampowered.com/app/476530/)).
- **Possible level mechanics, cheapest first** *(own analysis)*:
  - Target-orbit windows (Ap/Pe band, inclination is not needed in 2D).
  - Landing on a pad on a rotating body. Booster return-to-launch-site (RTLS).
  - A drone ship that moves along the surface or orbit.
  - Rendezvous with a station. Moving targets.
  - Free return around a moon. Gravity assist. Lagrange point station-keeping.
  - Later: atmosphere drag (aerobraking), reentry heat limit, asteroid
    hazards, rocket or stage choice before launch.
- **Make gravity readable and fair** *(own analysis plus the puzzle games)*:
  - Always show the predicted path.
  - Show each body's influence zone faintly. The Hill radius
    r ≈ a·∛(m/3M) is a good choice ([Wikipedia: Hill sphere](https://en.wikipedia.org/wiki/Hill_sphere)).
  - Use generous success windows, and check them over time (for example,
    "hold the orbit for one full revolution").
  - Keep chaotic close flybys out of early levels.
- **Difficulty curve for 5-15 levels** *(own analysis from the sources above)*:
  introduce one new idea per level and test it again in a later "remix"
  level. Teach the maneuver node early and say so out loud, because KSP's
  hardest UX failure was that people never found it.

## 3. N-body prediction (no patched conics)

- **The prediction only stays right if it is the same computation as
  flight.** Prediction drifts from the real flight when the predictor uses
  bigger steps or a different method than the flight sim. Orbiter's IMFD is
  one example ([Orbiter forum](https://orbiter-forum.com/goto/post?id=625020)).
  *(Own analysis)*: in Centrifugal only the rocket is integrated. Bodies are
  exact functions of time. So prediction and flight can call the same step
  function with the same dt sequence. Then the prediction is exact during a
  coast, bit for bit. The flight can even replay the predicted buffer.
- **Integrator choice**: leapfrog / velocity Verlet is symplectic (it
  conserves a slightly changed energy, so orbits do not slowly decay or
  grow) and time-reversible. It is 2nd order, and it needs a fixed step to
  stay stable. RK4 drifts in energy over long runs. Yoshida's method gives a
  4th-order version for 3 force evaluations per step
  ([Wikipedia: Leapfrog](https://en.wikipedia.org/wiki/Leapfrog_integration)).
  Close flybys need smaller steps. Principia lets the user set a tolerance
  and a max step count per segment, and warns that a tight tolerance costs a
  lot of CPU ([Concepts](https://github.com/mockingbirdnest/Principia/wiki/Concepts)).
  Symmetric multistep methods (used for planets) can go unstable at some
  step sizes ([Quinlan 1999](https://arxiv.org/abs/astro-ph/9901136)). That
  is not needed here.
- **Chaos and sensitivity**: Principia says errors of a fraction of a second
  or a few mm/s grow large downstream. It limits Δv input to about 1 mm/s
  and expects correction burns ([Concepts](https://github.com/mockingbirdnest/Principia/wiki/Concepts)).
  *(Own analysis)*: after the first close encounter the prediction is only a
  hint. Fade it out, and recompute every frame from the current state so it
  corrects itself.
- **Which frame to draw in**: Principia solves this by letting the player
  pick a plotting frame (see 1.1). Players on the forums still find that
  complex. *(Own analysis)*: default to an automatic frame. Draw each path
  sample relative to the body that dominates at that moment (the strongest
  pull, or the smallest Hill sphere that contains the rocket). Add
  hysteresis (a margin before switching back) so it does not flicker at the
  border. Offer a manual override: pinned body, surface-fixed for landing,
  target-relative for rendezvous.
- **Apses in n-body**: osculating Ap/Pe (computed from the current position
  and speed, as if only one body pulled) wobble when other bodies pull too.
  *(Own analysis)*: in the HUD, show osculating Ap/Pe relative to the
  dominant body. On the path, put markers where the predicted distance hits a
  real minimum or maximum.
- **Finite burns**: KSP 2 shows that faking burns as instant impulses breaks
  the timers. Principia integrates the real finite burn
  ([Concepts](https://github.com/mockingbirdnest/Principia/wiki/Concepts)).
  *(Own analysis)*: simulate the planned burn in the prediction with the
  same attitude hold the autopilot will fly. Then the plan matches what
  happens.
- **Precision and cost** *(own analysis)*: use f64 positions, and draw
  everything relative to the camera. In wasm, cap prediction work per frame
  (for example, a few thousand steps) and extend the path over several
  frames. Only recompute from scratch when thrust, the plan or the warp
  changes.

---

## Recommendations for Centrifugal

### Must-have for v1
1. One shared, deterministic step function (velocity Verlet, f64) for flight and prediction, so the path never drifts while coasting.
2. Pick dt from the state, never the frame rate (smaller near bodies), so warp and prediction step identically.
3. Always show the predicted path, coloured by dominant body. Fade it after the first close encounter. Chaos makes later parts unreliable.
4. Auto reference frame per path segment (dominant body, with hysteresis), plus a "pin frame" key. Moon transfers stay readable without Principia-style menus.
5. Markers for closest approach (time and altitude), impact (big red X), and Ap/Pe, with a ghost of the body at encounter time. These are the core planning signals.
6. Maneuver nodes: click the path to add one, drag prograde/radial handles, use scroll or keys for fine steps, and see live Ap/Pe and closest approach while editing. This fixes KSP's top complaints.
7. Finite burns in the plan, plus an "execute node" autopilot that flies exactly the planned burn. The plan then matches the flight, and there is no half-burn timer bug.
8. SAS hold modes: stability, prograde, retrograde, radial in/out, target, node. These are the cheapest assist with the biggest gain.
9. Landing aids: suicide-burn countdown (with the flight-path angle correction), predicted impact point on the surface, and a surface-relative speed readout. Pad landings become learnable.
10. Time warp: warp-to-node, warp-to-closest-approach, click-path "warp here" (snaps to markers), and auto drop to 1x before events or a dominant-body switch. This is what SFS and KSP mods proved works.
11. Instant restart, quicksave/quickload, and a rewind ring buffer. It is cheap because bodies are on rails and the rocket state is tiny. Failure must cost seconds, not minutes.
12. Continuous log zoom with focus cycling (rocket, dominant body, target, whole system). Bodies, rocket and target never shrink below a minimum icon size, so nothing gets lost when you zoom out.
13. Compact HUD: altitude, vertical and horizontal speed, Ap/Pe, Δv left, TWR, time to node and burn time, time to impact, dominant body, warp factor, and an objective checklist with live status.
14. Outcome-based 3 stars: complete, then fuel-left threshold, then precision or time. Never count attempts, because that punishes experimenting.
15. Generous success windows checked over time (for example, Pe in a band for one full orbit), so success feels fair, not pixel-perfect.
16. A campaign that adds one mechanic per level: hop and land, reach orbit, circularize a satellite, booster return to the pad, deorbit to the pad, moon flyby (free return), moon landing, moon return, rendezvous or drone ship, interplanetary capture, gravity assist.
17. Teach by doing inside the first levels: short contextual hints, and an explicit "place your first node" step. KSP's late tutorials and hidden nodes are its biggest onboarding failure.

### Later
18. Lagrange or rotating frame view, and L-point levels. They emerge from the physics, but players only read them in a rotating frame.
19. Atmosphere drag, aerobraking and a reentry heat limit. They add depth, but they add a second prediction model.
20. Simple node optimiser ("hit periapsis X at body Y"), shown as a suggestion. Principia shows a local optimiser can fail, so keep it optional.
21. Rocket or stage choice before a level as a constraint. It adds variety, but needs a parts UI.
22. Hazards (asteroid fields), moving drone ships with waves, and timed launch windows. Use these for remix and challenge levels.
23. Per-level replay ghost of your best run and a local leaderboard by stars or fuel. Cheap replay value, because the sim is deterministic.
24. Seeded procedural "daily system" challenges. They need a solver to check that a level can be solved, so leave them until after the campaign is done.
