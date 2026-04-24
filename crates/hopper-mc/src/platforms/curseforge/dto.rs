//! Raw wire-format types for the CurseForge Core API v1.
//!
//! CurseForge wraps every response in `{ "data": ... }`; list endpoints add a
//! `pagination` sibling. We model that with the generic [`Envelope`] and
//! [`PaginatedEnvelope`] types below.
//!
//! Only fields we actually map into the public model are exposed here.
//! Unknown fields are ignored by default so CurseForge can evolve the API
//! without breaking deserialization.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Envelope<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct PaginatedEnvelope<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub page_size: u32,
    #[serde(default)]
    pub result_count: u32,
    #[serde(default)]
    pub total_count: u64,
}

/// A CurseForge project (what CurseForge calls a "Mod" regardless of content type).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: u64,
    pub game_id: u64,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub links: ModLinks,
    #[serde(default)]
    pub summary: String,
    /// Approval status. 4 = Approved. We don't filter on this by default.
    #[serde(default)]
    pub status: Option<u32>,
    #[serde(default)]
    pub download_count: u64,
    #[serde(default)]
    pub is_featured: bool,
    #[serde(default)]
    pub primary_category_id: Option<u64>,
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub class_id: Option<u64>,
    #[serde(default)]
    pub authors: Vec<ModAuthor>,
    #[serde(default)]
    pub logo: Option<ModAsset>,
    #[serde(default)]
    pub screenshots: Vec<ModAsset>,
    #[serde(default)]
    pub latest_files: Vec<CfFile>,
    #[serde(default)]
    pub latest_files_indexes: Vec<FileIndex>,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
    #[serde(default)]
    pub date_released: Option<DateTime<Utc>>,
    #[serde(default)]
    pub allow_mod_distribution: Option<bool>,
    #[serde(default)]
    pub game_popularity_rank: Option<u64>,
    #[serde(default)]
    pub is_available: Option<bool>,
    #[serde(default)]
    pub thumbs_up_count: u64,
    #[serde(default)]
    pub rating: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModLinks {
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub wiki_url: Option<String>,
    #[serde(default)]
    pub issues_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub class_id: Option<u64>,
    #[serde(default)]
    pub is_class: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModAuthor {
    #[serde(default)]
    pub id: Option<u64>,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModAsset {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// An index row pointing at one game-version / loader combination.
/// This is where loader info lives on the project object.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub file_id: u64,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub release_type: Option<u32>,
    #[serde(default)]
    pub game_version_type_id: Option<u64>,
    /// 0=Any, 1=Forge, 2=Cauldron, 3=LiteLoader, 4=Fabric, 5=Quilt, 6=NeoForge.
    #[serde(default)]
    pub mod_loader: Option<u32>,
}

/// Minimal file shape — we keep what little we surface. `downloadUrl` is
/// intentionally optional because CurseForge nulls it when the project
/// disallows third-party distribution.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: u64,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub file_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub file_length: Option<u64>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
}
