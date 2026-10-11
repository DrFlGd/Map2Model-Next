# Map2Model Next — Windows desktop shell (A1-03)

This is a **real independent desktop shell**, not a terrain model generator yet. Only project persistence and WGS84 rectangle selection work. The map is an empty locally bundled MapLibre style with coordinate lines; no hosted basemap is required. It does not show roads, terrain or buildings.

## Developer commands
- `npm install` (installs pinned dev tools)
- `npm run desktop:build` (bundled renderer suitable for offline app package)
- `npm run desktop:electron` (launch installed Electron + already built renderer)
- For hot reload: start `npm run desktop:serve` in one terminal and set `MAP2MODEL_DEV=1` when running `npm run desktop:electron-dev` in another. The only temporary network exception is local loopback.
- `npm run desktop:package` on Windows produces a portable Windows EXE in `release/`.
- `npm run ci` runs the contract tests, desktop project/selection tests and renderer build; `cargo test --workspace` remains the Rust contract/engine smoke suite.

## Acceptance scope
1. On a disconnected Windows machine, launch the app with no dependency on a remote map.
2. Choose a local library folder; create a project.
3. Zoom/jump to longitude and latitude; use **Draw terrain rectangle**. Selections must be no more than 2km per side and remain within ±60° latitude.
4. Save the project, exit, restart offline, reopen and observe exactly the same bounds and deterministic selection ID.
5. Confirm that importing data / printing a 3MF is explicitly unavailable; a blank map is not claimed to represent elevation data.

Issue [#7](https://github.com/DrFlGd/Map2Model-Next/issues/7) tracks the UI implementation and Windows acceptance evidence.
