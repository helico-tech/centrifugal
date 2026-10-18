# CENTRIFUGAL

A 2D rocket and orbital-mechanics game in the RETRO SPORTS look (pixel
art, 16-bit colors, HELICO SOFT 1992), but at modern screen sizes and
resizable. Launch from a procedural planet, plan burns on the predicted
path, fly to moons and planets, land like a booster. Rust + macroquad,
software framebuffer, also runs in the browser at
https://centrifugal.helico.app.

## Build and run
- `cargo run --release` — play (window, keyboard + mouse, sound).
- `cargo test --release` — physics tests (`tests/physics.rs`).
- `cargo run --release -- --shots <dir>` — scripted PNG screenshots of
  every screen (640x360 at 2x). Use these to review visuals.
- `cargo run --release -- --bots` — bot pilots fly missions and report
  outcome and fuel left (balance check, see `docs/specs/game.md`).
- `web/build.sh` — browser build into `dist/`; test with
  `python3 -m http.server -d dist`, deploy with `npx wrangler deploy`
  (ADR 0002).

## Controls
Arrows/WASD throttle and turn, Z full, X cut, Space stage/chute, 1-6 attitude
hold, `,` `.` warp, G warp to next event, click the path to add a node,
drag its handles, N auto-burn, T target, F focus, M map, R rewind, F1
help, F2 pixel size, Esc pause.

## Map
- [docs/architecture.md](docs/architecture.md) — parts, data flow, key rules.
- [docs/house-style.md](docs/house-style.md) — the look and the screens.
- [docs/specs/game.md](docs/specs/game.md) — design, missions, balance.
- [docs/research/ux-and-mechanics.md](docs/research/ux-and-mechanics.md) —
  what other orbital games do well and badly, with sources.
- [docs/adr/](docs/adr/) — decisions.
- [work/queue.md](work/queue.md) — what to pick up next.
