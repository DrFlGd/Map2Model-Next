//! Build the signed synthetic reference as a genuine ZIP-based .3mf file:
//! cargo run -p map2model-export --example synthetic_3mf -- path/to/terrain.3mf
use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1, TerrainSourceManifestV1};
use map2model_export::package::{export_3mf, write_3mf_atomic};
use map2model_mesh::build_mesh;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: cargo run -p map2model-export --example synthetic_3mf -- OUTPUT.3mf")?;
    let grid: ElevationGridV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
    ))?;
    let request: TerrainBuildRequestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
    ))?;
    let manifest: TerrainSourceManifestV1 = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"
    ))?;
    let mesh = build_mesh(&grid, &request)?;
    let exported = export_3mf(&mesh, &grid, &request, &manifest)?;
    write_3mf_atomic(std::path::Path::new(&path), &exported, || false)?;
    println!("file={path}");
    println!(
        "vertices={} triangles={}",
        exported.vertices, exported.triangles
    );
    println!("bounding_box_mm={:?}", exported.aabb_mm);
    println!("3mf_bytes={}", exported.bytes.len());
    println!(
        "canonical_geometry_sha256={}",
        exported.canonical_geometry_sha256
    );
    println!("provenance_sha256={}", exported.provenance_sha256);
    println!("export_file_sha256={}", exported.package_sha256);
    Ok(())
}
fn main() {
    if let Err(err) = run() {
        eprintln!("synthetic 3MF export failed: {err}");
        std::process::exit(1);
    }
}
