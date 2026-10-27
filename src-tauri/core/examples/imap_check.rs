//! Diagnostic only. Never place credentials in source or command arguments.
use arus_core::{imap, models::ConnectionInput};
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let email = std::env::var("ARUS_IMAP_EMAIL").unwrap_or_default();
    let password = std::env::var("ARUS_IMAP_PASSWORD").unwrap_or_default();
    let input = ConnectionInput {
        email,
        password,
        provider: "Auto".into(),
        host: None,
        port: None,
    };
    match imap::test(input).await {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("JSON")),
        Err(e) => {
            eprintln!("{}: {}", e.code, e.message);
            std::process::exit(1)
        }
    }
}

