//! Developer-only, network-free G1 geodata inspection and decode entrypoint.
//! NOT an Electron/renderer IPC endpoint and not a source-rights approval path.
use map2model_contracts::{validation, SelectionV1};
use map2model_data::terrarium::decode_selection_grid;
use map2model_data::{import_local_pmtiles, inspect_unverified_local_pmtiles, SourceDeclaration};
use serde::de::DeserializeOwned;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

fn usage() -> &'static str {
    "Usage:\n  terrain_offline inspect <local.pmtiles> [zoom x y]\n  terrain_offline decode <local.pmtiles> <private-registry-dir> <source-declaration.json> <selection.json> <zoom> <output-grid.json>\n\ninspect: offline format/512px verification only; source datum/license unverified; never Generation Ready.\ndecode: requires explicit verified *caller declaration* and frozen SelectionV1; create-new grid output; NOT independent rights/datum approval."
}
fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let info = fs::symlink_metadata(path).map_err(|_| "DATA_MISSING: JSON input not found")?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() > 16_384 {
        return Err("INVALID_SOURCE: JSON argument must be a bounded ordinary file".into());
    }
    let mut f = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|_| "DATA_MISSING: cannot read JSON argument")?;
    let mut data = Vec::new();
    Read::by_ref(&mut f)
        .take(16_385)
        .read_to_end(&mut data)
        .map_err(|_| "INVALID_SOURCE: cannot read JSON argument")?;
    if data.len() > 16_384 || data.len() as u64 != info.len() {
        return Err("INVALID_SOURCE: JSON file changed during read".into());
    }
    serde_json::from_slice(&data).map_err(|_| "INVALID_SOURCE: JSON input schema invalid".into())
}
fn parse<T: std::str::FromStr>(v: &str, description: &str) -> Result<T, String> {
    v.parse()
        .map_err(|_| format!("INVALID_SELECTION: {description} must be a nonnegative integer"))
}
fn execute(args: &[String]) -> Result<serde_json::Value, String> {
    match args {
        [action, file] if action == "inspect" => {
            inspect_unverified_local_pmtiles(Path::new(file), None)
        }
        [action, file, z, x, y] if action == "inspect" => {
            let tile = Some((parse(z, "zoom")?, parse(x, "x")?, parse(y, "y")?));
            inspect_unverified_local_pmtiles(Path::new(file), tile)
        }
        [action, file, registry, source_json, selection_json, zoom, output]
            if action == "decode" =>
        {
            let source: SourceDeclaration = read_json(Path::new(source_json))?;
            // A production-shaped headless decode must pin the exact local file.
            // A mutable provider URL or invented release label cannot replace a SHA.
            if source.expected_archive_sha256.is_none() {
                return Err("INVALID_SOURCE: headless decode requires expectedArchiveSha256 from independently recorded local acquisition".into());
            }
            let selection: SelectionV1 = read_json(Path::new(selection_json))?;
            validation::validate_selection(&selection)?;
            let zoom: u8 = parse(zoom, "zoom")?;
            let imported = import_local_pmtiles(Path::new(file), Path::new(registry), &source)?;
            let grid = decode_selection_grid(&imported, &selection, zoom)?;
            let json = serde_json::to_vec_pretty(&grid)
                .map_err(|_| "INVALID_SOURCE: canonical grid cannot be serialized")?;
            let mut f = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output)
                .map_err(|_| {
                    "INVALID_SOURCE: output grid path already exists or cannot be created"
                })?;
            if let Err(e) = f.write_all(&json).and_then(|()| f.sync_all()) {
                drop(f);
                let _ = fs::remove_file(output);
                return Err(format!("INVALID_SOURCE: grid output write failed: {e}"));
            }
            Ok(serde_json::json!({
                "result": "decoded-v1-grid-source-authorization-externally-declared",
                "outputGridPath": output,
                "sourceArchiveSha256": imported.manifest.source_archive_sha256,
                "manifestSha256": imported.manifest.manifest_sha256,
                "gridSha256": grid.grid_sha256,
                "selectionId": grid.selection_id,
                "gridWidth": grid.width,
                "gridHeight": grid.height,
                "verticalDatumId": grid.vertical_datum_id,
                "licenseAndDatumIndependentlyVerified": false,
                "productionGenerationReady": false,
                "networkRequests": 0
            }))
        }
        _ => Err(format!("INVALID_REQUEST: {}", usage())),
    }
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match execute(&args) {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        ),
        Err(reason) => {
            eprintln!("{reason}");
            std::process::exit(2);
        }
    }
}
