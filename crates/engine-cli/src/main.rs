//! Initial engine health stub. Model generation is not implemented.
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match env::args().nth(1).as_deref() {
        Some("--health") => {
            let message = r#"{"apiVersion":"1.0.0","status":"ok","capabilities":[]}"#;
            println!("{message}");
            ExitCode::SUCCESS
        }
        Some("--version") => {
            println!("map2model-engine {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            let message = r#"{"apiVersion":"1.0.0","reasonCode":"UNSUPPORTED_OPERATION","message":"G0 stub only"}"#;
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn protocol_stub_is_not_a_model_generator() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.0.1");
    }
}
