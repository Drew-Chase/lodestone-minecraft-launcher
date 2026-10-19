//! Interactive device-code login — a live-network smoke test, run manually.
//!
//! Usage: `cargo run -p lodestone_minecraft_authentication --example device_login -- <azure-app-client-id>`
//!
//! Prints the verification url + code, waits for the sign-in to complete, and
//! prints the resolved profile and session expiry.

use lodestone_minecraft_authentication::Authentication;

#[tokio::main]
async fn main() {
    let Some(client_id) = std::env::args().nth(1) else {
        eprintln!("usage: device_login <azure-app-client-id>");
        std::process::exit(2);
    };

    let auth = Authentication::new(client_id);

    let start = match auth.begin_device_code_login().await {
        Ok(start) => start,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    };
    println!("open {} and enter: {}", start.verification_uri, start.user_code);
    println!("(the code expires in {} seconds — waiting...)", start.expires_in);

    let session = match auth.finish_device_code_login(start).await {
        Ok(session) => session,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    };

    println!("signed in as {} ({})", session.profile.name, session.profile.id);
    println!("token expires at unix {}", session.expires_at);
    if session.refresh_token.is_some() {
        println!("refresh token: issued (store it for silent re-login)");
    }
}
