#!/usr/bin/env python3
"""Independent third-party trimesh importer acceptance for Rust-produced 3MF.

Not a slicer, not an official Core conformance validator.
Usage: python crates/export/tests/trimesh_import_3mf.py FILE.3mf A3-01.reference.json
"""
import hashlib
import json
import math
import pathlib
import sys
import trimesh


def require(ok, message):
    if not ok:
        raise ValueError(message)


def validate(path, fixture):
    raw = pathlib.Path(path).read_bytes()
    scene = trimesh.load(path, force="scene")
    require(isinstance(scene, trimesh.Scene), "import did not return a 3MF scene")
    meshes = scene.dump(concatenate=False)
    require(len(meshes) == 1 and isinstance(meshes[0], trimesh.Trimesh),
            "not exactly one printable imported mesh")
    mesh = meshes[0]
    bounds = mesh.bounds.tolist()
    require(mesh.is_watertight, "third-party importer reports non-watertight mesh")
    require(mesh.is_winding_consistent, "third-party importer reports reversed winding")
    require(mesh.volume > 0 and math.isfinite(mesh.volume), "invalid signed volume")
    require(len(mesh.vertices) == fixture["vertices"], "reference vertex count mismatch")
    require(len(mesh.faces) == fixture["triangles"], "reference triangle count mismatch")
    require(all(abs(bounds[j][k]-fixture["measuredAabbMm"][3*j+k]) < 1e-5
                for j in range(2) for k in range(3)), "reference dimensions mismatch")
    require(abs(mesh.volume - fixture["volumeMm3"]) < 1e-5, "reference volume mismatch")
    return {
        "validatorId": "trimesh-3mf-independent-reader",
        "validatorVersion": trimesh.__version__,
        "validatorScope": "third-party 3MF import/mesh diagnostics, NOT a slicer or official Core conformance",
        "status": "passed",
        "importedObjects": len(meshes),
        "vertices": len(mesh.vertices),
        "triangles": len(mesh.faces),
        "aabbMm": [*bounds[0], *bounds[1]],
        "watertight": bool(mesh.is_watertight),
        "consistentWinding": bool(mesh.is_winding_consistent),
        "volumeMm3": float(mesh.volume),
        "packageSha256": hashlib.sha256(raw).hexdigest()
    }


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("Usage: trimesh_import_3mf.py FILE.3mf A3-01.reference.json")
        fixture = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
        print(json.dumps(validate(sys.argv[1], fixture), sort_keys=True, separators=(",", ":")))
    except Exception as error:
        print(f"independent trimesh 3MF validation FAILED: {error}", file=sys.stderr)
        sys.exit(1)
