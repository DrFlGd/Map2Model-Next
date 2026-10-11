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
    let attribution=preview["sourceAttributionFromArchive"].as_str().filter(|s|s.len()<=2048);
    let local_min=samples.iter().copied().fold(f64::INFINITY,f64::min);
    let consent_sha=format!("{:x}",Sha256::digest(
        format!("{job}\0{source_sha}\0{}\0{zoom}\0{POLICY}",selection.selection_id).as_bytes()));
    let provenance=json!({
        "kind":"Experimental3mfProvenanceV1",
        "experimentalOnly":true,"localPersonalUseOnly":true,"ownerAcknowledgedUnverified":true,
        "encodingAuthenticated":false,"sourceLicenseVerified":false,
        "nativeResolutionVerified":false,"redistributionApproved":false,
        "productionGenerationReady":false,"verticalDatumStatus":"unknown",
        "sourceLicenseStatus":"unknown","sourceArchiveSha256":source_sha,
        "sourceArchiveBytes":preview["sourceArchiveBytes"],
        "sourceAttributionFromArchive":attribution,
        "sourceMetadataDeclaresTerrarium":preview["sourceMetadataDeclaresTerrarium"],
        "candidateHeightsSha256":candidate_sha,"selectionId":selection.selection_id,
        "bboxWgs84":selection.bbox,"zoom":zoom,"width":w,"height":h,
        "dxMeters":dx,"dyMeters":dy,"localRelativeReferenceMinMeters":local_min,
        "printWidthMm":120.,"printHeightMm":mesh.geometry.measured_aabb_mm[4],
        "baseThicknessMm":4.,"verticalExaggeration":1.,
        "candidateEncoding":"Terrarium interpretation - unverified",
        "uncertaintyMeters":Value::Null,"nativeResolutionMeters":Value::Null,
        "jobId":job,"operation":"experimental-local-3mf-export",
        "acknowledgementPolicyVersion":POLICY,"consentBindingSha256":consent_sha,
        "warning":"EXPERIMENTAL - USER-SUPPLIED TERRAIN. Source permissions not independently verified; vertical datum and native accuracy unknown. Private local evaluation only; third-party license and attribution requirements still apply. Not a certified elevation model. Redistribution not approved by Map2Model Next."
    });
    let artifact=export_experimental_core(&mesh,&provenance)
        .map_err(|e|format!("{}: {}",e.job_reason_code(),e.message))?;
    let st=fs::symlink_metadata(file).map_err(|_|"DATA_MISSING: input removed")?;
    if !st.is_file()||st.file_type().is_symlink()||st.len()>128*1024*1024 {
        return Err("INVALID_SOURCE: source changed".into());
    }
    let latest=fs::read(file).map_err(|_|"DATA_MISSING: source unreadable")?;
    if format!("{:x}",Sha256::digest(&latest))!=source_sha {
        return Err("HASH_MISMATCH: source changed before publication".into());
    }
    write_3mf_atomic(destination,&artifact,||false)
        .map_err(|e|format!("{}: {}",e.job_reason_code(),e.message))?;
    let published=fs::read(destination).map_err(|_|"EXPORT_FAILED: published file unreadable")?;
    if format!("{:x}",Sha256::digest(&published))!=artifact.package_sha256 {
        return Err("HASH_MISMATCH: published experimental 3MF changed".into());
    }
    Ok(json!({"status":"experimental","experimentalOnly":true,
        "productionGenerationReady":false,"verticalDatumStatus":"unknown",
        "redistributionApproved":false,"sourceArchiveSha256":source_sha,
        "candidateHeightsSha256":candidate_sha,"selectionId":selection.selection_id,
        "jobId":job,"sourceAttributionFromArchive":attribution,
        "canonicalGeometrySha256":artifact.canonical_geometry_sha256,
        "provenanceSha256":artifact.provenance_sha256,
        "packageSha256":artifact.package_sha256,"bytes":published.len(),
        "vertices":artifact.vertices,"triangles":artifact.triangles,
        "aabbMm":artifact.aabb_mm,"threeMfExported":true,"networkRequests":0}))
}
