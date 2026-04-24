//! Thin wrapper over the CurseForge Core API v1.
//!
//! Every call requires an `x-api-key` header. Callers that have not
//! configured a key hit a [`ContentError::BadRequest`] before any network
//! request is made — there is no anonymous mode.
//!
//! Translation from raw DTOs to public model types is done in
//! [`super::mapping`]; this module stays at the wire level.

use reqwest::StatusCode;

use crate::error::{ContentError, Result};
use crate::platform::{ContentType, Sort};

use super::dto::{CfMod, Envelope, PaginatedEnvelope};
use super::mapping;

pub(crate) const BASE_URL: &str = "https://api.curseforge.com/v1";

/// Max page size accepted by `/mods/search`.
const MAX_PAGE_SIZE: u32 = 50;

/// CurseForge hard-caps `index + pageSize` at this value — requests beyond
/// it return 400. We saturate to this limit rather than letting callers
/// paginate into an error.
const PAGINATION_HARD_CAP: u32 = 10_000;

fn require_key(api_key: Option<&str>) -> Result<&str> {
    api_key.filter(|k| !k.is_empty()).ok_or_else(|| {
        ContentError::BadRequest(
            "CurseForge requires an API key. Construct the provider via \
             CurseForgeProvider::new_with_secret_key(Some(key)); see \
             examples/curseforge_with_keyring.rs for a secure flow that \
             loads the key from the OS credential store."
                .to_string(),
        )
    })
}

/// Run a `/mods/search` query for the given content type.
pub(crate) async fn search(
    client: &reqwest::Client,
    api_key: Option<&str>,
    query: Option<&str>,
    sort: Sort,
    kind: ContentType,
    page: u32,
    per_page: u32,
) -> Result<Vec<CfMod>> {
    let key = require_key(api_key)?;
    let Some(class_id) = mapping::content_type_to_class_id(kind) else {
        return Err(ContentError::UnsupportedContentType {
            platform: crate::platform::Platform::CurseForge,
            kind,
        });
    };

    let limit = per_page.clamp(1, MAX_PAGE_SIZE);
    let raw_offset = page.saturating_mul(limit);
    let offset = raw_offset.min(PAGINATION_HARD_CAP.saturating_sub(limit));
    let (sort_field, sort_order) = mapping::sort_to_cf(sort);

    let game_id_s = mapping::MINECRAFT_GAME_ID.to_string();
    let class_id_s = class_id.to_string();
    let sort_field_s = sort_field.to_string();
    let offset_s = offset.to_string();
    let limit_s = limit.to_string();

    let mut params: Vec<(&str, &str)> = vec![
        ("gameId", &game_id_s),
        ("classId", &class_id_s),
        ("sortField", &sort_field_s),
        ("sortOrder", sort_order),
        ("index", &offset_s),
        ("pageSize", &limit_s),
    ];
    if let Some(q) = query.filter(|s| !s.is_empty()) {
        params.push(("searchFilter", q));
    }

    let resp = client
        .get(format!("{BASE_URL}/mods/search"))
        .header("x-api-key", key)
        .query(&params)
        .send()
        .await?;

    handle_common_status(&resp)?;
    let resp = resp.error_for_status()?;
    let body: PaginatedEnvelope<CfMod> = resp.json().await?;
    Ok(body.data)
}

/// Fetch a single project by numeric id. Returns `Ok(None)` on 404.
pub(crate) async fn get_mod_by_id(
    client: &reqwest::Client,
    api_key: Option<&str>,
    id: u64,
) -> Result<Option<CfMod>> {
    let key = require_key(api_key)?;
    let resp = client
        .get(format!("{BASE_URL}/mods/{id}"))
        .header("x-api-key", key)
        .send()
        .await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    handle_common_status(&resp)?;
    let resp = resp.error_for_status()?;
    let body: Envelope<CfMod> = resp.json().await?;
    Ok(Some(body.data))
}

/// Look up a project by slug within a content class. CurseForge's
/// `/mods/search?slug=...&classId=...` returns at most one match when the
/// slug is exact; we pick the first hit (or `None`).
pub(crate) async fn get_mod_by_slug(
    client: &reqwest::Client,
    api_key: Option<&str>,
    slug: &str,
    class_id: u64,
) -> Result<Option<CfMod>> {
    let key = require_key(api_key)?;
    let game_id_s = mapping::MINECRAFT_GAME_ID.to_string();
    let class_id_s = class_id.to_string();

    let resp = client
        .get(format!("{BASE_URL}/mods/search"))
        .header("x-api-key", key)
        .query(&[
            ("gameId", game_id_s.as_str()),
            ("classId", class_id_s.as_str()),
            ("slug", slug),
            ("pageSize", "1"),
        ])
        .send()
        .await?;

    handle_common_status(&resp)?;
    let resp = resp.error_for_status()?;
    let body: PaginatedEnvelope<CfMod> = resp.json().await?;
    Ok(body.data.into_iter().next())
}

/// Fetch a project by either numeric id or slug, filtered to the given
/// content class when falling back to slug search.
pub(crate) async fn get_by_id_or_slug(
    client: &reqwest::Client,
    api_key: Option<&str>,
    id_or_slug: &str,
    class_id: u64,
) -> Result<Option<CfMod>> {
    if let Ok(id) = id_or_slug.parse::<u64>() {
        get_mod_by_id(client, api_key, id).await
    } else {
        get_mod_by_slug(client, api_key, id_or_slug, class_id).await
    }
}

fn handle_common_status(resp: &reqwest::Response) -> Result<()> {
    match resp.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ContentError::BadRequest(
            format!("curseforge rejected credentials: {}", resp.status()),
        )),
        StatusCode::TOO_MANY_REQUESTS => {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(std::time::Duration::from_secs);
            Err(ContentError::RateLimited(retry_after))
        }
        StatusCode::BAD_REQUEST => Err(ContentError::BadRequest(format!(
            "curseforge rejected request: {}",
            resp.status()
        ))),
        _ => Ok(()),
    }
}
