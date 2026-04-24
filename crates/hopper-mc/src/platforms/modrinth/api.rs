//! Thin wrapper over the Modrinth Labrinth v2 HTTP API.
//!
//! Every function here returns raw wire-format structs from [`super::dto`];
//! translation into public model types is done in [`super::mapping`].

use reqwest::StatusCode;

use crate::error::{ContentError, Result};
use crate::platform::{ContentType, Sort};

use super::dto::{Project, SearchResponse};
use super::mapping;

pub(crate) const BASE_URL: &str = "https://api.modrinth.com/v2";

/// Run a `/search` query for the given content type.
pub(crate) async fn search(
    client: &reqwest::Client,
    query: Option<&str>,
    sort: Sort,
    kind: ContentType,
    page: u32,
    per_page: u32,
) -> Result<SearchResponse> {
    let Some(facets) = mapping::content_type_to_facets(kind) else {
        return Err(ContentError::UnsupportedContentType {
            platform: crate::platform::Platform::Modrinth,
            kind,
        });
    };

    let limit = per_page.clamp(1, 100);
    let offset = page.saturating_mul(limit);
    let index = mapping::sort_to_index(sort);

    let mut req = client
        .get(format!("{BASE_URL}/search"))
        .query(&[
            ("index", index),
            ("offset", &offset.to_string()),
            ("limit", &limit.to_string()),
            ("facets", &facets),
        ]);

    if let Some(q) = query {
        req = req.query(&[("query", q)]);
    }

    let resp = req.send().await?;
    handle_common_status(&resp)?;
    let resp = resp.error_for_status()?;
    let body: SearchResponse = resp.json().await?;
    Ok(body)
}

/// Fetch a single project by id or slug. Returns `Ok(None)` on 404.
pub(crate) async fn get_project(
    client: &reqwest::Client,
    id_or_slug: &str,
) -> Result<Option<Project>> {
    let resp = client
        .get(format!("{BASE_URL}/project/{id_or_slug}"))
        .send()
        .await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    handle_common_status(&resp)?;
    let resp = resp.error_for_status()?;
    let body: Project = resp.json().await?;
    Ok(Some(body))
}

fn handle_common_status(resp: &reqwest::Response) -> Result<()> {
    match resp.status() {
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
            "modrinth rejected request: {}",
            resp.status()
        ))),
        _ => Ok(()),
    }
}
