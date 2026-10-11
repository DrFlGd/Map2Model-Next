//! A1-04 local, synthetic-only contract-to-mesh integration smoke.
//! No renderer path, network, geodata entitlement, or 3MF file is involved.
use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1};
use map2model_mesh::build_mesh;
use serde_json::{json, Value};

pub fn run() -> Result<Value, String> {
    let grid: ElevationGridV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
    ))
    .map_err(|_| "INTERNAL_ERROR: frozen synthetic grid fixture cannot deserialize")?;
    let request: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
    ))
    .map_err(|_| "INTERNAL_ERROR: frozen synthetic build fixture cannot deserialize")?;
    let built = build_mesh(&grid, &request)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    Ok(json!({
        "fixture": "G0-synthetic-elevation-grid-v1",
        "synthetic": true,
        "meshVerified": built.geometry.closed &&
            built.geometry.oriented &&
            built.geometry.non_manifold_edges == 0 &&
            built.geometry.degenerate_triangles == 0,
        "vertices": built.geometry.vertices,
        "triangles": built.geometry.triangles,
        "aabbMm": built.geometry.measured_aabb_mm,
        "minimumThicknessMm": built.geometry.minimum_measured_thickness_mm,
        "canonicalGeometrySha256": built.canonical_geometry_sha256,
        "threeMfExported": false,
        "productionGenerationReady": false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_grid_reaches_actual_watertight_mesh() {
        let value = run().expect("frozen G0 fixture builds");
        assert_eq!(value["meshVerified"], true);
        assert_eq!(value["vertices"], 24);
        assert_eq!(value["triangles"], 44);
        assert_eq!(value["aabbMm"], json!([0., 0., 0., 180., 80., 204.]));
        assert_eq!(
            value["canonicalGeometrySha256"],
            "dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9"
        );
        assert_eq!(value["threeMfExported"], false);
        assert_eq!(value["productionGenerationReady"], false);
    }
}
