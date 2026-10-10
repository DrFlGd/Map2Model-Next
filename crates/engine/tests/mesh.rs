use map2model_contracts::canonical;
use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1};
use map2model_mesh::{build_mesh, inspect_mesh};

fn fixture() -> (ElevationGridV1, TerrainBuildRequestV1) {
    let grid: ElevationGridV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
    ))
    .unwrap();
    let request: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
    ))
    .unwrap();
    (grid, request)
}
fn rehash(grid: &mut ElevationGridV1, request: &mut TerrainBuildRequestV1) {
    grid.grid_sha256 = canonical::grid_digest(grid).unwrap();
    request.grid_sha256 = grid.grid_sha256.clone();
}

#[test]
fn canonical_180_by_80_by_204_mm_reference_is_closed_oriented_and_deterministic() {
    let (grid, request) = fixture();
    let output = build_mesh(&grid, &request).unwrap();
    assert_eq!(output.mesh.columns, 4);
    assert_eq!(output.mesh.rows, 3);
    assert_eq!(output.geometry.vertices, 24);
    assert_eq!(output.geometry.triangles, 44);
    assert_eq!(output.geometry.measured_aabb_mm, [0.0, 0.0, 0.0, 180.0, 80.0, 204.0]);
    assert_eq!(output.geometry.minimum_measured_thickness_mm, 4.0);
    assert!((output.geometry.volume_mm3 - 1_449_600.0).abs() < 1e-6);
    assert_eq!(output.geometry.units, "millimeter");
    assert_eq!(output.geometry.component_count, 1);
    assert_eq!(output.geometry.part_count, 1);
    assert_eq!(output.geometry.non_manifold_edges, 0);
    assert_eq!(output.geometry.degenerate_triangles, 0);
    assert!(output.geometry.closed);
    assert!(output.geometry.oriented);
    assert_eq!(output.geometry.self_intersection_status, "passed");
    assert_eq!(output.geometry.validation.status, "passed");
    assert_eq!(
        output.canonical_geometry_sha256,
        "dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9"
    );
    let again = build_mesh(&grid, &request).unwrap();
    assert_eq!(output.mesh.vertices_mm, again.mesh.vertices_mm);
    assert_eq!(output.mesh.triangles, again.mesh.triangles);
    assert_eq!(output.canonical_geometry_sha256, again.canonical_geometry_sha256);
}

#[test]
fn flat_negative_terrain_is_a_solid_prism_with_explicit_base() {
    let (mut grid, mut req) = fixture();
    grid.samples_meters = Some(vec![Some(-10.0); grid.width * grid.height]);
    req.elevation_zero_meters = -10.0;
    req.base_thickness_mm = 3.0;
    req.vertical_exaggeration = 1.0;
    rehash(&mut grid, &mut req);
    let o = build_mesh(&grid, &req).unwrap();
    assert_eq!(o.geometry.measured_aabb_mm, [0.0, 0.0, 0.0, 180.0, 80.0, 3.0]);
    assert!((o.geometry.volume_mm3 - 43_200.0).abs() < 1e-6);
    assert_eq!(o.geometry.minimum_measured_thickness_mm, 3.0);
}

#[test]
fn signed_asymmetric_north_up_grid_has_correct_height_and_axes() {
    let (grid, request) = fixture();
    let o = build_mesh(&grid, &request).unwrap();
    assert_eq!(o.mesh.vertices_mm[0], [0.0, 80.0, 34.0]); // NW = -5 m
    assert_eq!(o.mesh.vertices_mm[3], [180.0, 80.0, 144.0]); // NE = +6 m
    assert_eq!(o.mesh.vertices_mm[8], [0.0, 0.0, 4.0]); // SW = -8 m
    assert_eq!(o.mesh.vertices_mm[11], [180.0, 0.0, 204.0]); // SE = +12 m
}

#[test]
fn bounded_node_aligned_clip_is_not_stretched() {
    let (grid, mut req) = fixture();
    req.clip_rect_meters = [12.0, 0.0, 36.0, 16.0];
    req.model_width_mm = 120.0;
    req.model_height_mm = 80.0;
    let o = build_mesh(&grid, &req).unwrap();
    assert_eq!(o.geometry.vertices, 18);
    assert_eq!(o.geometry.triangles, 32);
    assert_eq!(o.geometry.measured_aabb_mm, [0.0, 0.0, 0.0, 120.0, 80.0, 204.0]);
    assert_eq!(o.mesh.vertices_mm[0], [0.0, 80.0, 74.0]);
}

#[test]
fn missing_in_clip_and_unresolved_bulk_tokens_are_rejected() {
    let (mut grid, mut req) = fixture();
    grid.samples_meters.as_mut().unwrap()[5] = None;
    grid.valid_mask.as_mut().unwrap()[5] = false;
    rehash(&mut grid, &mut req);
    let err = build_mesh(&grid, &req).unwrap_err();
    assert_eq!(err.code, "DATA_MISSING");
    let (mut grid, req) = fixture();
    grid.storage_mode = "local-binary-token".into();
    grid.samples_meters = None;
    grid.valid_mask = None;
    grid.grid_data_sha256 = Some("f".repeat(64));
    grid.grid_data_token = Some(format!("m2m-grid:{}", "f".repeat(64)));
    assert_eq!(build_mesh(&grid, &req).unwrap_err().code, "DATA_MISSING");
}

#[test]
fn print_configuration_errors_are_typed_and_fail_closed() {
    let (grid, req) = fixture();
    let mut bad = req.clone();
    bad.model_height_mm = 90.0;
    assert_eq!(build_mesh(&grid, &bad).unwrap_err().code, "INVALID_DIMENSIONS");
    bad = req.clone();
    bad.clip_rect_meters[0] = 0.5;
    assert_eq!(build_mesh(&grid, &bad).unwrap_err().code, "INVALID_DIMENSIONS");
    bad = req.clone();
    bad.elevation_zero_meters = 0.0;
    assert_eq!(build_mesh(&grid, &bad).unwrap_err().code, "INVALID_DIMENSIONS");
    bad = req.clone();
    bad.minimum_feature_thickness_mm = 5.0;
    assert_eq!(build_mesh(&grid, &bad).unwrap_err().code, "INVALID_GEOMETRY");
    bad = req.clone();
    bad.selection_id = "sel-wrong".into();
    assert_eq!(build_mesh(&grid, &bad).unwrap_err().code, "HASH_MISMATCH");
    bad = req.clone();
    bad.schema_version = "9.0.0".into();
    assert_eq!(build_mesh(&grid, &bad).unwrap_err().code, "INVALID_REQUEST");
    let mut tampered = grid;
    tampered.samples_meters.as_mut().unwrap()[1] = Some(1_000.0);
    assert_eq!(build_mesh(&tampered, &req).unwrap_err().code, "INVALID_GRID_HASH");
}

#[test]
fn mesh_inspector_rejects_winding_folds_zero_thickness_and_bad_faces() {
    let (grid, req) = fixture();
    let output = build_mesh(&grid, &req).unwrap();

    let mut wrong = output.mesh.clone();
    wrong.triangles[0].swap(1, 2);
    assert_eq!(inspect_mesh(&wrong).unwrap_err().code, "INVALID_GEOMETRY");

    let mut wrong = output.mesh.clone();
    wrong.vertices_mm[1][0] = wrong.vertices_mm[0][0] - 1.0;
    assert_eq!(inspect_mesh(&wrong).unwrap_err().code, "INVALID_GEOMETRY");

    let mut wrong = output.mesh.clone();
    wrong.vertices_mm[0][2] = 0.0;
    assert_eq!(inspect_mesh(&wrong).unwrap_err().code, "INVALID_GEOMETRY");

    let mut wrong = output.mesh.clone();
    wrong.triangles.pop();
    assert_eq!(inspect_mesh(&wrong).unwrap_err().code, "NON_MANIFOLD_MESH");

    let mut wrong = output.mesh;
    wrong.vertices_mm[0][2] = f64::INFINITY;
    assert_eq!(inspect_mesh(&wrong).unwrap_err().code, "INVALID_GEOMETRY");
}
