//! Headless, non-networked G0 stdio protocol stub.
//! This is a protocol demonstration, not a working geographic or 3MF engine.
use serde_json::{json, Value};
use std::env;
use std::io::{self, BufRead};
use std::process::ExitCode;

const API_VERSION: &str = "1.0.0";
const MAX_MESSAGE_BYTES: usize = 1_048_576;

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
    match request.get("operation").and_then(Value::as_str) {
        Some("health") => json!({
            "apiVersion": API_VERSION, "requestId": request_id,
            "status": "ok", "capabilities": []
        }),
        Some(_) => error_response(
            Some(request_id),
            "UNSUPPORTED_OPERATION",
            "G0 engine stub does not implement model jobs",
        ),
        None => error_response(Some(request_id), "INVALID_REQUEST", "Missing operation"),
    }
}

fn error_response(request_id: Option<&str>, code: &str, message: &str) -> Value {
    json!({
        "apiVersion": API_VERSION,
        "requestId": request_id,
        "status": "error",
        "reasonCode": code,
        "message": message
    })
}

fn serve_stdio() -> ExitCode {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                let response = handle_line(&line);
                println!("{response}");
            }
            Err(_) => return ExitCode::from(1),
        }
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    match env::args().nth(1).as_deref() {
        Some("--stdio") => serve_stdio(),
        Some("--health") => {
            let message = json!({
                "apiVersion": API_VERSION, "status": "ok", "capabilities": []
            });
            println!("{message}");
            ExitCode::SUCCESS
        }
        Some("--version") => {
            println!("map2model-engine {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            let message = error_response(
                None,
                "UNSUPPORTED_OPERATION",
                "Expected --stdio or --health",
            );
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_does_not_claim_to_render_3mf() {
        let result =
            handle_line(r#"{"apiVersion":"1.0.0","requestId":"test-1","operation":"health"}"#);
        assert_eq!(result["status"], "ok");
        assert_eq!(result["capabilities"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn unknown_operation_reports_typed_error() {
        let result =
            handle_line(r#"{"apiVersion":"1.0.0","requestId":"test-2","operation":"generate"}"#);
        assert_eq!(result["reasonCode"], "UNSUPPORTED_OPERATION");
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
}
