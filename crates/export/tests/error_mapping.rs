use map2model_export::ExportError;

#[test]
fn native_job_reason_codes_are_exact_frozen_jobstate_v1_variants() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../packages/contracts/schema/v1/JobStateV1.schema.json"
    ))
    .unwrap();
    let valid = schema["properties"]["reasonCode"]["anyOf"][1]["enum"]
        .as_array()
        .unwrap();
    let codes = [
        "CANCELLED",
        "DATA_MISSING",
        "INVALID_REQUEST",
        "INVALID_SELECTION",
        "INVALID_CRS",
        "UNSUPPORTED_DATUM",
        "HASH_MISMATCH",
        "INVALID_SOURCE",
        "OUT_OF_MEMORY",
        "NON_MANIFOLD_MESH",
        "INTERNAL_ERROR",
        "INCOMPLETE_GRID",
        "INVALID_GRID_HASH",
        "INVALID_GEOMETRY",
        "INVALID_DIMENSIONS",
        "NONFINITE_ELEVATION",
        "INVALID_EXPORT",
        "UNKNOWN_PRIVATE_ERROR",
    ];
    for internal_code in codes {
        let error = ExportError::new(internal_code, "test");
        let public_code = error.job_reason_code();
        assert!(
            valid.iter().any(|x| x.as_str() == Some(public_code)),
            "{internal_code} wrongly escapes into frozen JobStateV1 as {public_code}"
        );
        assert_eq!(public_code, if internal_code == "INVALID_EXPORT" || internal_code ==
            "UNKNOWN_PRIVATE_ERROR" { "EXPORT_FAILED" } else { internal_code });
    }
}
