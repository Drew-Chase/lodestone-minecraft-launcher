//! The CurseForge modpack page an import is taken from. A modpack page names its project by a slug, and the page of one
//! of its files also names the file by its id.

/// A CurseForge modpack page, read from the address the user typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModpackPage {
    /// The project's slug, which the address names.
    pub slug: String,
    /// The file the address names. Without one, the newest file is taken.
    pub file_id: Option<String>,
}

impl ModpackPage {
    /// The project's page without any file, which the instance keeps as its source.
    pub fn project_url(&self) -> String {
        format!("https://www.curseforge.com/minecraft/modpacks/{}", self.slug)
    }

    /// The address the import is started from: the project's page, with the file's page after it when there is one.
    pub fn address(&self) -> String {
        match &self.file_id {
            Some(file) => format!("{}/files/{file}", self.project_url()),
            None => self.project_url(),
        }
    }
}

/// Reads a CurseForge modpack page from `text`: `curseforge.com/minecraft/modpacks/<slug>`, optionally followed by
/// `/files/<file id>`. The scheme may be left out. Any other host, game, or page is `None`.
pub fn parse(text: &str) -> Option<ModpackPage> {
    let text = text.trim();
    let with_scheme = if text.contains("://") {
        text.to_string()
    } else {
        format!("https://{text}")
    };
    let url = reqwest::Url::parse(&with_scheme).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    if !matches!(url.host_str(), Some("curseforge.com" | "www.curseforge.com")) {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.filter(|segment| !segment.is_empty()).collect();
    match segments.as_slice() {
        ["minecraft", "modpacks", slug] if is_slug(slug) => Some(ModpackPage {
            slug: slug.to_string(),
            file_id: None,
        }),
        ["minecraft", "modpacks", slug, "files", file] if is_slug(slug) && is_id(file) => Some(ModpackPage {
            slug: slug.to_string(),
            file_id: Some(file.to_string()),
        }),
        _ => None,
    }
}

/// Whether `slug` is a project slug: letters, digits, dashes and underscores.
fn is_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Whether `id` is a CurseForge id: digits only.
fn is_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
#[path = "../../../tests/app/import/page.rs"]
mod tests;
