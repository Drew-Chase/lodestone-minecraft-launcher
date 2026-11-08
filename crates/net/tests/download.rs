use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

/// A one-shot HTTP server that answers every request with `status` and `body`. Returns its address.
async fn serve(status: &'static str, body: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request).await;
            let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(head.as_bytes()).await;
            let _ = stream.write_all(body).await;
        }
    });
    format!("http://{address}/file")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-net-download-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The files in `dir`, by name.
fn listing(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

const BODY: &[u8] = b"hello world";
/// The SHA-1 of `BODY`.
const BODY_SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";

#[tokio::test]
async fn a_good_download_lands_at_its_path_and_leaves_no_part_file() {
    let url = serve("200 OK", BODY).await;
    let dir = scratch("good");
    let target = Target {
        url,
        path: dir.join("a.jar"),
        sha1: Some(BODY_SHA1.to_uppercase()),
        size: Some(BODY.len() as u64),
    };
    let mut seen = 0;
    let written = fetch(&reqwest::Client::new(), &target, |received, _| seen = received).await.unwrap();
    assert_eq!(written, BODY.len() as u64);
    assert_eq!(seen, BODY.len() as u64);
    assert_eq!(std::fs::read(&target.path).unwrap(), BODY);
    assert_eq!(listing(&dir), vec!["a.jar".to_string()]);
}

#[tokio::test]
async fn an_error_status_writes_nothing() {
    let url = serve("404 Not Found", b"<html>missing</html>").await;
    let dir = scratch("status");
    let target = Target::unchecked(url, dir.join("a.jar"));
    let error = fetch(&reqwest::Client::new(), &target, |_, _| {}).await.unwrap_err();
    assert!(matches!(error, Error::Status { status: 404, .. }), "{error}");
    assert!(listing(&dir).is_empty());
}

#[tokio::test]
async fn a_wrong_hash_leaves_the_old_file_alone() {
    let url = serve("200 OK", BODY).await;
    let dir = scratch("hash");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.jar"), b"old").unwrap();
    let target = Target {
        url,
        path: dir.join("a.jar"),
        sha1: Some("0000000000000000000000000000000000000000".to_string()),
        size: None,
    };
    let error = fetch(&reqwest::Client::new(), &target, |_, _| {}).await.unwrap_err();
    assert!(matches!(error, Error::Sha1 { .. }), "{error}");
    assert_eq!(std::fs::read(dir.join("a.jar")).unwrap(), b"old");
    assert_eq!(listing(&dir), vec!["a.jar".to_string()]);
}

#[tokio::test]
async fn a_wrong_size_fails() {
    let url = serve("200 OK", BODY).await;
    let dir = scratch("size");
    let target = Target {
        url,
        path: dir.join("a.jar"),
        sha1: None,
        size: Some(3),
    };
    let error = fetch(&reqwest::Client::new(), &target, |_, _| {}).await.unwrap_err();
    assert!(matches!(error, Error::Size { expected: 3, .. }), "{error}");
    assert!(!dir.join("a.jar").exists());
}
