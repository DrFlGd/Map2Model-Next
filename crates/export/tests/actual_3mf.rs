use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1, TerrainSourceManifestV1};
use map2model_export::{
    package::{export_3mf, inspect_3mf, write_3mf_atomic},
    prepare_export,
};
use map2model_mesh::build_mesh;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn fixtures() -> (
    ElevationGridV1,
    TerrainBuildRequestV1,
    TerrainSourceManifestV1,
) {
    (
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/TerrainBuildRequestV1.valid.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/TerrainSourceManifestV1.valid.json"
        ))
        .unwrap(),
    )
}
fn rewrite_part(bytes: &[u8], part: &str, transform: impl Fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        entries.push((file.name().to_string(), contents));
    }
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in entries {
        writer
            .start_file(
                &name,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        let bytes = if name == part {
            transform(&content)
        } else {
            content
        };
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn physical_synthetic_3mf_archive_has_correct_opc_parts_mm_dimensions_provenance_and_three_distinct_hashes(
) {
    let (grid, request, source) = fixtures();
    let mesh = build_mesh(&grid, &request).unwrap();
    let artifact = export_3mf(&mesh, &grid, &request, &source).unwrap();
    assert!(artifact.bytes.starts_with(b"PK"));
    assert!(artifact.bytes.len() > 2_000);
    assert_eq!(artifact.vertices, 24);
    assert_eq!(artifact.triangles, 44);
    assert_eq!(artifact.aabb_mm, [0., 0., 0., 180., 80., 204.]);
    assert_eq!(
        artifact.canonical_geometry_sha256,
        mesh.canonical_geometry_sha256
    );
    assert_eq!(
        artifact.package_sha256,
        format!("{:x}", Sha256::digest(&artifact.bytes))
    );
    assert_ne!(artifact.package_sha256, artifact.canonical_geometry_sha256);
    assert_ne!(artifact.package_sha256, artifact.provenance_sha256);
    assert_ne!(
        artifact.canonical_geometry_sha256,
        artifact.provenance_sha256
    );
    let mut zipped = ZipArchive::new(Cursor::new(&artifact.bytes)).unwrap();
    assert_eq!(zipped.len(), 4);
    let mut mimetypes = String::new();
    zipped
        .by_name("[Content_Types].xml")
        .unwrap()
        .read_to_string(&mut mimetypes)
        .unwrap();
    assert!(mimetypes.contains("application/vnd.ms-package.3dmanufacturing-3dmodel+xml"));
    let mut rels = String::new();
    zipped
        .by_name("_rels/.rels")
        .unwrap()
        .read_to_string(&mut rels)
        .unwrap();
    assert!(rels.contains("mustpreserve"));
    let mut xml = String::new();
    zipped
        .by_name("3D/3dmodel.model")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("unit=\"millimeter\""));
    assert_eq!(xml.matches("<vertex ").count(), 24);
    assert_eq!(xml.matches("<triangle ").count(), 44);
    assert!(xml.contains("CC0-1.0"));
    assert!(xml.contains("Artificial elevation samples"));
    let mut provenance = Vec::new();
    zipped
        .by_name("Metadata/map2model.provenance.json")
        .unwrap()
        .read_to_end(&mut provenance)
        .unwrap();
    assert_eq!(
        artifact.provenance_sha256,
        format!("{:x}", Sha256::digest(&provenance))
    );
    let parsed: serde_json::Value = serde_json::from_slice(&provenance).unwrap();
    assert_eq!(parsed["sourceLicenseIdentifier"], "CC0-1.0");
    let again = export_3mf(&mesh, &grid, &request, &source).unwrap();
    assert_eq!(
        artifact.bytes, again.bytes,
        "same inputs imply byte-for-byte reproducible 3MF ZIP"
    );
    assert_eq!(artifact.package_sha256, again.package_sha256);
}
#[test]
fn rejects_tampered_opc_relationship_provenance_dimensions_mesh_and_bad_zip() {
    let (grid, request, source) = fixtures();
    let mesh = build_mesh(&grid, &request).unwrap();
    let prepare = prepare_export(&mesh, &grid, &request, &source).unwrap();
    let artifact = export_3mf(&mesh, &grid, &request, &source).unwrap();
    assert!(inspect_3mf(b"not a ZIP", &mesh, &prepare).is_err());
    let swapped = rewrite_part(&artifact.bytes, "_rels/.rels", |s| {
        String::from_utf8(s.to_vec())
            .unwrap()
            .replace("mustpreserve", "OTHERREL")
            .into_bytes()
    });
    assert!(inspect_3mf(&swapped, &mesh, &prepare).is_err());
    let swapped = rewrite_part(&artifact.bytes, "[Content_Types].xml", |s| {
        String::from_utf8(s.to_vec())
            .unwrap()
            .replace("application/json", "text/plain")
            .into_bytes()
    });
    assert!(inspect_3mf(&swapped, &mesh, &prepare).is_err());
    let swapped = rewrite_part(&artifact.bytes, "Metadata/map2model.provenance.json", |s| {
        String::from_utf8(s.to_vec())
            .unwrap()
            .replace("CC0-1.0", "unknown")
            .into_bytes()
    });
    assert_eq!(
        inspect_3mf(&swapped, &mesh, &prepare).unwrap_err().code,
        "HASH_MISMATCH"
    );
    let swapped = rewrite_part(&artifact.bytes, "3D/3dmodel.model", |s| {
        String::from_utf8(s.to_vec())
            .unwrap()
            .replace("unit=\"millimeter\"", "unit=\"inch\"")
            .into_bytes()
    });
    assert!(inspect_3mf(&swapped, &mesh, &prepare).is_err());
    let swapped = rewrite_part(&artifact.bytes, "3D/3dmodel.model", |s| {
        String::from_utf8(s.to_vec())
            .unwrap()
            .replace("v1=\"0\"", "v1=\"999999\"")
            .into_bytes()
    });
    assert!(inspect_3mf(&swapped, &mesh, &prepare).is_err());
}
#[test]
fn genuine_3mf_atomic_publication_respects_cancel_and_never_clobbers() {
    let (grid, request, source) = fixtures();
    let mesh = build_mesh(&grid, &request).unwrap();
    let exported = export_3mf(&mesh, &grid, &request, &source).unwrap();
    let parent = std::env::temp_dir().join(format!(
        "map2model-a3-02-{}-{}",
        std::process::id(),
        &exported.package_sha256[..10]
    ));
    std::fs::create_dir_all(&parent).unwrap();
    let path = parent.join("fixture.3mf");
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        write_3mf_atomic(&path, &exported, || true)
            .unwrap_err()
            .code,
        "CANCELLED"
    );
    assert!(!path.exists());
    assert_eq!(
        write_3mf_atomic(&path, &exported, {
            let mut count = 0;
            move || {
                count += 1;
                count > 1
            }
        })
        .unwrap_err()
        .code,
        "CANCELLED"
    );
    assert!(!path.exists());
    write_3mf_atomic(&path, &exported, || false).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), exported.bytes);
    assert!(write_3mf_atomic(&path, &exported, || false).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), exported.bytes);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(parent).unwrap();
}

#[test]
fn readback_matches_checked_in_a3_01_reference_manifest() {
    let (grid, request, source) = fixtures();
    let mesh = build_mesh(&grid, &request).unwrap();
    let exported = export_3mf(&mesh, &grid, &request, &source).unwrap();
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../engine/fixtures/A3-01.reference.json")).unwrap();
    assert_eq!(
        reference["vertices"].as_u64(),
        Some(exported.vertices as u64)
    );
    assert_eq!(
        reference["triangles"].as_u64(),
        Some(exported.triangles as u64)
    );
    assert_eq!(
        reference["canonicalGeometrySha256"],
        exported.canonical_geometry_sha256
    );
    for i in 0..6 {
        assert!(
            (reference["measuredAabbMm"][i].as_f64().unwrap() - exported.aabb_mm[i]).abs() < 1e-6
        );
    }
}
