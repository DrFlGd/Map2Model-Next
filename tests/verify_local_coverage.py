"""Independent stdlib checks for native SHA-pinned PMTiles index coverage reporting."""
import json, pathlib, sys
assert len(sys.argv)==5, "Usage: verify_local_coverage.py source.json inside.json partial.json outside.json"
source,inside,partial,outside=(json.loads(pathlib.Path(p).read_text(encoding="utf-8-sig")) for p in sys.argv[1:])
for r in (source,inside,partial,outside):
    assert r["status"]=="ok",r
    x=r["result"]
    assert x["kind"]=="LocalPMTilesCoveragePreviewV1"
    assert x["sourceArchiveSha256"]=="489ec694f29c3d4c7bc5ed3f2dced736736e5c2a5d09cc1cef1ff3a0a8ede651"
    assert x["sourceLicenseVerified"] is False
    assert x["productionGenerationReady"] is False
    assert x["verticalDatumStatus"]=="unknown"
    assert x["networkRequests"]==0
assert source["result"]["declaredCoverageBboxWgs84"]==[-84.11132,39.63954,-83.93555,39.77476]
geom=source["result"]["coverageGeoJson"]["geometry"]
assert geom["type"]=="Polygon"
ring=geom["coordinates"][0]
assert len(ring)==5 and ring[0]==ring[-1]
assert source["result"]["selection"] is None
assert source["result"]["indexedTileCount"]==4
s=inside["result"]["selection"]
assert s["status"]=="inside" and s["requestedZoom"]==12
assert s["indexedRequiredTiles"]==s["requiredClosedBoundaryTiles"]==4
assert s["missingRequiredTiles"]==0 and s["allRequiredTilesIndexed"] is True
assert "NOT verified" in s["classificationScope"]
s=partial["result"]["selection"]
assert s["status"]=="partial" and s["missingRequiredTiles"]>0
assert s["indexedRequiredTiles"]>0
s=outside["result"]["selection"]
assert s["status"]=="outside" and s["indexedRequiredTiles"]==0
for r in (source,inside,partial,outside):
    assert "candidateTerrariumHeightsMeters" not in r["result"]
print("PASS: native PMTiles declared envelope + actual indexed inside/partial/outside, source SHA, zoom, unknown rights/datum")
