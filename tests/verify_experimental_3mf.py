#!/usr/bin/env python3
"""Separate independent structural validation of an ORIGINAL synthetic-source G2 .3mf."""
import hashlib, json, pathlib, sys, zipfile
import xml.etree.ElementTree as ET

assert len(sys.argv)==3, "usage: verify_experimental.py file.3mf native-result.json"
file=pathlib.Path(sys.argv[1])
resp=json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8-sig"))
assert resp["status"]=="ok",resp
r=resp["result"]
assert r["experimentalOnly"] and not r["productionGenerationReady"]
assert r["verticalDatumStatus"]=="unknown" and not r["redistributionApproved"]
assert r["threeMfExported"] and r["networkRequests"]==0
assert r["sourceArchiveSha256"]=="489ec694f29c3d4c7bc5ed3f2dced736736e5c2a5d09cc1cef1ff3a0a8ede651"
assert len(r["candidateHeightsSha256"])==64
data=file.read_bytes()
assert len(data)==r["bytes"]
assert hashlib.sha256(data).hexdigest()==r["packageSha256"]
with zipfile.ZipFile(file) as z:
    assert z.testzip() is None
    assert set(z.namelist())=={"[Content_Types].xml","_rels/.rels","3D/3dmodel.model","Metadata/map2model.provenance.json"}
    pb=z.read("Metadata/map2model.provenance.json")
    assert hashlib.sha256(pb).hexdigest()==r["provenanceSha256"]
    p=json.loads(pb)
    for k,v in {"experimentalOnly":True,"localPersonalUseOnly":True,
            "ownerAcknowledgedUnverified":True,"sourceLicenseVerified":False,
            "nativeResolutionVerified":False,"encodingAuthenticated":False,
            "productionGenerationReady":False,"redistributionApproved":False,
            "verticalDatumStatus":"unknown","sourceLicenseStatus":"unknown"}.items():
        assert p[k] is v if isinstance(v,bool) else p[k]==v, (k,p[k])
    assert p["sourceArchiveSha256"]==r["sourceArchiveSha256"]
    assert p["candidateHeightsSha256"]==r["candidateHeightsSha256"]
    assert p["acknowledgementPolicyVersion"]=="PRIVATE-LOCAL-TERRAIN-2026-10-11"
    assert p["canonicalGeometrySha256"]==r["canonicalGeometrySha256"]
    assert "EXPERIMENTAL" in p["warning"]
    assert "sourcePath" not in p and "filePath" not in p
    model=ET.fromstring(z.read("3D/3dmodel.model"))
    ns={"m":"http://schemas.microsoft.com/3dmanufacturing/core/2015/02"}
    assert model.attrib["unit"]=="millimeter"
    metadata={m.attrib["name"]:m.text for m in model.findall("m:metadata",ns)}
    assert "EXPERIMENTAL" in metadata["m2m:SourceStatus"]
    assert "not independently verified" in metadata["m2m:SourceWarning"]
    assert metadata["m2m:VerticalDatumStatus"]=="unknown"
    assert metadata["m2m:SourceArchiveSHA256"]==r["sourceArchiveSha256"]
    vertices=model.findall(".//m:mesh/m:vertices/m:vertex",ns)
    triangles=model.findall(".//m:mesh/m:triangles/m:triangle",ns)
    assert len(vertices)==r["vertices"] and len(triangles)==r["triangles"]
    assert r["vertices"]>24 and r["triangles"]>44
print("PASS: actual original-PMTiles experimental ZIP, OPC/Core, source/candidate/mesh/provenance/file SHA and unverified warnings")
