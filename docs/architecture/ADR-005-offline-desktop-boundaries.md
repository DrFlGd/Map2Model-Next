# ADR-005 — G1 Offline desktop trust boundaries

**Status:** Proposed implementation on A1-03; acceptance pending Windows runtime smoke checks and Agent 2 WGS84 review.

The desktop remains a clean-room replacement rather than embedding Mini Skyline's mirrored frontend. A blank coordinate-only MapLibre style is bundled inside the installed application and **does not need hosted tile servers** to start, pan, zoom or draw a rectangle. The interface clearly labels the absence of an actual terrain basemap; a procedural graticule is not a land-cover source.

## Trust separation
- Renderer is sandboxed and context isolated, with Node APIs unavailable and a restrictive Content Security Policy.
- Preload exposes only seven named methods; user-defined operation names or arbitrary filesystem paths are not forwarded to `ipcRenderer`.
- Electron main mediates an operating-system folder chooser, local library persistence and a read-only engine availability report. The renderer receives a folder's display basename rather than unrestricted native paths.
- Main session cancels external HTTP(S) requests. During development only `http://127.0.0.1:5173` may serve the Vite renderer. This exception is never enabled in packaged builds. Remote navigation and new-window creation are rejected.
- Project IDs are generated as `proj-<32 hex>` and validated before any filesystem operation. Files are read from the chosen library's `projects/` directory, with maximum size and symlink rejection. New data is staged with a random, exclusive-created temporary file, fsynced, then renamed. Project contents validate against the frozen `SelectionV1` contract.

## Lifecycle
First run: open Windows application → choose library → create a project → jump to WGS84 coordinates → zoom to selected site → draw a rectangle ≤2km per side, within ±60° latitude → save project → quit → reopen without network → open saved project. A future Agent 2 tile reader supplies real coverage and geodata; a future Agent 3 mesh/3MF module supplies export.

### Known limitations
- Geographic view has no offline basemap until the licensed data-library importer arrives. Place search and online tile browsing do not exist yet.
- `Generate`, `Import terrain` and `Export 3MF` controls remain disabled.
- Windows packaging and truly network-disconnected GUI startup are not proven by code or Node unit tests alone. Require Windows CI packaging and a human-visible offline launch smoke test.
- This A1-03 implementation is **not G3 offline model-generation acceptance**.
