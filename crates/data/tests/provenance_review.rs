//! Developer provenance worksheet guards, NEVER a production authorization
//! verifier. These tests use only text evidence and DO NOT load or distribute
//! owner-provided PMTiles, signatures or elevation rasters.
use serde_json::Value;

const ORIGINAL_ARCHIVE_SHA: &str =
    "40f452bd2449ac0b119efd1cabfcd1a0937d375865e4795810bf82d4743ad5ce";
const CORRECTED_ARCHIVE_SHA: &str =
    "75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0";

fn report() -> Value {
    serde_json::from_str(include_str!("../provenance/interlaken-z17-review.v1.json"))
        .expect("review provenance ledger is valid JSON")
}

fn rejected_as_production_authorization(report: &Value) -> bool {
    // The memo is an *advisory* research worksheet, not a trusted signed source
    // record. Do not permit it to become an accidental production permission.
    let gates = &report["provenanceGates"];
    report["kind"] == "UnverifiedTerrainProvenanceReview"
        && report["advisoryOnly"] == true
        && report["privateSourceBytesIncluded"] == false
        && gates["parentReleaseCryptographicallyPinned"] == false
        && gates["regionConstituentSourceSetVerified"] == false
        && gates["sourceLicenseVerified"] == false
        && gates["sourceAttributionVerified"] == false
        && gates["nativeResolutionVerified"] == false
        && gates["verticalDatumStatus"] == "unknown"
        && gates["verticalDatumVerified"] == false
        && gates["exactArchiveEncodingAttestationSignatureVerified"] == false
        && gates["redistributionApproved"] == false
        && gates["productionGenerationReady"] == false
        && report["officialPublisherEncodingDocumentation"]["exactArchiveEncodingAttested"] == false
        && report["candidateContributor"]["regionSpecificContributionVerified"] == false
}

#[test]
fn published_encoding_convention_is_not_production_permission() {
    let memo = report();
    assert!(rejected_as_production_authorization(&memo));
    assert_eq!(memo["schemaVersion"], "review-1");
    assert_eq!(memo["archiveSha256"], CORRECTED_ARCHIVE_SHA);
    assert_eq!(
        memo["earlierMalformedHeaderArchiveSha256"],
        ORIGINAL_ARCHIVE_SHA
    );
    assert_ne!(ORIGINAL_ARCHIVE_SHA, CORRECTED_ARCHIVE_SHA);
    assert_eq!(memo["archiveBytes"], 3_127_295);
    assert_eq!(
        memo["regionBboxWgs84"],
        serde_json::json!([7.88, 46.68, 7.89, 46.69])
    );
    assert_eq!(
        memo["observedTileZoomCounts"],
        serde_json::json!({
            "13": 1, "14": 2, "15": 4, "16": 9, "17": 24
        })
    );
    assert_eq!(memo["webpTileSizePixels"], 512);
    assert_eq!(
        memo["officialPublisherEncodingDocumentation"]["publisherRepositoryCommitSha"],
        "775663e9ca58c7fe741dbd53fbe4461cb73a9bb9"
    );
    assert_eq!(
        memo["officialPublisherEncodingDocumentation"]["declaredRgbEncoding"],
        "terrarium"
    );
    assert_eq!(memo["candidateContributor"]["catalogEntry"], "swissalti3d");
    assert_eq!(
        memo["candidateContributor"]["swisstopoPublishedVerticalSystem"],
        "LV95 / LN02"
    );
    assert_eq!(
        memo["nextApplicationTest"],
        "Lead A1-04 integrated Windows preview, not standalone source inspector"
    );
}

#[test]
fn tampering_with_review_ledger_approval_or_datum_must_be_detected() {
    let memo = report();
    for (section, key, value) in [
        (
            "provenanceGates",
            "productionGenerationReady",
            serde_json::json!(true),
        ),
        (
            "provenanceGates",
            "redistributionApproved",
            serde_json::json!(true),
        ),
        (
            "provenanceGates",
            "verticalDatumVerified",
            serde_json::json!(true),
        ),
        (
            "provenanceGates",
            "verticalDatumStatus",
            serde_json::json!("known"),
        ),
        (
            "provenanceGates",
            "sourceLicenseVerified",
            serde_json::json!(true),
        ),
        (
            "provenanceGates",
            "nativeResolutionVerified",
            serde_json::json!(true),
        ),
        (
            "provenanceGates",
            "exactArchiveEncodingAttestationSignatureVerified",
            serde_json::json!(true),
        ),
        (
            "officialPublisherEncodingDocumentation",
            "exactArchiveEncodingAttested",
            serde_json::json!(true),
        ),
        (
            "candidateContributor",
            "regionSpecificContributionVerified",
            serde_json::json!(true),
        ),
    ] {
        let mut forged = memo.clone();
        forged[section][key] = value;
        assert!(
            !rejected_as_production_authorization(&forged),
            "research worksheet was silently upgraded for {section}.{key}"
        );
    }
}
