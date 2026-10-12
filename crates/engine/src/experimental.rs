//! G2 unverified candidate-height mesh, never a G0 ElevationGridV1.
use crate::{inspect_mesh, BuildOutput, MeshError, TerrainMesh};
use map2model_contracts::canonical::{canonical_mesh_digest, MeshTriangle};

fn error(code: &'static str, message: &str) -> MeshError {
    MeshError {
        code,
        message: message.into(),
        sample_index: None,
        triangle_index: None,
    }
}

/// Print the actual finite candidate samples against their LOCAL sample minimum.
/// Does not infer any absolute elevation or vertical CRS.
// The separately typed experimental surface parameters are intentionally explicit.
#[allow(clippy::too_many_arguments)]
pub fn build_candidate(
    width: usize,
    height: usize,
    dx_m: f64,
    dy_m: f64,
    samples: &[f64],
    width_mm: f64,
    base_mm: f64,
    exaggeration: f64,
) -> Result<BuildOutput, MeshError> {
    let count = width
        .checked_mul(height)
        .ok_or_else(|| error("OUT_OF_MEMORY", "candidate count overflow"))?;
    if width < 2
        || height < 2
        || count > 100_000
        || samples.len() != count
        || !dx_m.is_finite()
        || dx_m <= 0.
        || !dy_m.is_finite()
        || dy_m <= 0.
        || !width_mm.is_finite()
        || !(20. ..=300.).contains(&width_mm)
        || !base_mm.is_finite()
        || !(1. ..=30.).contains(&base_mm)
        || !exaggeration.is_finite()
        || !(0.1..=10.).contains(&exaggeration)
    {
        return Err(error(
            "INVALID_DIMENSIONS",
            "experimental print dimensions are invalid",
        ));
    }
    if samples.iter().any(|v| !v.is_finite() || v.abs() > 100_000.) {
        return Err(error(
            "INVALID_GEOMETRY",
            "nonfinite or implausible candidate sample",
        ));
    }
    let x_span = dx_m * (width - 1) as f64;
    let y_span = dy_m * (height - 1) as f64;
    let scale = width_mm / x_span;
    let height_mm = y_span * scale;
    if !scale.is_finite() || !height_mm.is_finite() || !(10. ..=300.).contains(&height_mm) {
        return Err(error(
            "INVALID_DIMENSIONS",
            "model aspect outside print bounds",
        ));
    }
    let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let mut vertices = Vec::with_capacity(2 * count);
    for row in 0..height {
        for col in 0..width {
            let x = if col + 1 == width {
                width_mm
            } else {
                col as f64 * dx_m * scale
            };
            let y = if row == 0 {
                height_mm
            } else if row + 1 == height {
                0.
            } else {
                height_mm - row as f64 * dy_m * scale
            };
            let z = base_mm + (samples[row * width + col] - min) * scale * exaggeration;
            if !z.is_finite() || z <= 0. || z > 1_000_000. {
                return Err(error(
                    "INVALID_DIMENSIONS",
                    "invalid candidate print height",
                ));
            }
            vertices.push([x, y, z]);
        }
    }
    for i in 0..count {
        let p = vertices[i];
        vertices.push([p[0], p[1], 0.]);
    }
    let mut triangles =
        Vec::with_capacity(4 * (width - 1) * (height - 1) + 4 * (width + height - 2));
    for row in 0..height - 1 {
        for col in 0..width - 1 {
            let nw = row * width + col;
            let ne = nw + 1;
            let sw = nw + width;
            let se = sw + 1;
            triangles.extend([
                [nw, sw, ne],
                [ne, sw, se],
                [nw + count, ne + count, sw + count],
                [ne + count, se + count, sw + count],
            ]);
        }
    }
    let mut edge = Vec::with_capacity(2 * (width + height - 2));
    for col in 0..width {
        edge.push((height - 1) * width + col);
    }
    for row in (0..height - 1).rev() {
        edge.push(row * width + width - 1);
    }
    for col in (0..width - 1).rev() {
        edge.push(col);
    }
    for row in 1..height - 1 {
        edge.push(row * width);
    }
    for i in 0..edge.len() {
        let a = edge[i];
        let b = edge[(i + 1) % edge.len()];
        triangles.extend([[a, a + count, b], [b, a + count, b + count]]);
    }
    let mesh = TerrainMesh {
        vertices_mm: vertices,
        triangles,
        columns: width,
        rows: height,
    };
    let geometry = inspect_mesh(&mesh)?;
    if (geometry.measured_aabb_mm[3] - width_mm).abs() > 0.05
        || (geometry.measured_aabb_mm[4] - height_mm).abs() > 0.05
        || geometry.minimum_measured_thickness_mm < base_mm - 0.05
    {
        return Err(error("INVALID_GEOMETRY", "candidate geometry QA mismatch"));
    }
    let canonical = mesh
        .triangles
        .iter()
        .map(|t| MeshTriangle {
            part_id: "terrain".into(),
            indices: *t,
        })
        .collect::<Vec<_>>();
    let hash = canonical_mesh_digest(&mesh.vertices_mm, &canonical)
        .map_err(|msg| error("INVALID_GEOMETRY", &msg))?;
    Ok(BuildOutput {
        mesh,
        geometry,
        canonical_geometry_sha256: hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_relative_candidates_and_nonfinite_rejection() {
        let mesh = build_candidate(
            3,
            3,
            10.,
            10.,
            &[20., 21., 22., 19., 20., 21., 18., 19., 20.],
            60.,
            4.,
            1.,
        )
        .unwrap();
        assert!(mesh.geometry.closed && mesh.geometry.oriented);
        assert_eq!(mesh.geometry.vertices, 18);
        assert_eq!(mesh.geometry.triangles, 32);
        assert_eq!(mesh.geometry.component_count, 1);
        let mut corrupt = [20.; 9];
        corrupt[4] = f64::NAN;
        assert_eq!(
            build_candidate(3, 3, 10., 10., &corrupt, 60., 4., 1.)
                .unwrap_err()
                .code,
            "INVALID_GEOMETRY"
        );
    }
}
