//! Full Microsoft → Minecraft authentication flow.
//!
//! Opens the default browser for Microsoft login, waits for the redirect,
//! and prints the authenticated Minecraft profile.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example authenticate -- <YOUR_AZURE_CLIENT_ID>
//! ```
//!
//! You can register an Azure app at https://portal.azure.com:
//! 1. Go to Azure Active Directory → App registrations → New registration
//! 2. Set redirect URI to "http://localhost" (Mobile and desktop applications)
//! 3. Copy the Application (client) ID

use emerald_auth::MicrosoftAuth;
use secrecy::ExposeSecret;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client_id = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example authenticate -- <CLIENT_ID>");
        std::process::exit(1);
    });

    println!("Starting Microsoft authentication...");
    println!("A browser window will open. Please sign in with your Microsoft account.\n");

    let auth = MicrosoftAuth::new(&client_id);
    let profile = auth.authenticate().await?;

    println!("=== Authentication Successful ===");
    println!("Username:      {}", profile.username);
    println!("UUID:          {}", profile.uuid);
    println!("Access token:  {}...", &profile.access_token.expose_secret()[..20]);

    if let Some(skin) = &profile.skin {
        println!("Skin:          {} ({:?})", skin.url, skin.variant);
    } else {
        println!("Skin:          (default)");
    }

    if let Some(cape) = &profile.cape {
        println!("Cape:          {} ({})", cape.alias, cape.url);
    } else {
        println!("Cape:          (none)");
    }

    if profile.refresh_token.is_some() {
        println!("\nRefresh token received — you can use it to re-authenticate without the browser.");
        println!("See the `refresh` example.");
    }

    Ok(())
}
