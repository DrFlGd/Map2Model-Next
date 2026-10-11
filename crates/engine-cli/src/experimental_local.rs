//! Separate native experimental user-supplied local terrain path.
use map2model_contracts::{validation, SelectionV1};
use map2model_data::experimental::{decode_experimental_local_pmtiles, EXPLICIT_LOCAL_ONLY_ACK};
use map2model_export::package::{export_experimental_core, write_3mf_atomic};
use map2model_mesh::experimental::build_candidate;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
pub const POLICY: &str = "PRIVATE-LOCAL-TERRAIN-2026-10-11";
pub fn inspect(path: &Path) -> Result<Value,String> {
    map2model_data::inspect_unverified_local_pmtiles(path,None)
}
pub fn export(file: &Path, source_sha: &str, selection_path: &Path,
    zoom: u8, job: &str, policy: &str, destination: &Path) -> Result<Value,String> {
    if policy != POLICY || job.len() < 12 || job.len() > 128
        || !job.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        || !file.is_absolute() || !selection_path.is_absolute() || !destination.is_absolute()
        || destination.extension().and_then(|s|s.to_str()) != Some("3mf") || destination.exists() {
        return Err("INVALID_REQUEST: unsafe native experimental job".into());
    }
    let bytes=fs::read(selection_path).map_err(|_|"INVALID_SELECTION: cannot read selection")?;
    if bytes.len()>16384 {return Err("INVALID_SELECTION: selection too large".into());}
    let selection:SelectionV1=serde_json::from_slice(&bytes)
        .map_err(|_|"INVALID_SELECTION: invalid selection JSON")?;
    validation::validate_selection(&selection)?;
    let preview=decode_experimental_local_pmtiles(file,source_sha,&selection,zoom,EXPLICIT_LOCAL_ONLY_ACK)?;
    for (key,val) in [("experimentalOnly",true),("localPersonalUseOnly",true),
        ("ownerAcknowledgedUnverified",true),("encodingAuthenticated",false),
        ("sourceLicenseVerified",false),("nativeResolutionVerified",false),
        ("redistributionApproved",false),("productionGenerationReady",false)] {
        if preview[key]!=val {return Err("INVALID_SOURCE: forged experimental readiness".into());}
    }
    if preview["kind"]!="ExperimentalLocalTerrainPreviewV1"
        || preview["verticalDatumStatus"]!="unknown"
        || preview["storageConvention"]!="north-up-f64-row-major"
        || preview["selectionId"]!=selection.selection_id
        || preview["sourceArchiveSha256"]!=source_sha || preview["zoom"]!=zoom
        || preview["networkRequests"]!=0 {
        return Err("INVALID_SOURCE: experimental identity mismatch".into());
    }
    let w=preview["width"].as_u64().ok_or("INVALID_SOURCE: candidate width absent")? as usize;
    let h=preview["height"].as_u64().ok_or("INVALID_SOURCE: candidate height absent")? as usize;
    let dx=preview["dxMeters"].as_f64().ok_or("INVALID_SOURCE: candidate dx absent")?;
    let dy=preview["dyMeters"].as_f64().ok_or("INVALID_SOURCE: candidate dy absent")?;
    let list=preview["candidateTerrariumHeightsMeters"].as_array().ok_or("DATA_MISSING: candidate samples absent")?;
    let count=w.checked_mul(h).ok_or("OUT_OF_MEMORY: samples overflow")?;
    if w<2 || h<2 || count>100000 || list.len()!=count {
        return Err("DATA_MISSING: candidate shape mismatch".into());
    }
    let mut digest=Sha256::new();
    digest.update(b"M2M-EXPERIMENTAL-UNVERIFIED-HEIGHTS-1\0");
    digest.update(source_sha.as_bytes());
    digest.update(selection.selection_id.as_bytes());
    let mut samples=Vec::with_capacity(count);
    for v in list {
        let x=v.as_f64().ok_or("DATA_MISSING: invalid candidate sample")?;
        if !x.is_finite(){return Err("DATA_MISSING: nonfinite candidate".into());}
        digest.update(x.to_le_bytes());samples.push(x);
    }
    let candidate_sha=format!("{:x}",digest.finalize());
    if preview["candidateHeightsSha256"]!=candidate_sha
        || preview["bboxWgs84"]!=json!(selection.bbox) {
        return Err("HASH_MISMATCH: candidate selection/data digest mismatch".into());
    }
    let mesh=build_candidate(w,h,dx,dy,&samples,120.,4.,1.)
        .map_err(|e|format!("{}: {}",e.code,e.message))?;
