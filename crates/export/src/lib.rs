//! A3-02: real deterministic 3MF Core writer with fail-closed preflight,
//! independent structural/readback validation and atomic native publication.
//! Frozen G0 contracts stay unchanged; slicer acceptance is an external gate.

pub mod package;

use map2model_contracts::canonical::{canonical_mesh_digest, MeshTriangle};
use map2model_contracts::validation::{validate_build, validate_manifest};
use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1, TerrainSourceManifestV1};
use map2model_mesh::{inspect_mesh, BuildOutput, MeshError};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const MODEL_PART: &str = "/3D/3dmodel.model";
pub const PROVENANCE_PART: &str = "/Metadata/map2model.provenance.json";
pub const CORE_VERSION: &str = "1.4.0";
pub const MAX_PROVENANCE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportError {
    pub code: &'static str,
    pub message: String,
}
impl ExportError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    /// Frozen JobStateV1 reasonCode for native A1-04 supervision. Internal
    /// ZIP/OPC errors use INVALID_EXPORT, which is NOT a V1 wire reason.
    /// Never send self.code directly to the frozen JobStateV1 boundary.
    pub fn job_reason_code(&self) -> &'static str {
        match self.code {
            "CANCELLED" => "CANCELLED",
            "DATA_MISSING" => "DATA_MISSING",
            "INVALID_REQUEST" => "INVALID_REQUEST",
            "INVALID_SELECTION" => "INVALID_SELECTION",
            "INVALID_CRS" => "INVALID_CRS",
            "UNSUPPORTED_DATUM" => "UNSUPPORTED_DATUM",
            "HASH_MISMATCH" => "HASH_MISMATCH",
            "INVALID_SOURCE" => "INVALID_SOURCE",
            "OUT_OF_MEMORY" => "OUT_OF_MEMORY",
            "NON_MANIFOLD_MESH" => "NON_MANIFOLD_MESH",
            "INTERNAL_ERROR" => "INTERNAL_ERROR",
            "INCOMPLETE_GRID" => "INCOMPLETE_GRID",
            "INVALID_GRID_HASH" => "INVALID_GRID_HASH",
            "INVALID_GEOMETRY" => "INVALID_GEOMETRY",
            "INVALID_DIMENSIONS" => "INVALID_DIMENSIONS",
            "NONFINITE_ELEVATION" => "NONFINITE_ELEVATION",
            // INVALID_EXPORT / unrecognized internal errors fail closed to
            // the already frozen public reason; no G0 contract change.
            _ => "EXPORT_FAILED",
        }
    }

    fn contract(message: String) -> Self {
        let code = match message.split(':').next().unwrap_or("") {
            "DATA_MISSING" => "DATA_MISSING",
            "HASH_MISMATCH" => "HASH_MISMATCH",
            "INVALID_DIMENSIONS" => "INVALID_DIMENSIONS",
            "INVALID_GRID_HASH" => "INVALID_GRID_HASH",
            "UNSUPPORTED_DATUM" => "UNSUPPORTED_DATUM",
            "INVALID_SOURCE" => "INVALID_SOURCE",
            "OUT_OF_MEMORY" => "OUT_OF_MEMORY",
            _ => "INVALID_REQUEST",
        };
        Self::new(code, message)
    }
}
impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}
impl Error for ExportError {}
impl From<MeshError> for ExportError {
    fn from(error: MeshError) -> Self {
        Self::new(error.code, error.message)
    }
}

/// Output from preflight is NOT an exported artifact or a success state.
/// Passing this structure to a later writer must still require independent
/// 3MF Core read-back, embedded-provenance match, and atomic publication.
#[derive(Debug, Clone)]
pub struct PreparedExport {
    pub provenance_bytes: Vec<u8>,
    pub provenance_sha256: String,
    pub canonical_geometry_sha256: String,
    pub model_part: &'static str,
    pub provenance_part: &'static str,
    pub core_spec_version: &'static str,
}

/// Verify the actual terrain mesh and its source/license identity before
/// any disk writes. No user-provided path, ZIP, writer or IPC token is accepted.
pub fn prepare_export(
    result: &BuildOutput,
    grid: &ElevationGridV1,
    request: &TerrainBuildRequestV1,
    source: &TerrainSourceManifestV1,
) -> Result<PreparedExport, ExportError> {
    validate_build(request, grid).map_err(ExportError::contract)?;
    validate_manifest(source).map_err(ExportError::contract)?;
    if source.manifest_sha256 != grid.source_manifest_sha256 {
        return Err(ExportError::new(
            "HASH_MISMATCH",
            "grid and source manifest differ",
        ));
    }
    if source.vertical_datum_id != grid.vertical_datum_id {
        return Err(ExportError::new(
            "UNSUPPORTED_DATUM",
            "source and grid vertical datum differ",
        ));
    }
    if source.horizontal_crs != "EPSG:4326" {
        return Err(ExportError::new(
            "INVALID_CRS",
            "source CRS not supported by G1 exporter",
        ));
    }
    let verified = inspect_mesh(&result.mesh)?;
    if result.geometry.vertices != verified.vertices
        || result.geometry.triangles != verified.triangles
        || result.geometry.units != "millimeter"
        || !result.geometry.closed
        || !result.geometry.oriented
        || result.geometry.non_manifold_edges != 0
        || result.geometry.degenerate_triangles != 0
        || result.geometry.component_count != 1
        || result.geometry.part_count != 1
        || result.geometry.self_intersection_status != "passed"
        || result.geometry.validation.status != "passed"
    {
        return Err(ExportError::new(
            "INVALID_GEOMETRY",
            "mesh QA envelope is inconsistent",
        ));
    }
    if result
        .geometry
        .measured_aabb_mm
        .iter()
        .zip(verified.measured_aabb_mm.iter())
        .any(|(a, b)| !a.is_finite() || (a - b).abs() > 1e-6)
        || !result.geometry.volume_mm3.is_finite()
        || (result.geometry.volume_mm3 - verified.volume_mm3).abs() > 1e-6
        || (result.geometry.minimum_measured_thickness_mm - verified.minimum_measured_thickness_mm)
            .abs()
            > 1e-6
    {
        return Err(ExportError::new(
            "INVALID_GEOMETRY",
            "measured QA data does not match mesh",
        ));
    }
    if (verified.measured_aabb_mm[3] - request.model_width_mm).abs() > 0.05
        || (verified.measured_aabb_mm[4] - request.model_height_mm).abs() > 0.05
        || verified.measured_aabb_mm[0].abs() > 0.05
        || verified.measured_aabb_mm[1].abs() > 0.05
        || verified.measured_aabb_mm[2].abs() > 0.05
        || verified.minimum_measured_thickness_mm + 0.05
            < request
                .base_thickness_mm
                .max(request.minimum_feature_thickness_mm)
    {
        return Err(ExportError::new(
            "INVALID_DIMENSIONS",
            "terrain size or minimum thickness differs from build request",
        ));
    }
    let triangles = result
        .mesh
        .triangles
        .iter()
        .map(|triangle| MeshTriangle {
            indices: *triangle,
            part_id: "terrain".to_owned(),
        })
        .collect::<Vec<_>>();
    let digest = canonical_mesh_digest(&result.mesh.vertices_mm, &triangles)
        .map_err(|message| ExportError::new("INVALID_GEOMETRY", message))?;
    if digest != result.canonical_geometry_sha256 {
        return Err(ExportError::new(
            "HASH_MISMATCH",
            "canonical geometry digest differs from mesh bytes",
        ));
    }

    // serde_json::Map is a BTreeMap without preserve_order. This produces
    // deterministic UTF-8 in the current frozen Rust/serde_json toolchain.
    // The embedded digest intentionally excludes the digest of itself.
    let provenance = json!({
        "formatVersion": "M2M-PROVENANCE-1",
        "coreSpecVersion": CORE_VERSION,
        "modelPart": MODEL_PART,
        "jobId": request.job_id,
        "correlationId": request.correlation_id,
        "selectionId": request.selection_id,
        "gridSha256": grid.grid_sha256,
        "sourceManifestSha256": source.manifest_sha256,
        "sourceArchiveSha256": source.source_archive_sha256,
        "sourceName": source.source_name,
        "sourceReleaseId": source.release_id,
        "sourceArchiveId": source.archive_id,
        "sourceLicenseIdentifier": source.license_identifier,
        "sourceAttribution": source.attribution,
        "sourceDecoderVersion": source.decoder_version,
        "sourceVerticalDatumId": source.vertical_datum_id,
        "sourceHorizontalCrs": source.horizontal_crs,
        "coverageStatus": source.coverage_status,
        "coverageIndexSha256": source.coverage_index_sha256,
        "nominalGroundSampleMeters": source.nominal_ground_sample_meters,
        "uncertaintyMeters": source.uncertainty_meters,
        "engineVersion": request.engine_version,
        "exportToken": request.export_token,
        "clipRectMeters": request.clip_rect_meters,
        "modelWidthMm": request.model_width_mm,
        "modelHeightMm": request.model_height_mm,
        "baseThicknessMm": request.base_thickness_mm,
        "minimumFeatureThicknessMm": request.minimum_feature_thickness_mm,
        "verticalExaggeration": request.vertical_exaggeration,
        "elevationZeroMeters": request.elevation_zero_meters,
        "canonicalGeometrySha256": digest,
        "meshVertexCount": verified.vertices,
        "meshTriangleCount": verified.triangles,
        "measuredAabbMm": verified.measured_aabb_mm,
        "meshVolumeMm3": verified.volume_mm3,
        "meshThicknessMm": verified.minimum_measured_thickness_mm,
        "meshSelfIntersectionStatus": verified.self_intersection_status
    });
    let bytes = serde_json::to_vec(&provenance).map_err(|e| {
        ExportError::new("INTERNAL_ERROR", format!("provenance serialization: {e}"))
    })?;
    if bytes.len() > MAX_PROVENANCE_BYTES {
        return Err(ExportError::new(
            "OUT_OF_MEMORY",
            "provenance exceeds 64KiB",
        ));
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(PreparedExport {
        provenance_bytes: bytes,
        provenance_sha256: sha256,
        canonical_geometry_sha256: digest,
        model_part: MODEL_PART,
        provenance_part: PROVENANCE_PART,
        core_spec_version: CORE_VERSION,
    })
}
