# 0002. Browser build on Cloudflare Workers static assets

## Status
accepted

## Context
The game should be playable at centrifugal.helico.app, like RETRO SPORTS
(its ADR 0002).

## Decision
Same setup as RETRO SPORTS: build for `wasm32-unknown-unknown`;
`web/build.sh` copies `web/index.html`, the wasm and the macroquad
loader from the locked macroquad version into `dist/`; `wrangler.toml`
serves `dist/` as static assets on the custom domain. Deploy by hand with
`npx wrangler deploy`. The canvas blocks the right-click menu (right
click deletes nodes and pans).

## Consequences
- One code base for desktop and web.
- Keyboard and mouse only; no touch yet (issue 0003).
- Sound starts after the first key press or click (browser rule).
