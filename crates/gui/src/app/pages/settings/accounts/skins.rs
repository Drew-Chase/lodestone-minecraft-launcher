//! The saved skins: the PNG files in the skins folder, the colour and size the selected panel shows, and the upload of a
//! skin to the Microsoft profile.

use std::path::Path;

/// The address the profile's skins are uploaded to.
const UPLOAD_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";

/// The boundary between the parts of the upload's form.
const BOUNDARY: &str = "lodestone-skin-upload";

/// One skin in the skins folder.
#[derive(Debug, Clone)]
pub struct LibrarySkin {
    /// The file's name without its extension, which the page shows.
    pub name: String,
    /// The picture the grid shows.
    pub handle: iced::widget::image::Handle,
    /// The PNG's bytes, which the upload sends.
    pub png: Vec<u8>,
}

/// A skin the page staged for upload on the next Save. The root confirms it, then uploads it.
#[derive(Debug, Clone)]
pub struct PendingSkin {
    pub name: String,
    pub png: Vec<u8>,
    pub slim: bool,
}

/// Reads the PNG skins in `dir`, sorted by name. A folder that cannot be read has no skins.
pub fn scan(dir: &Path) -> Vec<LibrarySkin> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut skins: Vec<LibrarySkin> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("png")))
        .filter_map(|path| {
            let png = std::fs::read(&path).ok()?;
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let handle = iced::widget::image::Handle::from_bytes(png.clone());
            Some(LibrarySkin { name, handle, png })
        })
        .collect();
    skins.sort_by(|a, b| a.name.cmp(&b.name));
    skins
}

/// The size of a PNG, read from its header. `None` when the bytes are not a PNG.
pub fn dimensions(png: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(png))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// The most common color among the skin's visible pixels, as `#rrggbb`, with each channel rounded to four bits. Near-black
/// pixels are skipped, so a dark outline does not stand for the skin.
pub fn accent_hex(png: &[u8]) -> Option<String> {
    let image = image::load_from_memory(png).ok()?.to_rgba8();
    let mut counts = [0u32; 4096];
    for pixel in image.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha < 128 || u32::from(red) + u32::from(green) + u32::from(blue) < 90 {
            continue;
        }
        let bucket = usize::from(red >> 4) << 8 | usize::from(green >> 4) << 4 | usize::from(blue >> 4);
        counts[bucket] += 1;
    }
    let (bucket, count) = counts.iter().enumerate().max_by_key(|(_, count)| **count)?;
    if *count == 0 {
        return None;
    }
    let channel = |shift: usize| (((bucket >> shift) & 0xF) as u8) * 17;
    Some(format!("#{:02x}{:02x}{:02x}", channel(8), channel(4), channel(0)))
}

/// Uploads `png` as the profile's skin, in the classic or slim model. The upload makes the skin the one in use.
pub async fn upload(token: String, slim: bool, png: Vec<u8>) -> Result<(), String> {
    let mut body = Vec::with_capacity(png.len() + 256);
    push_text(&mut body, "variant", if slim { "slim" } else { "classic" });
    push_file(&mut body, "file", "skin.png", "image/png", &png);
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    let response = lodestone_net::client()
        .post(UPLOAD_URL)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, format!("multipart/form-data; boundary={BOUNDARY}"))
        .body(body)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("the profile service answered {}", response.status()))
    }
}

/// Adds a plain form field to the upload.
fn push_text(body: &mut Vec<u8>, name: &str, value: &str) {
    body.extend_from_slice(format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
}

/// Adds a file field to the upload.
fn push_file(body: &mut Vec<u8>, name: &str, file_name: &str, mime: &str, bytes: &[u8]) {
    body.extend_from_slice(
        format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\"\r\nContent-Type: {mime}\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");
}
