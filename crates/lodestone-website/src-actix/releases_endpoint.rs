use actix_web::{get, web, HttpResponse, Responder};
use crate::util::github_releases::ReleasesCache;
use crate::util::http_error::Result;
use serde_json::json;
use std::sync::Arc;

/// GET /api/releases
/// Returns all non-draft releases with assets and platform info.
#[get("/")]
async fn all_releases(cache: web::Data<Arc<ReleasesCache>>) -> Result<impl Responder> {
    let releases = cache
        .get_releases()
        .await?;
    Ok(HttpResponse::Ok().json(releases))
}

/// GET /api/releases/latest
/// Returns the latest non-prerelease with grouped platform downloads.
#[get("/latest")]
async fn latest_release(cache: web::Data<Arc<ReleasesCache>>) -> Result<impl Responder> {
    let latest = cache
        .get_latest()
        .await?;

    match latest {
        Some(release) => Ok(HttpResponse::Ok().json(release)),
        None => Ok(HttpResponse::NotFound().json(json!({ "error": "no releases found" }))),
    }
}

/// GET /api/releases/latest/version
/// Returns just the version string of the latest release — useful for update checks.
#[get("/latest/version")]
async fn latest_version(cache: web::Data<Arc<ReleasesCache>>) -> Result<impl Responder> {
    let latest = cache
        .get_latest()
        .await?;

    match latest {
        Some(release) => Ok(HttpResponse::Ok().json(json!({
            "version": release.release.version,
            "tag": release.release.tag,
            "published_at": release.release.published_at,
        }))),
        None => Ok(HttpResponse::NotFound().json(json!({ "error": "no releases found" }))),
    }
}

/// GET /api/releases/latest/changelog
/// Returns the changelog/body of the latest release as markdown text.
#[get("/latest/changelog")]
async fn latest_changelog(cache: web::Data<Arc<ReleasesCache>>) -> Result<impl Responder> {
    let latest = cache
        .get_latest()
        .await?;

    match latest {
        Some(release) => Ok(HttpResponse::Ok()
            .content_type("text/markdown; charset=utf-8")
            .body(release.release.changelog)),
        None => Ok(HttpResponse::NotFound()
            .content_type("text/plain")
            .body("no releases found")),
    }
}

/// GET /api/releases/latest/{platform}
/// Returns download assets for a specific platform (windows, macos, linux).
#[get("/latest/{platform}")]
async fn latest_platform(
    cache: web::Data<Arc<ReleasesCache>>,
    platform: web::Path<String>,
) -> Result<impl Responder> {
    let latest = cache
        .get_latest()
        .await?;

    let latest = match latest {
        Some(l) => l,
        None => return Ok(HttpResponse::NotFound().json(json!({ "error": "no releases found" }))),
    };

    let platform_key = platform.into_inner().to_lowercase();
    let available: Vec<String> = latest.downloads.iter().map(|d| d.platform.clone()).collect();
    let download = latest
        .downloads
        .into_iter()
        .find(|d| d.platform == platform_key);

    match download {
        Some(d) => Ok(HttpResponse::Ok().json(json!({
            "version": latest.release.version,
            "tag": latest.release.tag,
            "platform": d.platform,
            "label": d.label,
            "assets": d.assets,
        }))),
        None => Ok(HttpResponse::NotFound().json(json!({
            "error": format!("no assets for platform '{}'", platform_key),
            "available_platforms": available,
        }))),
    }
}

/// GET /api/releases/{tag}
/// Returns a specific release by its tag name (e.g., "v0.1.0").
#[get("/{tag}")]
async fn release_by_tag(
    cache: web::Data<Arc<ReleasesCache>>,
    tag: web::Path<String>,
) -> Result<impl Responder> {
    let releases = cache
        .get_releases()
        .await?;

    let tag_str = tag.into_inner();
    let release = releases.into_iter().find(|r| r.tag == tag_str);

    match release {
        Some(r) => Ok(HttpResponse::Ok().json(r)),
        None => Ok(HttpResponse::NotFound().json(json!({
            "error": format!("release '{}' not found", tag_str),
        }))),
    }
}

/// GET /api/stats
/// Returns GitHub stars and total download count across all releases.
#[get("/stats")]
async fn repo_stats(cache: web::Data<Arc<ReleasesCache>>) -> Result<impl Responder> {
    let stats = cache.get_stats().await?;
    Ok(HttpResponse::Ok().json(stats))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(repo_stats)
        .service(
            web::scope("/releases")
                .service(all_releases)
                .service(latest_release)
                .service(latest_version)
                .service(latest_changelog)
                .service(latest_platform)
                .service(release_by_tag),
        );
}
