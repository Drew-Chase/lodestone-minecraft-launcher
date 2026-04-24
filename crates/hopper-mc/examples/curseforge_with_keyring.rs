//! Secure CurseForge API-key flow using the OS credential store.
//!
//! # Why this pattern
//!
//! CurseForge keys are *developer* secrets that ship inside end-user
//! applications — there is no anonymous mode, so every installation of
//! your app needs a working key. Two anti-patterns to avoid:
//!
//! - **Environment variables** are readable by any process running under
//!   the same user account. A compromised unrelated process (or a child
//!   process of your app) can harvest `$CURSEFORGE_API_KEY` without any
//!   prompt or escalation.
//! - **Plain `String` keys** linger in freed memory until the allocator
//!   happens to reuse the slot. They can be recovered from core dumps,
//!   heap profiler snapshots, or Windows memory dumps.
//!
//! This example addresses both:
//!
//! 1. The key lives in the **OS credential store** —
//!    Windows Credential Manager on Windows,
//!    Keychain on macOS,
//!    the Secret Service (GNOME Keyring / KWallet) on Linux —
//!    accessed through the [`keyring`] crate. Credentials in that store
//!    are encrypted by the OS and scoped to the logged-in account.
//! 2. In memory the key is a [`secrecy::SecretString`], which **zeros
//!    its backing allocation on drop** and refuses to appear in
//!    `Debug`/`Display` output. The plaintext is exposed for exactly
//!    one call — building the outbound HTTPS request.
//! 3. On first run we prompt for the key with [`rpassword`], which
//!    disables terminal echo so the key does not land in shell history
//!    or scroll-back.
//!
//! What this example does **not** protect against: a debugger attached
//! to the live process, a user with root / SYSTEM privileges, or a
//! compromised OS credential store itself. Those are out of scope for
//! anything short of a remote-proxy architecture.
//!
//! # Run
//!
//! ```sh
//! cargo run --example curseforge_with_keyring
//! ```
//!
//! First run: prompts you for a key, stores it in the OS credential
//! manager, then performs a sample search.
//! Subsequent runs: silently fetches the key and performs the search.
//!
//! To *remove* the stored key, delete the entry
//! `service="hopper-mc", user="curseforge-api-key"` from your OS
//! credential manager's UI.

use std::io::{self, Write};

use hopper_mc::{CurseForgeProvider, ModProvider, Sort};
use keyring::Entry;
use secrecy::SecretString;

/// Namespace the entry in the OS credential store. Using the crate name
/// as the service avoids colliding with other tools' keys.
const KEYRING_SERVICE: &str = "hopper-mc";
const KEYRING_USERNAME: &str = "curseforge-api-key";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = load_or_prompt_api_key()?;

    // Hand the key off to the provider. From this point on the
    // plaintext lives inside `cf`'s `SecretString`; when `cf` is dropped
    // the allocation is zeroed.
    let cf = CurseForgeProvider::new_with_secret_key(Some(key));

    println!("\nSearching CurseForge for \"sodium\"...\n");
    let hits = cf.find_mods(Some("sodium"), Sort::Relevance, 0, 5).await?;

    for m in hits {
        println!(
            "  {:<32} downloads={:>10}  loaders={:?}",
            truncate(&m.base.title, 32),
            m.base.downloads,
            m.loaders,
        );
    }

    Ok(())
}

/// Look up the API key in the OS credential store. On first run, prompt
/// for it and persist it for next time.
fn load_or_prompt_api_key() -> Result<SecretString, Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)?;

    match entry.get_password() {
        Ok(existing) => {
            // `From<String> for SecretString` moves the allocation into
            // the secret — no extra copy. The old `existing` binding is
            // consumed by the conversion.
            Ok(SecretString::from(existing))
        }
        Err(keyring::Error::NoEntry) => {
            println!(
                "No CurseForge API key is stored for this user yet.\n\
                 Get one from https://console.curseforge.com/ (developer console),\n\
                 then paste it below. It will be saved securely in your OS\n\
                 credential manager — not in an env var, not in a config file."
            );
            println!();
            io::stdout().flush().ok();

            // `rpassword` reads a line from the terminal without
            // echoing it. The returned `String` owns a single freshly
            // allocated buffer.
            let entered = rpassword::prompt_password("CurseForge API key: ")?;
            let trimmed = entered.trim();
            if trimmed.is_empty() {
                return Err("empty key".into());
            }

            // Persist to the OS credential store. `set_password` copies
            // the bytes across the platform IPC boundary; we can't
            // prevent that copy, but the backing store encrypts what
            // lands on disk.
            entry.set_password(trimmed)?;
            println!(
                "\nStored in the OS credential manager under service=\"{KEYRING_SERVICE}\", \
                 user=\"{KEYRING_USERNAME}\"."
            );

            // Move the plaintext `String` directly into a `SecretString`
            // — zero-copy (hands off the allocation) — so it gets
            // zeroed when the secret drops.
            Ok(SecretString::from(entered))
        }
        Err(e) => Err(Box::new(e)),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
