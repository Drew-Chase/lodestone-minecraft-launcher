//! The multipart form a skin upload sends.

/// The boundary between the parts of the upload's form.
pub(crate) const BOUNDARY: &str = "lodestone-skin-upload";

/// The body of a skin upload: the model, then the PNG as `skin.png`.
pub(crate) fn skin_upload_body(slim: bool, png: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(png.len() + 256);
    push_text(&mut body, "variant", if slim { "slim" } else { "classic" });
    push_file(&mut body, "file", "skin.png", "image/png", png);
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
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
