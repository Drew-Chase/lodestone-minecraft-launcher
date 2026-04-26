//! Authenticate using a fixed callback port.
//!
//! Some Azure app registrations require a specific redirect URI with a known
//! port. This example shows how to pin the callback server to a fixed port.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example custom_port -- <CLIENT_ID> [PORT]
//! ```

use std::time::Duration;

use emerald_auth::MicrosoftAuth;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let client_id = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example custom_port -- <CLIENT_ID> [PORT]");
        std::process::exit(1);
    });
    let port: u16 = args
        .get(2)
        .and_then(|p| p.parse().ok())
        .unwrap_or(25585);

    println!("Using fixed callback port: {port}");
    println!("Make sure your Azure app has http://localhost as a redirect URI.\n");

    let auth = MicrosoftAuth::new(client_id)
        .with_port(port)
        .with_timeout(Duration::from_secs(120));

    let profile = auth.authenticate().await?;

    println!("Logged in as {} ({})", profile.username, profile.uuid);

    Ok(())
}
