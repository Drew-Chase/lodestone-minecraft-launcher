//! Tests for `pages/instance/files/servers.rs`, kept out of the source file. They are compiled as the `tests` child module of
//! `servers`.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::Serialize;

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-servers-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// Writes `servers.dat` in `folder` with `contents`.
fn write_list(folder: &Path, contents: &[u8]) {
    fs::write(folder.join("servers.dat"), contents).expect("list writes");
}

/// The list the game writes: an unnamed root that holds `servers`.
#[derive(Serialize)]
struct TestList {
    servers: Vec<TestServer>,
}

/// One entry of the list, with the keys a test sets.
#[derive(Serialize)]
struct TestServer {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    /// A byte flag the game writes as well, which the reader does not use.
    #[serde(rename = "acceptTextures")]
    accept_textures: i8,
}

fn entry(name: Option<&str>, ip: &str, icon: Option<String>) -> TestServer {
    TestServer {
        name: name.map(str::to_owned),
        ip: ip.to_owned(),
        icon,
        accept_textures: 1,
    }
}

fn list_bytes(servers: Vec<TestServer>) -> Vec<u8> {
    fastnbt::to_bytes(&TestList { servers }).expect("nbt encodes")
}

fn gzipped(nbt: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(nbt).expect("gzip writes");
    encoder.finish().expect("gzip finishes")
}

/// Bytes shaped like a PNG, base64 encoded the way the game keeps an icon.
fn icon_text() -> String {
    STANDARD.encode(b"\x89PNG\r\n\x1a\nicon")
}

#[test]
fn the_list_gives_each_server_its_name_address_and_icon_in_its_order() {
    let root = scratch("order");
    write_list(
        &root,
        &list_bytes(vec![
            entry(Some("Hub"), "mc.example.net", Some(icon_text())),
            entry(Some("Creative"), "192.168.1.42:25565", None),
        ]),
    );
    let servers = scan(&root);
    let names: Vec<&str> = servers.iter().map(|server| server.name.as_str()).collect();
    assert_eq!(names, ["Hub", "Creative"]);
    assert_eq!(servers[0].address, "mc.example.net");
    assert_eq!(servers[1].address, "192.168.1.42:25565");
    assert!(servers[0].icon.is_some());
    assert!(servers[1].icon.is_none());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_server_without_a_name_is_named_by_its_address() {
    let root = scratch("unnamed");
    write_list(
        &root,
        &list_bytes(vec![entry(None, "mc.example.net", None), entry(Some("  "), "play.example.net", None)]),
    );
    let names: Vec<String> = scan(&root).into_iter().map(|server| server.name).collect();
    assert_eq!(names, ["mc.example.net", "play.example.net"]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn an_entry_without_an_address_is_left_out() {
    let root = scratch("no-address");
    write_list(
        &root,
        &list_bytes(vec![entry(Some("Nowhere"), "", None), entry(Some("Hub"), "mc.example.net", None)]),
    );
    let addresses: Vec<String> = scan(&root).into_iter().map(|server| server.address).collect();
    assert_eq!(addresses, ["mc.example.net"]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn an_icon_with_the_data_url_prefix_decodes_and_a_broken_one_gives_none() {
    let root = scratch("icons");
    let prefixed = format!("data:image/png;base64,{}", icon_text());
    write_list(
        &root,
        &list_bytes(vec![
            entry(Some("Prefixed"), "a.example", Some(prefixed)),
            entry(Some("Broken"), "b.example", Some("not base64!".to_owned())),
        ]),
    );
    let servers = scan(&root);
    assert!(servers[0].icon.is_some());
    assert!(servers[1].icon.is_none());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_gzipped_list_reads_the_same_as_a_plain_one() {
    let root = scratch("gzipped");
    write_list(&root, &gzipped(&list_bytes(vec![entry(Some("Hub"), "mc.example.net", None)])));
    let servers = scan(&root);
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "Hub");
    assert_eq!(servers[0].address, "mc.example.net");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_or_broken_list_lists_no_servers() {
    let root = scratch("missing");
    assert!(scan(&root).is_empty());
    write_list(&root, b"not nbt at all");
    assert!(scan(&root).is_empty());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_search_matches_a_name_or_an_address_read_in_lower_case() {
    let server = ServerFile {
        name: "Ad Astra".to_owned(),
        address: "play.astra.net".to_owned(),
        icon: None,
    };
    assert!(server.matches(""));
    assert!(server.matches("astra"));
    assert!(server.matches("play.astra"));
    assert!(!server.matches("hub"));
}
