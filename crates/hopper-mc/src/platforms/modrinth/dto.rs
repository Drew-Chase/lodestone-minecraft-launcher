//! Raw wire-format types for the Modrinth v2 API.
//!
//! These are kept deliberately minimal — we only deserialize the fields we
//! actually surface on the public model. Unknown fields are ignored by default.
//!
//! Some fields are retained on the DTOs (e.g. `project_type`, `versions`,
//! `team`) even though the current mapping doesn't consume them — they
//! document the wire shape and let us wire them up later without touching
//! the deserialization layer.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub total_hits: u32,
}

/// A hit in `GET /v2/search`. Much thinner than a full project document.
#[derive(Debug, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub display_categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
    #[serde(default)]
    pub client_side: Option<String>,
    #[serde(default)]
    pub server_side: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub project_type: Option<String>,
    #[serde(default)]
    pub gallery: Vec<String>,
    #[serde(default)]
    pub featured_gallery: Option<String>,
}

/// Full project document from `GET /v2/project/{id|slug}`.
#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: String, // short form (summary-ish)
    #[serde(default)]
    pub body: String, // long-form markdown
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub additional_categories: Vec<String>,
    #[serde(default)]
    pub client_side: Option<String>,
    #[serde(default)]
    pub server_side: Option<String>,
    #[serde(default)]
    pub project_type: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub followers: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub issues_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub wiki_url: Option<String>,
    #[serde(default)]
    pub discord_url: Option<String>,
    #[serde(default)]
    pub donation_urls: Vec<DonationUrl>,
    #[serde(default)]
    pub gallery: Vec<GalleryItem>,
    pub published: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    #[serde(default)]
    pub license: Option<ProjectLicense>,
    #[serde(default)]
    pub versions: Vec<String>, // version IDs, not MC versions
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub team: Option<String>, // team id; members are a separate endpoint
}

#[derive(Debug, Deserialize)]
pub struct ProjectLicense {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DonationUrl {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct GalleryItem {
    pub url: String,
    #[serde(default)]
    pub featured: bool,
}

/// Team member — not used yet but declared so the shape exists if we
/// wire the `team/{id}/members` endpoint later.
#[derive(Debug, Deserialize)]
pub struct TeamMember {
    pub user: TeamUser,
    #[serde(default)]
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct TeamUser {
    pub username: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}
