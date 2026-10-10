//! Proposed cross-agent V1 serializable messages.
//! JSON Schemas in packages/contracts/schema/v1 are authoritative at G0.
//! These structs deserialize samples; higher-level semantic validation is required.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainProfileV1 {
    pub target_ground_sample_meters: f64,
    pub no_data_policy: String,
    pub max_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionV1 {
    pub schema_version: String,
    pub kind: String,
    pub selection_id: String,
    pub planet: String,
    pub horizontal_crs: String,
    pub bbox: [f64; 4],
    pub terrain_profile: TerrainProfileV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainSourceManifestV1 {
    pub schema_version: String,
    pub kind: String,
    pub source_name: String,
    pub release_id: String,
    pub archive_id: String,
    pub local_asset_token: String,
    pub sha256: String,
    pub coverage_bbox: [f64; 4],
    pub horizontal_crs: String,
    pub vertical_datum_id: String,
    pub native_ground_sample_meters: f64,
    pub source_bytes: u64,
    pub license_identifier: String,
    pub attribution: String,
    pub decoder_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElevationGridV1 {
    pub schema_version: String,
    pub kind: String,
    pub grid_id: String,
    pub selection_id: String,
    pub source_manifest_sha256: String,
    pub width: usize,
    pub height: usize,
    pub samples_meters: Vec<Option<f64>>,
    pub valid_mask: Vec<bool>,
    pub min_east_meters: f64,
    pub max_north_meters: f64,
    pub dx_meters: f64,
    pub dy_meters: f64,
    pub anchor_wgs84: [f64; 2],
    pub horizontal_transform_id: String,
    pub vertical_datum_id: String,
    pub storage_convention: String,
    pub grid_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainBuildRequestV1 {
    pub schema_version: String,
    pub kind: String,
    pub job_id: String,
    pub correlation_id: String,
    pub selection_id: String,
    pub grid_sha256: String,
    pub clip_rect_meters: [f64; 4],
    pub model_width_mm: f64,
    pub model_height_mm: f64,
    pub base_thickness_mm: f64,
    pub minimum_feature_thickness_mm: f64,
    pub vertical_exaggeration: f64,
    pub elevation_zero_meters: f64,
    pub engine_version: String,
    pub export_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryReportV1 {
    pub units: String,
    pub vertices: usize,
    pub triangles: usize,
    pub closed: bool,
    pub oriented: bool,
    pub non_manifold_edges: usize,
    pub degenerate_triangles: usize,
    pub minimum_measured_thickness_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportArtifactV1 {
    pub export_token: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub format: String,
    pub conformance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshAnd3mfResultV1 {
    pub schema_version: String,
    pub kind: String,
    pub job_id: String,
    pub geometry: GeometryReportV1,
    pub artifact: ExportArtifactV1,
    pub canonical_geometry_sha256: String,
    pub provenance_sha256: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobStateV1 {
    pub schema_version: String,
    pub kind: String,
    pub job_id: String,
    pub correlation_id: String,
    pub phase: String,
    pub status: String,
    pub progress01: f64,
    pub reason_code: Option<String>,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_v1_fixtures_deserialize_and_roundtrip() {
        let values = [
            include_str!("../../../tests/fixtures/contracts/v1/SelectionV1.valid.json"),
            include_str!("../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"),
            include_str!("../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"),
            include_str!("../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"),
            include_str!("../../../tests/fixtures/contracts/v1/MeshAnd3mfResultV1.valid.json"),
            include_str!("../../../tests/fixtures/contracts/v1/JobStateV1.valid.json"),
        ];
        for raw in values {
            let decoded: serde_json::Value = serde_json::from_str(raw).unwrap();
            assert_eq!(decoded["schemaVersion"], "1.0.0");
            assert!(decoded["kind"].is_string());
        }
        let s: SelectionV1 = serde_json::from_str(values[0]).unwrap();
        let m: TerrainSourceManifestV1 = serde_json::from_str(values[1]).unwrap();
        let g: ElevationGridV1 = serde_json::from_str(values[2]).unwrap();
        let b: TerrainBuildRequestV1 = serde_json::from_str(values[3]).unwrap();
        let e: MeshAnd3mfResultV1 = serde_json::from_str(values[4]).unwrap();
        let j: JobStateV1 = serde_json::from_str(values[5]).unwrap();
        assert_eq!(s.kind, "SelectionV1");
        assert_eq!(m.kind, "TerrainSourceManifestV1");
        assert_eq!(g.kind, "ElevationGridV1");
        assert_eq!(b.kind, "TerrainBuildRequestV1");
        assert_eq!(e.kind, "MeshAnd3mfResultV1");
        assert_eq!(j.kind, "JobStateV1");
        assert_eq!(g.samples_meters.len(), g.width * g.height);
        assert_eq!(g.valid_mask.len(), g.width * g.height);
        // JSON numeric spelling (10 versus 10.0) is not semantically significant.
        // Test actual typed serde roundtrips instead of comparing number formatting.
        let selection_roundtrip: SelectionV1 =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        let grid_roundtrip: ElevationGridV1 =
            serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(selection_roundtrip.selection_id, s.selection_id);
        assert_eq!(grid_roundtrip.samples_meters, g.samples_meters);
        assert_eq!(grid_roundtrip.grid_sha256, g.grid_sha256);
    }
}
