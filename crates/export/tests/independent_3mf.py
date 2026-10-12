#!/usr/bin/env python3
"""Independent stdlib-only structural 3MF verification for *Rust-generated* bytes.

Usage: python independent_3mf.py OUTPUT.3mf A3-01.reference.json
This is a separate Python ZIP/XML/mesh implementation. It is deliberately
NOT an official 3MF Consortium schema/validator or slicer conformance claim.
"""
import hashlib
import json
import math
import pathlib
import struct
import sys
import xml.etree.ElementTree as ET
import zipfile
from collections import defaultdict, deque

PARTS = {
    "[Content_Types].xml",
    "_rels/.rels",
    "3D/3dmodel.model",
    "Metadata/map2model.provenance.json",
}
CORE = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02"
OPC_TYPES = "http://schemas.openxmlformats.org/package/2006/content-types"
OPC_RELS = "http://schemas.openxmlformats.org/package/2006/relationships"
MODEL_REL = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"
PRESERVE_REL = "http://schemas.openxmlformats.org/package/2006/relationships/mustpreserve"
MODEL_TYPE = "application/vnd.ms-package.3dmanufacturing-3dmodel+xml"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def quantize(x):
    require(math.isfinite(x), "nonfinite vertex")
    q = math.floor(abs(x) * 1_000_000 + 0.5)
    require(q <= 2**53, "vertex quantization overflow")
    return -q if x < 0 else q


def mesh_digest(points, faces):
    """Independent frozen M2M-MESH-1 binary encoder, not the Rust implementation."""
    sorted_vertices = sorted((tuple(quantize(v) for v in p), i) for i, p in enumerate(points))
    require(len(set(q for q, _ in sorted_vertices)) == len(points), "duplicate quantized point")
    remap = {original: i for i, (_, original) in enumerate(sorted_vertices)}
    triangles = []
    for tri in faces:
        a, b, c = (remap[i] for i in tri)
        require(len({a, b, c}) == 3, "degenerate quantized triangle")
        oriented = min((a, b, c), (b, c, a), (c, a, b))
        triangles.append((b"terrain", oriented))
    triangles.sort()
    out = bytearray(b"M2M-MESH-1\x00")
    out.extend(struct.pack("<I", len(sorted_vertices)))
    for point, _ in sorted_vertices:
        for n in point:
            out.extend(struct.pack("<q", n))
    out.extend(struct.pack("<I", len(triangles)))
    for part, face in triangles:
        out.extend(struct.pack("<I", len(part)))
        out.extend(part)
        for index in face:
            out.extend(struct.pack("<I", index))
    return sha(out)


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


def inspect_mesh(points, faces):
    require(3 <= len(points) <= 200_000, "unbounded/missing mesh vertices")
    require(1 <= len(faces) <= 400_000, "unbounded/missing triangles")
    incidence = defaultdict(list)
    connected = defaultdict(set)
    six_volumes = []
    for i, (a, b, c) in enumerate(faces):
        require(all(0 <= v < len(points) for v in (a, b, c)), "triangle index out of bounds")
        require(len({a, b, c}) == 3, "repeated triangle vertex")
        pa, pb, pc = (points[v] for v in (a, b, c))
        ab = tuple(pb[d] - pa[d] for d in range(3))
        ac = tuple(pc[d] - pa[d] for d in range(3))
        normal = cross(ab, ac)
        require(dot(normal, normal) > 1e-24, f"degenerate triangle {i}")
        six_volumes.append(dot(pa, cross(pb, pc)))
        for u, v in ((a, b), (b, c), (c, a)):
            incidence[tuple(sorted((u, v)))].append((u, v))
            connected[u].add(v)
            connected[v].add(u)
    for edge, oriented in incidence.items():
        require(len(oriented) == 2 and oriented[0] == tuple(reversed(oriented[1])),
                f"non-manifold or wrongly oriented edge {edge}")
    seen = set()
    components = 0
    for vertex in range(len(points)):
        if vertex not in seen:
            components += 1
            queue = deque([vertex])
            seen.add(vertex)
            while queue:
                for nxt in connected[queue.popleft()]:
                    if nxt not in seen:
                        seen.add(nxt)
                        queue.append(nxt)
    volume = math.fsum(six_volumes) / 6.0
    require(volume > 0 and math.isfinite(volume), "not a positive-volume oriented solid")
    bounds = [min(p[i] for p in points) for i in range(3)] + [
        max(p[i] for p in points) for i in range(3)
    ]
    return bounds, volume, components, len(incidence)


def validate(path, reference):
    raw = pathlib.Path(path).read_bytes()
    require(len(raw) < 80 * 1024 * 1024, "archive oversize")
    require(raw.startswith(b"PK\x03\x04"), "not a ZIP local-file header")
    with zipfile.ZipFile(path, "r", allowZip64=False) as archive:
        names = archive.namelist()
        require(len(names) == 4 and set(names) == PARTS, "missing/duplicate/unexpected ZIP parts")
        require(all(z.file_size < 64 * 1024 * 1024 for z in archive.infolist()), "ZIP part oversize")
        require(archive.testzip() is None, "ZIP CRC failure")
        ctypes = ET.fromstring(archive.read("[Content_Types].xml"))
        rels = ET.fromstring(archive.read("_rels/.rels"))
        model = ET.fromstring(archive.read("3D/3dmodel.model"))
        provenance_bytes = archive.read("Metadata/map2model.provenance.json")
    require(ctypes.tag == f"{{{OPC_TYPES}}}Types", "invalid OPC content types root")
    mime_overrides = {
        o.get("PartName"): o.get("ContentType")
        for o in ctypes.findall(f"{{{OPC_TYPES}}}Override")
    }
    require(mime_overrides.get("/3D/3dmodel.model") == MODEL_TYPE, "3MF model MIME absent")
    require(mime_overrides.get("/Metadata/map2model.provenance.json") == "application/json",
            "provenance MIME absent")
    require(rels.tag == f"{{{OPC_RELS}}}Relationships", "bad OPC relationship root")
    relations = [(x.get("Target"), x.get("Type")) for x in rels]
    require(relations.count(("/3D/3dmodel.model", MODEL_REL)) == 1, "bad model StartPart")
    require(relations.count(("/Metadata/map2model.provenance.json", PRESERVE_REL)) == 1,
            "bad provenance MustPreserve")
    require(len(relations) == 2, "unexpected OPC relationships")
    require(model.tag == f"{{{CORE}}}model" and model.get("unit") == "millimeter",
            "3MF unit is not millimeter")
    resources = model.find(f"{{{CORE}}}resources")
    build = model.find(f"{{{CORE}}}build")
    require(resources is not None and build is not None, "3MF lacks resources or build")
    objects = resources.findall(f"{{{CORE}}}object")
    require(len(objects) == 1 and objects[0].get("id") == "1" and
            objects[0].get("type") == "model", "unexpected model object")
    items = build.findall(f"{{{CORE}}}item")
    require(len(items) == 1 and items[0].get("objectid") == "1" and
            "transform" not in items[0].attrib, "unexpected build item/transform")
    mesh = objects[0].find(f"{{{CORE}}}mesh")
    require(mesh is not None, "missing Core mesh")
    vr = mesh.find(f"{{{CORE}}}vertices")
    tr = mesh.find(f"{{{CORE}}}triangles")
    require(vr is not None and tr is not None, "missing Core mesh points/faces")
    points = []
    for node in vr:
        require(node.tag == f"{{{CORE}}}vertex", "unknown vertex tag")
        point = tuple(float(node.attrib[name]) for name in ("x", "y", "z"))
        require(all(math.isfinite(x) for x in point), "nonfinite 3MF coordinate")
        points.append(point)
    faces = []
    for node in tr:
        require(node.tag == f"{{{CORE}}}triangle", "unknown face tag")
        faces.append(tuple(int(node.attrib[name]) for name in ("v1", "v2", "v3")))
    bounds, volume, components, edges = inspect_mesh(points, faces)
    geometry = mesh_digest(points, faces)
    provenance = json.loads(provenance_bytes)
    metadata = {m.get("name"): m.text for m in model.findall(f"{{{CORE}}}metadata")}
    require(geometry == provenance["canonicalGeometrySha256"], "actual geometry != provenance hash")
    require(provenance["sourceLicenseIdentifier"] and provenance["sourceAttribution"],
            "source license or attribution missing")
    require(metadata.get("m2m:CanonicalGeometrySHA256") == geometry, "XML geometry metadata wrong")
    require(metadata.get("m2m:ProvenanceSHA256") == sha(provenance_bytes), "XML provenance SHA wrong")
    require(metadata.get("m2m:SourceLicense") == provenance["sourceLicenseIdentifier"],
            "XML source license mismatch")
    require(metadata.get("m2m:SourceAttribution") == provenance["sourceAttribution"],
            "XML attribution mismatch")
    require(geometry == reference["canonicalGeometrySha256"], "frozen SHA mismatch")
    require(len(points) == reference["vertices"], "frozen vertex count mismatch")
    require(len(faces) == reference["triangles"], "frozen triangle count mismatch")
    require(all(abs(a-b) <= 1e-6 for a, b in zip(bounds, reference["measuredAabbMm"])),
            "frozen XYZ bounds mismatch")
    require(abs(volume-reference["volumeMm3"]) < 1e-6, "frozen solid volume mismatch")
    require(components == 1, "multiple mesh components")
    return {
        "validatorId": "python-stdlib-zipfile-etree-mesh-v1",
        "validatorScope": "independent OPC/geometry/provenance, not official Core 1.4 conformance",
        "status": "passed",
        "packageSha256": sha(raw),
        "provenanceSha256": sha(provenance_bytes),
        "canonicalGeometrySha256": geometry,
        "aabbMm": bounds,
        "vertices": len(points),
        "triangles": len(faces),
        "volumeMm3": volume,
        "connectedComponents": components,
        "undirectedManifoldEdges": edges,
        "sourceLicenseIdentifier": provenance["sourceLicenseIdentifier"],
        "sourceAttribution": provenance["sourceAttribution"],
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("Usage: independent_3mf.py FILE.3mf A3-01.reference.json")
    fixture = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
    print(json.dumps(validate(sys.argv[1], fixture), sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, IndexError, OverflowError, ET.ParseError,
            zipfile.BadZipFile, json.JSONDecodeError) as error:
        print(f"independent 3MF validation FAILED: {error}", file=sys.stderr)
        sys.exit(1)
