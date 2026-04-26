//! Re-authenticate using a previously stored refresh token.
//!
//! This skips the browser login entirely and directly exchanges the refresh
//! token for new Microsoft → Xbox → Minecraft tokens.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example refresh -- <CLIENT_ID> <REFRESH_TOKEN>
//! ```
//!
//! You can get a refresh token from the `authenticate` example.

use emerald_auth::MicrosoftAuth;
use secrecy::{ExposeSecret, SecretString};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cargo run --example refresh -- <CLIENT_ID> <REFRESH_TOKEN>");
        std::process::exit(1);
    }

    let client_id = &args[1];
    let refresh_token = SecretString::from(args[2].clone());

    println!("Refreshing authentication (no browser needed)...\n");

    let auth = MicrosoftAuth::new(client_id);
    let profile = auth.refresh(&refresh_token).await?;

    println!("=== Refresh Successful ===");
    println!("Username:      {}", profile.username);
    println!("UUID:          {}", profile.uuid);
    println!("Access token:  {}...", &profile.access_token.expose_secret()[..20]);

    Ok(())
}
