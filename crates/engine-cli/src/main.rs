//! A1-04 native-only offline host. No renderer filesystem operations or
//! real-source generation are authorized by this stdio interface.
mod pmtiles_demo;
mod synthetic;

use serde_json::{json, Value};
use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

const API_VERSION: &str = "1.0.0";
const MAX_MESSAGE_BYTES: usize = 1_048_576;

fn error_response(request_id: Option<&str>, code: &str, message: &str) -> Value {
    json!({
        "apiVersion": API_VERSION,
        "requestId": request_id,
        "status": "error",
        "reasonCode": code,
        "message": message
    })
}

fn health(request_id: Option<&str>) -> Value {
    json!({
        "apiVersion": API_VERSION,
        "requestId": request_id,
        "status": "ok",
        "capabilities": ["syntheticMeshSmoke"],
        "productionGenerationReady": false,
        "threeMfExportEnabled": false
    })
}

fn synthetic_response(request_id: &str) -> Value {
    match synthetic::run() {
        Ok(result) => json!({
            "apiVersion": API_VERSION, "requestId": request_id,
            "status": "ok", "operation": "syntheticMeshSmoke", "result": result
        }),
        Err(reason) => {
            // Only registered G0 reason codes may cross the native IPC boundary.
            let code = reason.split(':').next().unwrap_or("");
            let safe_code = match code {
                "INVALID_REQUEST" | "INVALID_GRID_HASH" | "INVALID_GEOMETRY"
                | "INVALID_DIMENSIONS" | "DATA_MISSING" | "INCOMPLETE_GRID" | "OUT_OF_MEMORY"
                | "NON_MANIFOLD_MESH" | "UNSUPPORTED_DATUM" => code,
                _ => "INTERNAL_ERROR",
            };
            error_response(
                Some(request_id),
                safe_code,
                "Synthetic native mesh smoke failed",
            )
        }
    }
}

fn handle_line(line: &str) -> Value {
    if line.len() > MAX_MESSAGE_BYTES {
        return error_response(None, "INVALID_REQUEST", "Request exceeds G0 size budget");
    }
    let request: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return error_response(None, "INVALID_REQUEST", "Invalid JSON"),
    };
    let request_id = request
        .get("requestId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 128)
        .filter(|id| {
            id.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
        });
    let request_id = match request_id {
        Some(id) => id,
        None => return error_response(None, "INVALID_REQUEST", "Invalid requestId"),
    };
    if request.get("apiVersion").and_then(Value::as_str) != Some(API_VERSION) {
        return error_response(
            Some(request_id),
            "UNSUPPORTED_OPERATION",
            "Unsupported protocol version",
        );
    }
    // This phase deliberately permits no caller-supplied grid, paths or source URLs.
    let safe_keys = request.as_object().is_some_and(|map| {
        map.keys()
            .all(|key| matches!(key.as_str(), "apiVersion" | "requestId" | "operation"))
    });
    if !safe_keys {
        return error_response(
            Some(request_id),
            "INVALID_REQUEST",
            "Unsupported request fields",
        );
    }
    match request.get("operation").and_then(Value::as_str) {
        Some("health") => health(Some(request_id)),
        Some("syntheticMeshSmoke") => synthetic_response(request_id),
        Some(_) => error_response(
            Some(request_id),
            "UNSUPPORTED_OPERATION",
            "Operation not authorized by native A1-04 host",
        ),
        None => error_response(Some(request_id), "INVALID_REQUEST", "Missing operation"),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FrameError {
    Oversized,
    InvalidUtf8,
}

/// Read one newline-delimited frame without allocating more than MAX_MESSAGE_BYTES
/// for untrusted input. Drain oversized frames through the next newline so the
/// following valid request remains aligned. EOF without newline closes a frame.
fn next_frame<R: BufRead>(input: &mut R) -> io::Result<Option<Result<String, FrameError>>> {
    let mut data = Vec::new();
    let mut too_large = false;
    let mut seen = false;
    loop {
        let buffer = input.fill_buf()?;
        if buffer.is_empty() {
            if !seen {
                return Ok(None);
            }
            break;
        }
        seen = true;
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let length = newline.unwrap_or(buffer.len());
        if !too_large {
            if length > MAX_MESSAGE_BYTES.saturating_sub(data.len()) {
                too_large = true;
                data.clear();
            } else {
                data.extend_from_slice(&buffer[..length]);
            }
        }
        let consumed = length + usize::from(newline.is_some());
        input.consume(consumed);
        if newline.is_some() {
            break;
        }
    }
    if too_large {
        return Ok(Some(Err(FrameError::Oversized)));
    }
    match String::from_utf8(data) {
        Ok(line) => Ok(Some(Ok(line))),
        Err(_) => Ok(Some(Err(FrameError::InvalidUtf8))),
    }
}

fn serve<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> io::Result<()> {
    while let Some(frame) = next_frame(input)? {
        let result = match frame {
            Ok(line) => handle_line(&line),
            Err(FrameError::Oversized) => {
                error_response(None, "INVALID_REQUEST", "Request exceeds G0 size budget")
            }
            Err(FrameError::InvalidUtf8) => {
                error_response(None, "INVALID_REQUEST", "Request is not UTF-8")
            }
        };
        writeln!(output, "{result}")?;
        output.flush()?;
    }
    Ok(())
}

fn serve_stdio() -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    match serve(&mut stdin.lock(), &mut stdout.lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}

fn main() -> ExitCode {
    match env::args().nth(1).as_deref() {
        Some("--stdio") => serve_stdio(),
        Some("--health") => {
            println!("{}", health(None));
            ExitCode::SUCCESS
        }
        Some("--export-original-pmtiles") => {
            // Only Electron main supplies the fixed bundled original synthetic
            // source path and the OS Save dialog-derived destination.
            let args = env::args().skip(2).collect::<Vec<_>>();
            let response = match args.as_slice() {
                [source_dir, destination] => match pmtiles_demo::export_fixture(
                    std::path::Path::new(source_dir),
                    std::path::Path::new(destination),
                ) {
                    Ok(result) => json!({"status":"ok", "result":result}),
                    Err(reason) => {
                        let code = reason.split(':').next().unwrap_or("");
                        let safe = match code {
                            "INVALID_REQUEST" | "INVALID_SOURCE" | "HASH_MISMATCH"
                            | "DATA_MISSING" | "UNSUPPORTED_DATUM" | "INVALID_SELECTION"
                            | "INVALID_DIMENSIONS" | "INVALID_GEOMETRY" | "INVALID_GRID_HASH"
                            | "INCOMPLETE_GRID" | "NON_MANIFOLD_MESH" | "OUT_OF_MEMORY"
                            | "EXPORT_FAILED" => code,
                            _ => "INTERNAL_ERROR",
                        };
                        error_response(None, safe, "Original synthetic PMTiles export failed")
                    }
                },
                _ => error_response(
                    None,
                    "INVALID_REQUEST",
                    "Expected synthetic fixture root and native output path",
                ),
            };
            let success = response["status"] == "ok";
            println!("{response}");
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Some("--export-synthetic") => {
            // This internal command is launched only by Electron main after its
            // native Save dialog; no renderer may provide an arbitrary path.
            let destination = env::args().nth(2);
            let response = match destination {
                Some(destination) => match synthetic::export_to(std::path::Path::new(&destination))
                {
                    Ok(result) => json!({"status":"ok","result":result}),
                    Err(reason) => {
                        let code = reason.split(':').next().unwrap_or("");
                        let safe = match code {
                            "INVALID_REQUEST" | "HASH_MISMATCH" | "DATA_MISSING"
                            | "INVALID_GEOMETRY" | "INVALID_DIMENSIONS" | "EXPORT_FAILED"
                            | "OUT_OF_MEMORY" | "UNSUPPORTED_DATUM" | "INCOMPLETE_GRID" => code,
                            _ => "INTERNAL_ERROR",
                        };
                        error_response(None, safe, "Synthetic 3MF native export failed")
                    }
                },
                None => error_response(
                    None,
                    "INVALID_REQUEST",
                    "Missing approved output destination",
                ),
            };
            let success = response["status"] == "ok";
            println!("{response}");
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Some("--smoke-mesh") => {
            let outcome = synthetic_response("smoke-1");
            let success = outcome["status"] == "ok";
            println!("{outcome}");
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Some("--version") => {
            println!("map2model-engine {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            let result = error_response(
                None,
                "UNSUPPORTED_OPERATION",
                "Expected --stdio, --health, --smoke-mesh or --export-synthetic",
            );
            eprintln!("{result}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn health_reports_only_real_synthetic_capability() {
        let result =
            handle_line(r#"{"apiVersion":"1.0.0","requestId":"test-1","operation":"health"}"#);
        assert_eq!(result["status"], "ok");
        assert_eq!(result["capabilities"], json!(["syntheticMeshSmoke"]));
        assert_eq!(result["productionGenerationReady"], false);
        assert_eq!(result["threeMfExportEnabled"], false);
    }

    #[test]
    fn synthetic_rpc_reaches_real_mesh_and_never_claims_3mf() {
        let result = handle_line(
            r#"{"apiVersion":"1.0.0","requestId":"smoke-123","operation":"syntheticMeshSmoke"}"#,
        );
        assert_eq!(result["status"], "ok");
        assert_eq!(result["requestId"], "smoke-123");
        assert_eq!(result["result"]["vertices"], 24);
        assert_eq!(result["result"]["triangles"], 44);
        assert_eq!(result["result"]["meshVerified"], true);
        assert_eq!(result["result"]["threeMfExported"], false);
    }

    #[test]
    fn unknown_ops_and_untrusted_fields_are_rejected() {
        let result =
            handle_line(r#"{"apiVersion":"1.0.0","requestId":"test-2","operation":"generate"}"#);
        assert_eq!(result["reasonCode"], "UNSUPPORTED_OPERATION");
        let result = handle_line(
            r#"{"apiVersion":"1.0.0","requestId":"test-3","operation":"syntheticMeshSmoke","path":"C:/secret"}"#,
        );
        assert_eq!(result["reasonCode"], "INVALID_REQUEST");
    }

    #[test]
    fn invalid_requests_do_not_echo_data() {
        assert_eq!(handle_line("not json")["reasonCode"], "INVALID_REQUEST");
        let result = handle_line(
            r#"{"apiVersion":"1.0.0","requestId":"../../../secret","operation":"health"}"#,
        );
        assert_eq!(result["reasonCode"], "INVALID_REQUEST");
        assert_eq!(result["requestId"], Value::Null);
    }

    #[test]
    fn huge_untrusted_frame_is_bounded_and_subsequent_rpc_survives() {
        let health = br#"{"apiVersion":"1.0.0","requestId":"after-big","operation":"health"}"#;
        let mut wire = vec![b'x'; MAX_MESSAGE_BYTES * 3];
        wire.push(b'\n');
        wire.extend_from_slice(health);
        wire.push(b'\n');
        let mut input = Cursor::new(wire);
        let mut output = Vec::new();
        serve(&mut input, &mut output).unwrap();
        let results = String::from_utf8(output).unwrap();
        let mut lines = results
            .lines()
            .map(|s| serde_json::from_str::<Value>(s).unwrap());
        assert_eq!(lines.next().unwrap()["reasonCode"], "INVALID_REQUEST");
        assert_eq!(lines.next().unwrap()["requestId"], "after-big");
        assert!(lines.next().is_none());
    }

    #[test]
    fn invalid_utf8_does_not_break_framing() {
        let mut wire = vec![0xff, b'\n'];
        wire.extend_from_slice(
            br#"{"apiVersion":"1.0.0","requestId":"next","operation":"health"}"#,
        );
        let mut input = Cursor::new(wire);
        let mut output = Vec::new();
        serve(&mut input, &mut output).unwrap();
        let results = String::from_utf8(output).unwrap();
        let mut lines = results
            .lines()
            .map(|s| serde_json::from_str::<Value>(s).unwrap());
        assert_eq!(lines.next().unwrap()["reasonCode"], "INVALID_REQUEST");
        assert_eq!(lines.next().unwrap()["requestId"], "next");
        assert!(lines.next().is_none());
    }
}
