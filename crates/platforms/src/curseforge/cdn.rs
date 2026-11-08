//! CurseForge's file CDN. A file's direct download address is built from its id and name, for when its record names no url.

use crate::http::percent_encode;

/// Where CurseForge serves its files from.
const FILES: &str = "https://mediafilez.forgecdn.net/files";

/// The direct download address of the file `file_id` named `file_name`. The CDN files a download under the id split into
/// all but its last three digits and those three digits, with no leading zeros: `9394038` is `files/9394/38`, and
/// `9323162` is `files/9323/162`. The name is percent-encoded as the last path segment.
pub(crate) fn direct_url(file_id: i64, file_name: &str) -> String {
    format!("{FILES}/{}/{}/{}", file_id / 1000, file_id % 1000, percent_encode(file_name))
}
