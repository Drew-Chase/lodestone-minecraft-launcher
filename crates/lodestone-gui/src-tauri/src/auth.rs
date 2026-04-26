use std::path::PathBuf;
use std::sync::Mutex;

use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tauri::Manager;

use emerald_auth::MicrosoftAuth;

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
// On-disk persistence
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

fn auth_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("auth.json"))
}

fn save_persisted(app: &tauri::AppHandle, auth: &PersistedAuth) -> Result<(), String> {
    let path = auth_file_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("failed to create app data dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(auth).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("failed to write auth.json: {e}"))
}

fn load_persisted(app: &tauri::AppHandle) -> Result<Option<PersistedAuth>, String> {
    let path = auth_file_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path).map_err(|e| format!("failed to read auth.json: {e}"))?;
    let auth: PersistedAuth = serde_json::from_str(&data).map_err(|e| format!("failed to parse auth.json: {e}"))?;
    Ok(Some(auth))
}

fn clear_persisted(app: &tauri::AppHandle) {
    if let Ok(path) = auth_file_path(app) {
        let _ = std::fs::remove_file(path);
    }
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
    app: tauri::AppHandle,
) -> Result<UserSession, String> {
    let auth = MicrosoftAuth::new(CLIENT_ID);
    let profile = auth.authenticate().await.map_err(format_auth_error)?;

    let session = UserSession::Microsoft {
        uuid: profile.uuid,
        username: profile.username,
        skin_url: profile.skin.map(|s| s.url),
        cape_url: profile.cape.map(|c| c.url),
    };

    let access_token = profile.access_token.expose_secret().to_owned();
    let refresh_token = profile.refresh_token.as_ref().map(|t| t.expose_secret().to_owned());

    // Persist refresh token for session restoration.
    if let Some(ref rt) = refresh_token {
        let _ = save_persisted(&app, &PersistedAuth::Microsoft {
            refresh_token: rt.clone(),
        });
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
        uuid,
        username: username.clone(),
    };

    let _ = save_persisted(&app, &PersistedAuth::Offline { username });

    let mut inner = state.lock().unwrap();
    inner.session = Some(session.clone());
    inner.access_token = None;
    inner.refresh_token = None;

    Ok(session)
}

#[tauri::command]
pub async fn login_demo(
    state: tauri::State<'_, AuthState>,
    app: tauri::AppHandle,
) -> Result<UserSession, String> {
    let username = "Steve".to_string();
    let uuid = offline_uuid(&username);
    let session = UserSession::Demo { uuid, username };

    let _ = save_persisted(&app, &PersistedAuth::Demo);

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
    app: tauri::AppHandle,
) -> Result<(), String> {
    clear_persisted(&app);
    let mut inner = state.lock().unwrap();
    inner.session = None;
    inner.access_token = None;
    inner.refresh_token = None;
    Ok(())
}

#[tauri::command]
pub async fn restore_session(
    state: tauri::State<'_, AuthState>,
    app: tauri::AppHandle,
) -> Result<Option<UserSession>, String> {
    let persisted = match load_persisted(&app)? {
        Some(p) => p,
        None => return Ok(None),
    };

    let result = match persisted {
        PersistedAuth::Microsoft { refresh_token } => {
            log::info!("restoring Microsoft session via token refresh");
            let secret = secrecy::SecretString::from(refresh_token);
            let auth = MicrosoftAuth::new(CLIENT_ID);
            match auth.refresh(&secret).await {
                Ok(profile) => {
                    let session = UserSession::Microsoft {
                        uuid: profile.uuid,
                        username: profile.username,
                        skin_url: profile.skin.map(|s| s.url),
                        cape_url: profile.cape.map(|c| c.url),
                    };
                    let access = profile.access_token.expose_secret().to_owned();
                    let refresh = profile.refresh_token.as_ref().map(|t| t.expose_secret().to_owned());

                    // Update persisted refresh token.
                    if let Some(ref rt) = refresh {
                        let _ = save_persisted(&app, &PersistedAuth::Microsoft {
                            refresh_token: rt.clone(),
                        });
                    }

                    let mut inner = state.lock().unwrap();
                    inner.session = Some(session.clone());
                    inner.access_token = Some(access);
                    inner.refresh_token = refresh;
                    Some(session)
                }
                Err(e) => {
                    log::warn!("failed to restore Microsoft session: {e}");
                    clear_persisted(&app);
                    None
                }
            }
        }
        PersistedAuth::Offline { username } => {
            let uuid = offline_uuid(&username);
            let session = UserSession::Offline { uuid, username };
            let mut inner = state.lock().unwrap();
            inner.session = Some(session.clone());
            Some(session)
        }
        PersistedAuth::Demo => {
            let username = "Steve".to_string();
            let uuid = offline_uuid(&username);
            let session = UserSession::Demo { uuid, username };
            let mut inner = state.lock().unwrap();
            inner.session = Some(session.clone());
            Some(session)
        }
    };

    Ok(result)
}
