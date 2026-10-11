# Official Core 1.4 schema test fixtures — pinned and non-runtime

- `3mf-core-1.4.xsd`: **verbatim XML inside Appendix B.1.1**, [3MF Consortium Core specification v1.4.0](https://github.com/3MFConsortium/spec_core/blob/997b385e06f3181cf9aae0c578e0b45ccd48ccb2/3MF%20Core%20Specification.md), pinned commit `997b385e06f3181cf9aae0c578e0b45ccd48ccb2`; BSD-2-Clause per `LICENSE-3MF`. This file itself is unmodified and its SHA-256 is reported on each workflow run.
- `xml.xsd`: W3C XML namespace import snapshot distributed under the 3MF Consortium [test suite](https://github.com/3MFConsortium/test_suites/blob/f483d3beee069e7adbac125d1f7baa36b5b3e5bf/Consolidated_Schema/Schema/xml.xsd), pinned commit `f483d3beee069e7adbac125d1f7baa36b5b3e5bf`. Original XML namespace schema is a W3C publication; W3C use/redistribution terms should be checked before release. These files are **test-only**, not bundled in the application.
- `core_xsd_check.py` uses lxml/libxml2 offline, with only the pinned local W3C schema import. Current libxml2 rejects `maxOccurs="2147483647"` as an implementation limit even though that is in published XSD; the checker transparently normalizes only those bounds to `maxOccurs="unbounded"` **in memory**, with exact replacement count and normalized SHA in its report. For G1 meshes with at most 400k faces, this cannot affect the validity decision on generated outputs. It must not be described as validating against an entirely unmodified in-memory schema; source file itself remains preserved.
- Independent ZIP/OPC/MustPreserve, mesh topology/hash and third-party trimesh import checks run **separately**. Passing this XSD test is evidence for the Core 1.4 XML model schema only, not proof of full OPC or slicer conformance.

## Third-party implementation dependency inventory

Runtime writer: pinned `zip=2.4.2` (MIT), `roxmltree=0.20.0` (MIT/Apache-2.0); `serde_json=1.0.145` and `sha2=0.10.9` (MIT/Apache-2.0). Their **complete resolved transitive package license metadata and Cargo.lock** are generated as workflow artifacts for lead review. This is *not* a redistribution NOTICE audit or finalized Windows SDK license approval.

CI-only reader/test dependencies: Python `trimesh==4.11.1` (MIT), `numpy==2.3.5` (BSD-3-Clause), `networkx==3.6.1` (BSD-3-Clause) and `lxml==6.1.1` (BSD-3-Clause). These are **not** runtime dependencies of the Windows application.

The signed synthetic source fixture is original, declared `CC0-1.0` in the frozen V1 fixture. No genuine third-party terrain pixels or proprietary assets are included.
