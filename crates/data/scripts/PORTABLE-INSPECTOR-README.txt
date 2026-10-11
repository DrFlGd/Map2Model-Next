MAP2MODEL NEXT - PRIVATE Mapterhorn FORMAT inspector for Windows
=================================================================
Build: CI-compiled Rust binary, Agent 2 draft PR #16
This is NOT the final Map2Model Next installer or Electron application.

A. HOW TO RUN WITHOUT CARGO / RUST
1. Download the GitHub Actions artifact "map2model-agent2-windows-inspector"
   from the successful workflow run at:
   https://github.com/DrFlGd/Map2Model-Next/actions
   (Workflow: Agent 2 portable Windows PMTiles format inspector)
2. Extract the ZIP to a private folder; keep the 4 delivered files together:
     terrain_offline.exe
     Inspect-Interlaken.cmd
     Inspect-Interlaken.ps1
     BUILD-PROVENANCE.txt
     README.txt
3. Keep your privately acquired interlaken-z17.pmtiles OUTSIDE the ZIP.
   File must be 3,127,295 bytes with SHA-256:
   75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0
4. Drag the private interlaken-z17.pmtiles file onto Inspect-Interlaken.cmd,
   OR open Command Prompt and execute:
     Inspect-Interlaken.cmd "C:\path\to\interlaken-z17.pmtiles"
5. Copy the printed PASS lines or error message into the existing PR #16.
   NEVER upload your real PMTiles archive or decoded terrain to public GitHub.

B. WHAT THIS TESTS (AND WHAT IT DOES NOT)
- Exact source file bytes and SHA-256; PMTiles directory gzip, real index entries,
  40 indexed tiles at z13-17, real decompressed 512x512 WebP samples and
  five independently reference-checked candidate Terrarium RGB values.
- The native Rust binary does not perform HTTP downloads or DNS lookups.
  To certify physically offline use, disconnect Windows from networks first.
- The inspector must report productionGenerationReady=false,
  sourceLicenseVerified=false, verticalDatumVerified=false,
  nativeResolutionVerified=false and no encoding metadata attestation.
- It DOES NOT create, validate or export a real-terrain 3MF. The archived
  sample's vertical datum, source-specific derivative-use rights, DEM
  resolution and upstream pinned release remain unverified.
- The full GUI/3MF integration belongs to Agent 1 / A1-04 and is a separate
  deliverable. The Windows executable is an unsigned developer preview;
  verify its BUILD-PROVENANCE.txt digest before running.

C. ADVANCED / OTHER SOURCE FILES
  terrain_offline.exe inspect "C:\path\to\private.pmtiles"
  terrain_offline.exe inspect "C:\path\to\private.pmtiles" 13 4275 2891

All format-only source results stay unverified. The production-shaped "decode"
subcommand enforces explicit archive SHA and a verified declaration; do not
invent a vertical datum, source license or native resolution merely to pass it.

D. BUILD / RELEASE EVIDENCE
Workflow: .github/workflows/a2-02-portable-windows.yml
The workflow runs the complete Rust workspace tests and compiles the real
native Windows Rust binary. It publishes no real source terrain bytes.
