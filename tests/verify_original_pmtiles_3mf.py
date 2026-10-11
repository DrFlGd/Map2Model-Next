#!/usr/bin/env python3
"""Independent stdlib verification of original 512px PMTiles -> 3MF native output."""
import hashlib
import json
import pathlib
import sys
import xml.etree.ElementTree as ET
import zipfile

def check(path, report):
    info = json.loads(pathlib.Path(report).read_text(encoding="utf-8-sig"))
    assert info["status"] == "ok", info
    r = info["result"]
    assert r["synthetic"] and r["originalPmtilesDecoded"] and r["threeMfExported"]
    assert r["productionGenerationReady"] is False
    assert r["sourceArchiveSha256"] == "489ec694f29c3d4c7bc5ed3f2dced736736e5c2a5d09cc1cef1ff3a0a8ede651"
    assert r["networkRequests"] == 0
    assert r["gridWidth"] >= 4 and r["gridHeight"] >= 4
    assert r["vertices"] > 24 and r["triangles"] > 44
    data = pathlib.Path(path).read_bytes()
    assert len(data) == r["bytes"]
    assert hashlib.sha256(data).hexdigest() == r["packageSha256"]
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None
        names = set(archive.namelist())
        assert "3D/3dmodel.model" in names
        assert "Metadata/map2model.provenance.json" in names
        assert "[Content_Types].xml" in names
        provenance_bytes = archive.read("Metadata/map2model.provenance.json")
        provenance = json.loads(provenance_bytes)
        assert hashlib.sha256(provenance_bytes).hexdigest() == r["provenanceSha256"]
        assert provenance["sourceArchiveSha256"] == r["sourceArchiveSha256"]
        assert provenance["gridSha256"] == r["gridSha256"]
        assert "Mapterhorn" not in provenance.get("sourceName", "").replace("NOT Mapterhorn", "")
        root = ET.fromstring(archive.read("3D/3dmodel.model"))
        ns = {"m": "http://schemas.microsoft.com/3dmanufacturing/core/2015/02"}
        vertices = root.findall(".//m:mesh/m:vertices/m:vertex", ns)
        triangles = root.findall(".//m:mesh/m:triangles/m:triangle", ns)
        assert len(vertices) == r["vertices"], (len(vertices), r["vertices"])
        assert len(triangles) == r["triangles"], (len(triangles), r["triangles"])
        assert root.attrib["unit"] == "millimeter"
        bbox = r["aabbMm"]
        assert len(bbox) == 6 and abs(bbox[3] - 120.0) < 0.05
        assert bbox[5] > 4.0
    print(json.dumps({"result":"PASS","sourceSha":r["sourceArchiveSha256"],
        "gridSha":r["gridSha256"],"meshSha":r["canonicalGeometrySha256"],
        "zipSha":r["packageSha256"],"triangles":r["triangles"]},sort_keys=True))

if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("Usage: verify_original_pmtiles_3mf.py file.3mf native-report.json")
    check(sys.argv[1], sys.argv[2])
