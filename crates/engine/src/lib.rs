//! A3-01: deterministic, closed terrain relief from validated V1 grid nodes.
//! No PMTiles, renderer, IPC, 3MF writer, or network dependencies.
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use map2model_contracts::canonical::{canonical_mesh_digest, MeshTriangle};
use map2model_contracts::validation;
use map2model_contracts::{ElevationGridV1, GeometryReportV1, MeshValidationV1, TerrainBuildRequestV1};

const MAX_MESH_NODES: usize = 100_000;
const EPS_MM: f64 = 1e-9;
const MIN_CROSS_SQUARED: f64 = 1e-24;

/// Stable G0 reason code plus indexed diagnostics, suitable for a failed job event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshError {
    pub code: &'static str,
    pub message: String,
    pub sample_index: Option<usize>,
    pub triangle_index: Option<usize>,
}
impl MeshError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            sample_index: None,
            triangle_index: None,
        }
    }
    fn from_contract(error: String) -> Self {
        let code = match error.split(':').next().unwrap_or("") {
            "DATA_MISSING" => "DATA_MISSING",
            "INCOMPLETE_GRID" => "INCOMPLETE_GRID",
            "INVALID_GRID_HASH" => "INVALID_GRID_HASH",
            "INVALID_DIMENSIONS" => "INVALID_DIMENSIONS",
            "INVALID_CRS" => "INVALID_CRS",
            "UNSUPPORTED_DATUM" => "UNSUPPORTED_DATUM",
            "HASH_MISMATCH" => "HASH_MISMATCH",
            "OUT_OF_MEMORY" => "OUT_OF_MEMORY",
            "INVALID_SOURCE" => "INVALID_SOURCE",
            _ => "INVALID_REQUEST",
        };
        Self::new(code, error)
    }
    fn at_triangle(mut self, index: usize) -> Self {
        self.triangle_index = Some(index);
        self
    }
    fn at_sample(mut self, index: usize) -> Self {
        self.sample_index = Some(index);
        self
    }
}
impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl Error for MeshError {}

/// Mesh topology is a single part, with top nodes followed by matching bottom nodes.
/// Row zero is northernmost; every bottom vertex has Z=0.
#[derive(Debug, Clone)]
pub struct TerrainMesh {
    pub vertices_mm: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    pub columns: usize,
    pub rows: usize,
}

#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub mesh: TerrainMesh,
    pub geometry: GeometryReportV1,
    pub canonical_geometry_sha256: String,
}

fn surface_faces(columns: usize, rows: usize) -> Vec<[usize; 3]> {
    let top_count = columns * rows;
    let mut faces = Vec::with_capacity(
        4 * (columns - 1) * (rows - 1) + 4 * (columns + rows - 2),
    );
    for row in 0..rows - 1 {
        for col in 0..columns - 1 {
            let nw = row * columns + col;
            let ne = nw + 1;
            let sw = nw + columns;
            let se = sw + 1;
            // North-up rows decrease in Y. These top faces have +Z winding.
            faces.push([nw, sw, ne]);
            faces.push([ne, sw, se]);
            // The flat bottom has the opposite (-Z) winding.
            faces.push([nw + top_count, ne + top_count, sw + top_count]);
            faces.push([ne + top_count, se + top_count, sw + top_count]);
        }
    }

    // Perimeter travels CCW as seen from above: south, east, north, west.
    let mut perimeter = Vec::with_capacity(2 * (columns + rows - 2));
    for col in 0..columns {
        perimeter.push((rows - 1) * columns + col);
    }
    for row in (0..rows - 1).rev() {
        perimeter.push(row * columns + columns - 1);
    }
    for col in (0..columns - 1).rev() {
        perimeter.push(col);
    }
    for row in 1..rows - 1 {
        perimeter.push(row * columns);
    }
    for index in 0..perimeter.len() {
        let a = perimeter[index];
        let b = perimeter[(index + 1) % perimeter.len()];
        // These faces point to the right of the CCW perimeter (outward).
        faces.push([a, a + top_count, b]);
        faces.push([b, a + top_count, b + top_count]);
    }
    faces
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// O(V+T) structural, edge incidence, winding, component and signed-volume QA.
/// Self-intersection is ruled out by the verified heightfield topology: each
/// XY rectangle is independently triangulated exactly once, all top Z > 0,
/// the base is Z=0, and non-overlapping vertical walls follow the perimeter.
/// This is NOT a general triangle-soup self-intersection detector.
pub fn inspect_mesh(mesh: &TerrainMesh) -> Result<GeometryReportV1, MeshError> {
    let columns = mesh.columns;
    let rows = mesh.rows;
    let node_count = columns
        .checked_mul(rows)
        .ok_or_else(|| MeshError::new("OUT_OF_MEMORY", "mesh node-count overflow"))?;
    if columns < 2 || rows < 2 || node_count > MAX_MESH_NODES {
        return Err(MeshError::new("OUT_OF_MEMORY", "mesh dimensions exceed G1 budget"));
    }
    if mesh.vertices_mm.len() != 2 * node_count {
        return Err(MeshError::new("INVALID_GEOMETRY", "top/bottom node pairing is incomplete"));
    }
    for (index, vertex) in mesh.vertices_mm.iter().enumerate() {
        if !vertex.iter().all(|v| v.is_finite()) {
            return Err(MeshError::new("INVALID_GEOMETRY", "non-finite vertex").at_sample(index));
        }
    }
    let mut min_thickness = f64::INFINITY;
    for row in 0..rows {
        for col in 0..columns {
            let index = row * columns + col;
            let top = mesh.vertices_mm[index];
            let bottom = mesh.vertices_mm[node_count + index];
            if (top[0] - bottom[0]).abs() > EPS_MM
                || (top[1] - bottom[1]).abs() > EPS_MM
                || bottom[2].abs() > EPS_MM
                || top[2] <= EPS_MM
            {
                return Err(
                    MeshError::new("INVALID_GEOMETRY", "bottom must be flat and strictly below top")
                        .at_sample(index),
                );
            }
            // Each row has the same X coordinates; each column has the same Y.
            if (top[0] - mesh.vertices_mm[col][0]).abs() > EPS_MM
                || (top[1] - mesh.vertices_mm[row * columns][1]).abs() > EPS_MM
                || (col > 0 && top[0] - mesh.vertices_mm[index - 1][0] <= EPS_MM)
                || (row > 0 && mesh.vertices_mm[index - columns][1] - top[1] <= EPS_MM)
            {
                return Err(
                    MeshError::new("INVALID_GEOMETRY", "non-monotonic or folded XY heightfield")
                        .at_sample(index),
                );
            }
            min_thickness = min_thickness.min(top[2]);
        }
    }
    let expected = surface_faces(columns, rows);
    if mesh.triangles.len() != expected.len() {
        return Err(MeshError::new("NON_MANIFOLD_MESH", "missing or extra terrain faces"));
    }
    for (index, (actual, reference)) in mesh.triangles.iter().zip(expected.iter()).enumerate() {
        if actual != reference {
            return Err(
                MeshError::new("INVALID_GEOMETRY", "terrain face indexing/winding differs")
                    .at_triangle(index),
            );
        }
    }

    // Edge orientations must cancel pairwise across a closed, oriented surface.
    let mut edges: HashMap<(usize, usize), (u8, i8)> = HashMap::new();
    let mut used = HashSet::new();
    let mut signed_six_volume = 0.0;
    for (index, [a, b, c]) in mesh.triangles.iter().copied().enumerate() {
        if a == b || a == c || b == c || c >= mesh.vertices_mm.len()
            || a >= mesh.vertices_mm.len() || b >= mesh.vertices_mm.len()
        {
            return Err(
                MeshError::new("INVALID_GEOMETRY", "triangle has invalid/repeated vertices")
                    .at_triangle(index),
            );
        }
        let va = mesh.vertices_mm[a];
        let vb = mesh.vertices_mm[b];
        let vc = mesh.vertices_mm[c];
        let n = cross(subtract(vb, va), subtract(vc, va));
        if !dot(n, n).is_finite() || dot(n, n) <= MIN_CROSS_SQUARED {
            return Err(
                MeshError::new("INVALID_GEOMETRY", "degenerate or non-finite triangle area")
                    .at_triangle(index),
            );
        }
        signed_six_volume += dot(va, cross(vb, vc));
        used.extend([a, b, c]);
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let key = if u < v { (u, v) } else { (v, u) };
            let edge = edges.entry(key).or_insert((0, 0));
            edge.0 = edge.0.saturating_add(1);
            edge.1 += if u < v { 1 } else { -1 };
        }
    }
    if used.len() != mesh.vertices_mm.len()
        || edges.values().any(|(count, direction)| *count != 2 || *direction != 0)
    {
        return Err(MeshError::new("NON_MANIFOLD_MESH", "edge incidence or winding is invalid"));
    }
    // All top nodes are connected by the rectangular triangulation, and each
    // bottom node joins the same connected shell via sidewalls.
    let component_count = 1;
    let volume = signed_six_volume / 6.0;
    if !volume.is_finite() || volume <= 0.0 {
        return Err(MeshError::new("INVALID_GEOMETRY", "non-positive oriented volume"));
    }
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for vertex in &mesh.vertices_mm {
        for axis in 0..3 {
            bounds[axis] = bounds[axis].min(vertex[axis]);
            bounds[axis + 3] = bounds[axis + 3].max(vertex[axis]);
        }
    }
    Ok(GeometryReportV1 {
        units: "millimeter".into(),
        vertices: mesh.vertices_mm.len(),
        triangles: mesh.triangles.len(),
        closed: true,
        oriented: true,
        non_manifold_edges: 0,
        degenerate_triangles: 0,
        minimum_measured_thickness_mm: min_thickness,
        measured_aabb_mm: bounds,
        component_count,
        part_count: 1,
        volume_mm3: volume,
        self_intersection_status: "passed".into(),
        validation: MeshValidationV1 {
            status: "passed".into(),
            checks: vec![
                "heightfield-xy-monotonic".into(),
                "constructive-no-self-intersection".into(),
                "oriented-edge-incidence".into(),
                "nondegenerate-triangles".into(),
                "single-connected-shell".into(),
                "positive-signed-volume".into(),
            ],
            diagnostics: vec![],
        },
    })
}

/// Only valid, decoded ElevationGridV1 data may enter. Bulk-token envelopes
/// must first be hydrated and independently verified by the native registry.
/// The frozen input structs and hashing code are used without modification.
pub fn build_mesh(
    grid: &ElevationGridV1,
    request: &TerrainBuildRequestV1,
) -> Result<BuildOutput, MeshError> {
    validation::validate_build(request, grid).map_err(MeshError::from_contract)?;
    if request.schema_version != "1.0.0" || request.kind != "TerrainBuildRequestV1" {
        return Err(MeshError::new("INVALID_REQUEST", "unsupported build request version/kind"));
    }
    let [west, south, east, north] = request.clip_rect_meters;
    let scale = request.model_width_mm / (east - west);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(MeshError::new("INVALID_DIMENSIONS", "invalid geographic print scale"));
    }
    let col_start = ((west - grid.min_east_meters) / grid.dx_meters).round() as usize;
    let col_end = ((east - grid.min_east_meters) / grid.dx_meters).round() as usize;
    let row_start = ((grid.max_north_meters - north) / grid.dy_meters).round() as usize;
    let row_end = ((grid.max_north_meters - south) / grid.dy_meters).round() as usize;
    if col_end >= grid.width || row_end >= grid.height || col_end <= col_start
        || row_end <= row_start
    {
        return Err(MeshError::new("INVALID_DIMENSIONS", "clip node indices invalid"));
    }
    let columns = col_end - col_start + 1;
    let rows = row_end - row_start + 1;
    let count = columns
        .checked_mul(rows)
        .ok_or_else(|| MeshError::new("OUT_OF_MEMORY", "mesh node-count overflow"))?;
    if count > MAX_MESH_NODES {
        return Err(MeshError::new("OUT_OF_MEMORY", "G1 engine limit: 100000 clipped nodes"));
    }
    let elevations = grid.samples_meters.as_ref().ok_or_else(|| {
        MeshError::new("DATA_MISSING", "the grid is not decoded and verified")
    })?;
    let mut vertices = Vec::with_capacity(2 * count);
    for row in 0..rows {
        for col in 0..columns {
            let source_row = row_start + row;
            let source_col = col_start + col;
            let sample_index = source_row * grid.width + source_col;
            let height = elevations[sample_index].ok_or_else(|| {
                MeshError::new("DATA_MISSING", "clipped elevation sample is absent")
                    .at_sample(sample_index)
            })?;
            let local_east = grid.min_east_meters + source_col as f64 * grid.dx_meters;
            let local_north = grid.max_north_meters - source_row as f64 * grid.dy_meters;
            let x = if col == 0 { 0.0 } else if col + 1 == columns {
                request.model_width_mm
            } else {
                (local_east - west) * scale
            };
            let y = if row == 0 { request.model_height_mm } else if row + 1 == rows {
                0.0
            } else {
                (local_north - south) * scale
            };
            let z = request.base_thickness_mm
                + (height - request.elevation_zero_meters) * scale
                    * request.vertical_exaggeration;
            if !z.is_finite() || z <= EPS_MM {
                return Err(
                    MeshError::new("INVALID_DIMENSIONS", "terrain Z is not finite/positive")
                        .at_sample(sample_index),
                );
            }
            vertices.push([x, y, z]);
        }
    }
    for index in 0..count {
        let v = vertices[index];
        vertices.push([v[0], v[1], 0.0]);
    }
    let mesh = TerrainMesh {
        vertices_mm: vertices,
        triangles: surface_faces(columns, rows),
        columns,
        rows,
    };
    let geometry = inspect_mesh(&mesh)?;
    if geometry.minimum_measured_thickness_mm + 0.05
        < request.base_thickness_mm.max(request.minimum_feature_thickness_mm)
    {
        return Err(MeshError::new("INVALID_GEOMETRY", "terrain minimum thickness below print constraint"));
    }
    if (geometry.measured_aabb_mm[3] - request.model_width_mm).abs() > 0.05
        || (geometry.measured_aabb_mm[4] - request.model_height_mm).abs() > 0.05
        || geometry.measured_aabb_mm[2].abs() > 0.05
    {
        return Err(MeshError::new("INVALID_DIMENSIONS", "measured mesh dimensions differ from request"));
    }
    let triangles: Vec<MeshTriangle> = mesh.triangles.iter().map(|indices| MeshTriangle {
        part_id: "terrain".into(),
        indices: *indices,
    }).collect();
    let digest = canonical_mesh_digest(&mesh.vertices_mm, &triangles)
        .map_err(|reason| MeshError::new("INVALID_GEOMETRY", reason))?;
    Ok(BuildOutput {
        mesh,
        geometry,
        canonical_geometry_sha256: digest,
    })
}
