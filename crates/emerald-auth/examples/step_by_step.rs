//! Demonstrates the lower-level API by executing each authentication step manually.
//!
//! This is useful if you need fine-grained control over the auth chain, custom
//! error handling per step, or want to cache intermediate tokens.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example step_by_step -- <CLIENT_ID>
//! ```

use std::time::Duration;

use emerald_auth::callback::CallbackServer;
use emerald_auth::{microsoft, minecraft, xbox};
use secrecy::ExposeSecret;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client_id = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example step_by_step -- <CLIENT_ID>");
        std::process::exit(1);
    });

    let http = reqwest::Client::new();

    // Step 1: Start the callback server.
    println!("[1/7] Starting callback server...");
    let (server, redirect_uri) = CallbackServer::bind().await?;
    println!("       Listening at {redirect_uri}");

    // Step 2: Build the auth URL and open the browser.
    let state = "my-csrf-state-token";
    let auth_url = microsoft::build_auth_url(&client_id, &redirect_uri, state);
    println!("[2/7] Opening browser for login...");
    println!("       URL: {auth_url}");
    open::that(&auth_url)?;

    // Step 3: Wait for the callback.
    println!("[3/7] Waiting for callback (60s timeout)...");
    let (code, received_state) = server
        .wait_for_callback(Duration::from_secs(60))
        .await?;
    assert_eq!(state, received_state, "CSRF state mismatch!");
    println!("       Received authorization code: {}...", &code[..code.len().min(20)]);

    // Step 4: Exchange the code for Microsoft tokens.
    println!("[4/7] Exchanging code for Microsoft tokens...");
    let ms_tokens = microsoft::exchange_code(&http, &client_id, &code, &redirect_uri).await?;
    println!("       Microsoft access token received (expires in {}s)", ms_tokens.expires_in);

    // Step 5: Xbox Live + XSTS.
    println!("[5/7] Authenticating with Xbox Live...");
    let xbox_token = xbox::authenticate_xbox_live(&http, &ms_tokens.access_token).await?;
    println!("       Xbox user hash: {}", xbox_token.user_hash);

    println!("[6/7] Obtaining XSTS token...");
    let xsts_token = xbox::authenticate_xsts(&http, &xbox_token.token).await?;
    println!("       XSTS user hash: {}", xsts_token.user_hash);

    // Step 6: Minecraft authentication.
    println!("[7/7] Authenticating with Minecraft services...");
    let mc_token = minecraft::authenticate_minecraft(&http, &xsts_token.token, &xsts_token.user_hash).await?;
    println!("       Minecraft token received (expires in {}s)", mc_token.expires_in);

    // Step 7: Check ownership and fetch profile.
    let owns = minecraft::check_ownership(&http, &mc_token.access_token).await?;
    if !owns {
        eprintln!("ERROR: This account does not own Minecraft!");
        std::process::exit(1);
    }
    println!("       Game ownership verified.");

    let profile = minecraft::fetch_profile(&http, &mc_token.access_token).await?;
    println!("\n=== Minecraft Profile ===");
    println!("Username: {}", profile.name);
    println!("UUID:     {}", profile.id);
    println!("Skins:    {}", profile.skins.len());
    println!("Capes:    {}", profile.capes.len());
    println!("Token:    {}...", &mc_token.access_token.expose_secret()[..20]);

    Ok(())
}
