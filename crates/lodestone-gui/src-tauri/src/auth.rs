use std::sync::Mutex;

use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tauri::Manager;

use emerald_auth::MicrosoftAuth;

use crate::instances::{ensure_manager, InstanceManagerState};

const CLIENT_ID: &str = "b1156b7e-e0fe-4f86-a835-afed728d093d";

// ---------------------------------------------------------------------------
// Types exposed to the frontend
// ---------------------------------------------------------------------------

/// The user session sent to the frontend via Tauri commands.
/// Tagged union so the frontend can discriminate on `mode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum UserSession {
    #[serde(rename_all = "camelCase")]
    Microsoft {
        uuid: String,
        username: String,
        skin_url: Option<String>,
        cape_url: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Offline {
        uuid: String,
        username: String,
    },
    #[serde(rename_all = "camelCase")]
    Demo {
        uuid: String,
        username: String,
    },
}

// ---------------------------------------------------------------------------
// Internal state (managed by Tauri)
// ---------------------------------------------------------------------------

pub struct AuthInner {
    pub session: Option<UserSession>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

pub type AuthState = Mutex<AuthInner>;

impl AuthInner {
    pub fn new() -> Self {
        Self {
            session: None,
            access_token: None,
            refresh_token: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy JSON migration (auth.json → DB)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
enum PersistedAuth {
    #[serde(rename_all = "camelCase")]
    Microsoft { refresh_token: String },
    #[serde(rename_all = "camelCase")]
    Offline { username: String },
    Demo,
}

/// Migrate auth.json into the database if it exists, then delete the file.
async fn migrate_auth_json(
    mgr_state: &InstanceManagerState,
    app: &tauri::AppHandle,
) {
    let dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    let path = dir.join("auth.json");
    if !path.exists() {
        return;
    }

    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return,
    };
    let persisted: PersistedAuth = match serde_json::from_str(&data) {
        Ok(p) => p,
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            return;
        }
    };

    let guard = mgr_state.lock().await;
    let mgr = match guard.as_ref() {
        Some(m) => m,
        None => return,
    };

    match persisted {
        PersistedAuth::Microsoft { refresh_token } => {
            // We don't have the full profile, just store the refresh token
            // with a placeholder — it will be refreshed on restore_session
            let _ = mgr
                .add_account("microsoft", "pending-migration", "Microsoft User",
                             Some(&refresh_token), None, None)
                .await;
            if let Ok(Some(acc)) = mgr.get_active_account().await {
                let _ = mgr.set_active_account(acc.id).await;
            } else {
                // Set the newly created account as active
                let accounts = mgr.list_accounts().await.unwrap_or_default();
                if let Some(acc) = accounts.first() {
                    let _ = mgr.set_active_account(acc.id).await;
                }
            }
        }
        PersistedAuth::Offline { username } => {
            let uuid = offline_uuid(&username);
            let _ = mgr.add_account("offline", &uuid, &username, None, None, None).await;
            let accounts = mgr.list_accounts().await.unwrap_or_default();
            if let Some(acc) = accounts.first() {
                let _ = mgr.set_active_account(acc.id).await;
            }
        }
        PersistedAuth::Demo => {
            let uuid = offline_uuid("Steve");
            let _ = mgr.add_account("demo", &uuid, "Steve", None, None, None).await;
            let accounts = mgr.list_accounts().await.unwrap_or_default();
            if let Some(acc) = accounts.first() {
                let _ = mgr.set_active_account(acc.id).await;
            }
        }
    }

    // Delete the old auth.json
    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// Offline UUID (matches Java's UUID.nameUUIDFromBytes)
// ---------------------------------------------------------------------------

fn offline_uuid(username: &str) -> String {
    let input = format!("OfflinePlayer:{username}");
    let mut hash = md5::compute(input.as_bytes()).0;
    hash[6] = (hash[6] & 0x0f) | 0x30; // version 3
    hash[8] = (hash[8] & 0x3f) | 0x80; // variant 2
    hex::encode(&hash)
}

/// Encode bytes as lowercase hex.
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push(HEX[(*b >> 4) as usize]);
            s.push(HEX[(*b & 0x0f) as usize]);
        }
        s
    }

    const HEX: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7',
        '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
}

// ---------------------------------------------------------------------------
// Microsoft error formatting
// ---------------------------------------------------------------------------

fn format_auth_error(e: emerald_auth::AuthError) -> String {
    match &e {
        emerald_auth::AuthError::Minecraft(msg) if msg.contains("Invalid app registration") => {
            "Microsoft sign-in succeeded, but Minecraft Services rejected the app registration. \
             This is a known issue pending Xbox Developer Program approval. \
             Please use Offline or Demo mode in the meantime."
                .to_string()
        }
        emerald_auth::AuthError::NoGameOwnership => {
            "This Microsoft account does not own Minecraft Java Edition.".to_string()
        }
        emerald_auth::AuthError::Timeout(_) => {
            "Authentication timed out. Please try again.".to_string()
        }
        emerald_auth::AuthError::BrowserOpen(msg) => {
            format!("Failed to open the browser: {msg}")
        }
        _ => e.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn login_microsoft(
    state: tauri::State<'_, AuthState>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<UserSession, String> {
    let auth = MicrosoftAuth::new(CLIENT_ID);
    let profile = auth.authenticate().await.map_err(format_auth_error)?;

    let session = UserSession::Microsoft {
        uuid: profile.uuid.clone(),
        username: profile.username.clone(),
        skin_url: profile.skin.as_ref().map(|s| s.url.clone()),
        cape_url: profile.cape.as_ref().map(|c| c.url.clone()),
    };

    let access_token = profile.access_token.expose_secret().to_owned();
    let refresh_token = profile.refresh_token.as_ref().map(|t| t.expose_secret().to_owned());

    // Save to database
    ensure_manager(&mgr_state, &app).await?;
    {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        let acc_id = mgr.add_account(
            "microsoft",
            &profile.uuid,
            &profile.username,
            refresh_token.as_deref(),
            profile.skin.as_ref().map(|s| s.url.as_str()),
            profile.cape.as_ref().map(|c| c.url.as_str()),
        ).await.map_err(|e| format!("failed to save account: {e}"))?;
        mgr.set_active_account(acc_id)
            .await
            .map_err(|e| format!("failed to activate account: {e}"))?;
    }

    let mut inner = state.lock().unwrap();
    inner.session = Some(session.clone());
    inner.access_token = Some(access_token);
    inner.refresh_token = refresh_token;

    Ok(session)
}

#[tauri::command]
pub async fn login_offline(
    username: String,
    state: tauri::State<'_, AuthState>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<UserSession, String> {
    // Validate username: 1-16 chars, alphanumeric + underscore.
    let username = username.trim().to_string();
    if username.is_empty() || username.len() > 16 {
        return Err("Username must be 1–16 characters.".to_string());
    }
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Username may only contain letters, numbers, and underscores.".to_string());
    }

    let uuid = offline_uuid(&username);
    let session = UserSession::Offline {
        uuid: uuid.clone(),
        username: username.clone(),
    };

    // Save to database
    ensure_manager(&mgr_state, &app).await?;
    {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        let acc_id = mgr.add_account("offline", &uuid, &username, None, None, None)
            .await.map_err(|e| format!("failed to save account: {e}"))?;
        mgr.set_active_account(acc_id)
            .await.map_err(|e| format!("failed to activate account: {e}"))?;
    }

    let mut inner = state.lock().unwrap();
    inner.session = Some(session.clone());
    inner.access_token = None;
    inner.refresh_token = None;

    Ok(session)
}

#[tauri::command]
pub async fn login_demo(
    state: tauri::State<'_, AuthState>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<UserSession, String> {
    let username = "Steve".to_string();
    let uuid = offline_uuid(&username);
    let session = UserSession::Demo { uuid: uuid.clone(), username: username.clone() };

    // Save to database
    ensure_manager(&mgr_state, &app).await?;
    {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        let acc_id = mgr.add_account("demo", &uuid, &username, None, None, None)
            .await.map_err(|e| format!("failed to save account: {e}"))?;
        mgr.set_active_account(acc_id)
            .await.map_err(|e| format!("failed to activate account: {e}"))?;
    }

    let mut inner = state.lock().unwrap();
    inner.session = Some(session.clone());
    inner.access_token = None;
    inner.refresh_token = None;

    Ok(session)
}

#[tauri::command]
pub async fn get_session(
    state: tauri::State<'_, AuthState>,
) -> Result<Option<UserSession>, String> {
    let inner = state.lock().unwrap();
    Ok(inner.session.clone())
}

#[tauri::command]
pub async fn logout(
    state: tauri::State<'_, AuthState>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Deactivate all accounts (keep them in DB for re-login)
    ensure_manager(&mgr_state, &app).await?;
    {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        let _ = mgr.deactivate_all_accounts().await;
    }

    let mut inner = state.lock().unwrap();
    inner.session = None;
    inner.access_token = None;
    inner.refresh_token = None;
    Ok(())
}

#[tauri::command]
pub async fn restore_session(
    state: tauri::State<'_, AuthState>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Option<UserSession>, String> {
    ensure_manager(&mgr_state, &app).await?;

    // Migrate legacy auth.json if present
    migrate_auth_json(&mgr_state, &app).await;

    let account = {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        mgr.get_active_account()
            .await
            .map_err(|e| format!("failed to get active account: {e}"))?
    };

    let account = match account {
        Some(a) => a,
        None => return Ok(None),
    };

    let result = match account.mode.as_str() {
        "microsoft" => {
            let refresh_token = match &account.refresh_token {
                Some(rt) if !rt.is_empty() => rt.clone(),
                _ => return Ok(None),
            };

            log::info!("restoring Microsoft session via token refresh");
            let secret = secrecy::SecretString::from(refresh_token);
            let auth = MicrosoftAuth::new(CLIENT_ID);
            match auth.refresh(&secret).await {
                Ok(profile) => {
                    let session = UserSession::Microsoft {
                        uuid: profile.uuid.clone(),
                        username: profile.username.clone(),
                        skin_url: profile.skin.as_ref().map(|s| s.url.clone()),
                        cape_url: profile.cape.as_ref().map(|c| c.url.clone()),
                    };
                    let access = profile.access_token.expose_secret().to_owned();
                    let refresh = profile.refresh_token.as_ref().map(|t| t.expose_secret().to_owned());

                    // Update stored token and profile info
                    {
                        let guard = mgr_state.lock().await;
                        let mgr = guard.as_ref().unwrap();
                        let _ = mgr.update_account_token(account.id, refresh.as_deref()).await;
                        // Update profile info via upsert
                        let _ = mgr.add_account(
                            "microsoft",
                            &profile.uuid,
                            &profile.username,
                            refresh.as_deref(),
                            profile.skin.as_ref().map(|s| s.url.as_str()),
                            profile.cape.as_ref().map(|c| c.url.as_str()),
                        ).await;
                    }

                    let mut inner = state.lock().unwrap();
                    inner.session = Some(session.clone());
                    inner.access_token = Some(access);
                    inner.refresh_token = refresh;
                    Some(session)
                }
                Err(e) => {
                    log::warn!("failed to restore Microsoft session: {e}");
                    // Deactivate the failed account
                    let guard = mgr_state.lock().await;
                    let mgr = guard.as_ref().unwrap();
                    let _ = mgr.deactivate_all_accounts().await;
                    None
                }
            }
        }
        "offline" => {
            let session = UserSession::Offline {
                uuid: account.uuid,
                username: account.username,
            };
            let mut inner = state.lock().unwrap();
            inner.session = Some(session.clone());
            Some(session)
        }
        "demo" => {
            let session = UserSession::Demo {
                uuid: account.uuid,
                username: account.username,
            };
            let mut inner = state.lock().unwrap();
            inner.session = Some(session.clone());
            Some(session)
        }
        _ => None,
    };

    Ok(result)
}

// ---------------------------------------------------------------------------
// Multi-account management commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_accounts(
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Vec<lodestone_core::instance_manager::AccountRecord>, String> {
    ensure_manager(&mgr_state, &app).await?;
    let guard = mgr_state.lock().await;
    let mgr = guard.as_ref().unwrap();
    mgr.list_accounts()
        .await
        .map_err(|e| format!("failed to list accounts: {e}"))
}

#[tauri::command]
pub async fn switch_account(
    account_id: i64,
    state: tauri::State<'_, AuthState>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Option<UserSession>, String> {
    ensure_manager(&mgr_state, &app).await?;

    let account = {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        mgr.set_active_account(account_id)
            .await
            .map_err(|e| format!("failed to switch account: {e}"))?;
        mgr.get_active_account()
            .await
            .map_err(|e| format!("failed to get account: {e}"))?
    };

    let account = match account {
        Some(a) => a,
        None => return Ok(None),
    };

    let session = match account.mode.as_str() {
        "microsoft" => UserSession::Microsoft {
            uuid: account.uuid,
            username: account.username,
            skin_url: account.skin_url,
            cape_url: account.cape_url,
        },
        "offline" => UserSession::Offline {
            uuid: account.uuid,
            username: account.username,
        },
        "demo" => UserSession::Demo {
            uuid: account.uuid,
            username: account.username,
        },
        _ => return Ok(None),
    };

    let mut inner = state.lock().unwrap();
    inner.session = Some(session.clone());
    // Note: for Microsoft accounts, access_token needs refresh.
    // The caller should invoke restore_session after switching.
    inner.access_token = None;
    inner.refresh_token = account.refresh_token;

    Ok(Some(session))
}

#[tauri::command]
pub async fn remove_account(
    account_id: i64,
    state: tauri::State<'_, AuthState>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    ensure_manager(&mgr_state, &app).await?;

    // Check if we're removing the active account
    let was_active = {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        let active = mgr.get_active_account().await.ok().flatten();
        let is_active = active.as_ref().is_some_and(|a| a.id == account_id);
        mgr.remove_account(account_id)
            .await
            .map_err(|e| format!("failed to remove account: {e}"))?;
        is_active
    };

    if was_active {
        let mut inner = state.lock().unwrap();
        inner.session = None;
        inner.access_token = None;
        inner.refresh_token = None;
    }

    Ok(())
}
