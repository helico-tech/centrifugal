# Architecture

One crate: a library with all game logic and drawing, and a thin binary
(`src/main.rs`) that owns the window, keyboard, mouse and speakers through
macroquad. The same binary builds for the browser (ADR 0002).

## Layout
```
src/
  main.rs       window, input, pixel scale, sound; --shots and --bots
  headless.rs   scripted screenshots and bot balance runs
  engine/       no game rules
    gfx.rs      Frame (any size), primitives, clipped lines, 5x7 font, logo()
    input.rs    Input: keys and mouse for one tick
    noise.rs    value noise, fbm, ridged noise
    synth.rs    chiptune synth (from RETRO SPORTS)
    v2.rs       f64 2D vector (world meters)
    rng.rs ui.rs png.rs
  sim/          the universe, no drawing
    system.rs   bodies on Kepler rails, states at time t, dominant body
    worldgen.rs procedural systems: star, planets, moons, looks, names, sites
    surface.rs  elevation field of a solid body (shared by globe and terrain)
    terrain.rs  equator heights (the ground rockets stand on)
    names.rs    procedural names
    physics.rs  n-body gravity (+ indirect term), drag, RK4 step, step size
    predict.rs  trajectory prediction with nodes and events
    vehicle.rs  stages, delta-v, tank sizing
  game/
    flight.rs   one flight: craft, staging, landing/crash, warp, SAS, nodes,
                node autopilot, rewind snapshots
    mission.rs  campaign missions, goals, stars, tank sizing per system
    play.rs     flight screen: controls, mouse, camera, HUD
    app.rs      screens: title, mission board, briefing, results
    fx.rs       particles (fire, smoke, plasma, explosions)
    audio.rs    all sounds and songs
    bot.rs      scripted pilots (ascent, hop, transfer)
  render/
    camera.rs   world <-> screen with zoom and rotation
    globe.rs    polar planet textures (pole view, see ADR 0004)
    body.rs     per-pixel planet: globe, side-view ground, sky, clouds, rings
    space.rs    star field, nebula, sun, orbit lines
    scenery.rs  trees, launch complex, landing zones, cows
    rocket.rs   rockets from stages, outlined; icon when far
    traj.rs     predicted path, markers, ghosts, node handles
    local.rs    drawing in a rotated local frame
    text.rs     number formatting
```

## Key rules
- Everything visible goes through `Frame`; headless shots equal the window.
- The library never reads the clock or the window. Same seed and input,
  same game.
- Bodies move only by the clock (Kepler rails); only rockets are
  integrated (ADR 0003).
- Flight and prediction share `physics::step` and `coast_dt`: the step
  size depends on the state, never on frame rate or warp. While coasting
  the drawn path is exactly the flown path (`tests/physics.rs`).
- Gravity includes the indirect term for the frame body, so moons don't
  drag low orbits around a planet that never feels them.
- Everything orbits and spins clockwise, so prograde is to the right on
  the pad (ADR 0004).
- Missions name bodies by role (home, moon, neighbor, giant) and size the
  rocket tanks from the delta-v that system needs, so any seed works.

## Data flow per tick (60 Hz)
`main` reads input -> `App::update` -> `Play::update` (controls, mouse,
`Flight::update` = autopilot, steer, warp limits, physics steps, contact,
debris, prediction) -> `Tracker::update` (goals) -> particles, camera.
Then `App::draw` renders into the `Frame`, which `main` scales up.
