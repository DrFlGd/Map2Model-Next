# A3-02 Windows reference-slicer acceptance — evidence procedure

**Status (2026-10-11):** Product owner confirmed the *actual v0.0.2 Windows desktop GUI-produced synthetic .3mf* **imports and slices without errors in BOTH PrusaSlicer and Bambu Studio**. This is **PASS for basic prototype dual-slicer smoke** (see Lead record on PR #15 comment #6110895112). Do not ask the owner to repeat the identical basic smoke. However, the separately published Agent 3 standalone Rust CI sample has **not** been independently correlated by on-machine SHA to that preview file; exact slicer versions/builds, XYZ tolerance measurements, auto-repair state, screenshots/layers, and physical network status of that particular GUI test remain undocumented. The more precise procedure below is optional *additional release conformance evidence*, not a prerequisite to accepting already completed basic preview smoke. Whole-package Consortium conformance and production-source authorization remain open.

## Signed Rust output to test

Use the **Windows-produced actual Rust ZIP** from [GitHub Actions 38098914848](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38098914848), artifact named `a3-02-rust-3mf-windows-latest`. No derived sample or third-party terrain data. The artifact contains the `.3mf`, Python reports and the CLI's printed SHA evidence.

Known SHA-256 values (synthetic G0 frozen 4×3 terrain; both OSes generated the same ZIP):

| Hash | Exact SHA-256 |
|---|---|
| Real `synthetic-terrain.3mf` ZIP | `8463789ff97816d3dccd8b8085444c623150fb68417f3da04eafe9364afce675` |
| Embedded provenance JSON | `6f4536749caa1a2b1cccda07b62b339eb70bd04d77c770ff5e0d4f1d8de66d07` |
| Frozen V1 canonical mesh | `dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9` |

Reference: AABB `[0,0,0,180,80,204]` mm, 24 vertices / 44 triangles, volume 1,449,600 mm³ and minimum vertical thickness 4 mm. Synthetic source attribution and `CC0-1.0` license embedded. The file is an untransformed single closed terrain relief; no repair should be required.

## Physical Windows acceptance protocol

1. Download the **Windows artifact** from GitHub, extract `synthetic-terrain.3mf`, verify SHA-256 independently in PowerShell:

   `(Get-FileHash -Algorithm SHA256 .\synthetic-terrain.3mf).Hash.ToLowerInvariant()`

   Abort if it differs from the ZIP SHA above. Record build run URL, artifact name, machine OS and **exact Windows version**.
2. Disconnect all network connections if testing offline acceptance. Start PrusaSlicer; record the **exact executable/product version and build** from Help → About and a screenshot. Import **the unmodified `.3mf`** via File → Import. Record whether the file loads, if any repair dialog/warning was shown, and whether a single object appears in the correct positive-Z orientation (terrain up, base down). Record measured object X, Y, Z; each must differ by at most **0.05 mm** from 180, 80, 204. Do not silently fix, scale, rotate or translate except ordinary automatic bed placement; record any auto-placement separately.
3. Select a printer/bed large enough for 180×80×204 mm, slice; inspect bottom/sides and visible layer contours. Record success/failure, slicer warnings, and screenshot of 3D preview *and* sliced layers. If the printer is too small, document the printer constraint separately rather than mislabeling a valid mesh as structurally invalid.
4. Repeat steps 2–3 for Bambu Studio with its **exact installed version/build**, documenting repair, dimensions, orientation, slicing and screenshots. A file opening successfully without actually slicing is **not** enough to claim slicer acceptance.
5. Attach the two logs/screenshots and completed [slicer evidence template](tests/slicer-acceptance-template.json) to issue #6 / PR #15. Mark separately: import, warnings, repair, dimension tolerance, usable slice, offline status. **Only the product owner/lead may declare G3 slicer acceptance.**

## Automated independent evidence completed

- Cross-platform actual Rust ZIP generation and archive SHA.
- Python standard-library ZIP/ElementTree reader independently verified OPC StartPart/MustPreserve, bound geometry, exact canonical mesh digest, watertight/winding, license and provenance linkage.
- Third-party `trimesh 4.11.1` imported the **Rust-generated file**, one watertight, outward-oriented solid with correct dimensions/volume.
- Published 3MF Consortium **Core v1.4.0** model XSD from pinned spec commit `997b385`, checked using lxml 6.1.1 after explicitly recorded, **test-only** normalization of 16 `maxOccurs="2147483647"` to `unbounded` because libxml2 cannot accept the oversized bound; exact source/XSD hashes are in the workflow report.

These are individual evidence levels, not a blanket Consortium conformance certificate or proof of Windows slicer success. Source coverage/import validation, native job IPC success/cancellation transitions, and production-data rights/datum remain integration/release gates.

## Coordination

This PR introduces a tightly scoped `.github/workflows/a3-02-3mf-evidence.yml` file to attach **synthetic, redistribution-safe** samples and reports. CI is formally **Agent 1's ownership**: require lead review of this workflow before merge. Do not independently merge PR #15 or issue a successful `MeshAnd3mfResultV1` until native authorized publication, contract result validation and the agreed acceptance criteria are satisfied.
