use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A status as a server sends it: a version, players, a message of the day with formatting, and an icon.
const SAMPLE: &str = r#"{"version":{"name":"1.21.4","protocol":769},"players":{"max":20,"online":3,"sample":[]},"description":{"text":"Hello §a","extra":[{"text":"world"}]},"favicon":"data:image/png;base64,iVBORw0KGgo="}"#;

#[test]
fn a_status_reads_its_version_players_message_and_icon() {
    let status = from_json(SAMPLE).expect("the sample reads");
    assert_eq!(status.version, "1.21.4");
    assert_eq!(status.protocol, 769);
    assert_eq!(status.online, 3);
    assert_eq!(status.max, 20);
    assert_eq!(status.motd, "Hello world");
    assert_eq!(status.favicon.as_deref(), Some("data:image/png;base64,iVBORw0KGgo="));
}

#[test]
fn a_status_with_no_fields_reads_as_empty() {
    let status = from_json("{}").expect("an empty status reads");
    assert_eq!(status.version, "");
    assert_eq!(status.online, 0);
    assert_eq!(status.favicon, None);
}

#[test]
fn text_that_is_not_json_is_malformed() {
    assert!(matches!(from_json("nope"), Err(PingError::Malformed(_))));
}

#[test]
fn an_icon_decodes_to_its_png_bytes_and_other_addresses_do_not() {
    assert_eq!(
        favicon_png("data:image/png;base64,iVBORw0KGgo="),
        Some(vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
    );
    assert_eq!(favicon_png("https://example.com/icon.png"), None);
}

/// Reads one packet that a client sent, returning its id and its body.
async fn read_client_packet(stream: &mut TcpStream) -> (i32, Vec<u8>) {
    let mut length = Vec::new();
    loop {
        let byte = stream.read_u8().await.expect("a length byte");
        length.push(byte);
        if byte & 0x80 == 0 {
            break;
        }
    }
    let (size, _) = protocol::read_varint(&length).expect("a length");
    let mut payload = vec![0; usize::try_from(size).expect("a positive length")];
    stream.read_exact(&mut payload).await.expect("the payload");
    let (id, used) = protocol::read_varint(&payload).expect("an id");
    (id, payload.split_off(used))
}

#[tokio::test]
async fn a_server_that_answers_gives_its_status_and_a_latency() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let port = listener.local_addr().expect("an address").port();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accepts");
        let (handshake, _) = read_client_packet(&mut stream).await;
        assert_eq!(handshake, HANDSHAKE);
        let (request, _) = read_client_packet(&mut stream).await;
        assert_eq!(request, STATUS_REQUEST);
        let mut json = Vec::new();
        protocol::write_string(&mut json, SAMPLE);
        stream.write_all(&protocol::frame(0, &json)).await.expect("the status is written");
        let (ping, body) = read_client_packet(&mut stream).await;
        assert_eq!(ping, PING);
        stream.write_all(&protocol::frame(1, &body)).await.expect("the pong is written");
    });
    let address = ServerAddress {
        host: "127.0.0.1".to_owned(),
        port,
    };
    let status = ping(&address, Duration::from_secs(5)).await.expect("the server answers");
    assert_eq!(status.online, 3);
    assert_eq!(status.motd, "Hello world");
    assert!(status.latency < Duration::from_secs(5));
}

#[tokio::test]
async fn a_port_with_nothing_listening_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let port = listener.local_addr().expect("an address").port();
    drop(listener);
    let address = ServerAddress {
        host: "127.0.0.1".to_owned(),
        port,
    };
    assert!(matches!(ping(&address, Duration::from_secs(5)).await, Err(PingError::Unreachable(_))));
}

#[tokio::test]
async fn a_server_that_never_answers_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let port = listener.local_addr().expect("an address").port();
    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.expect("accepts");
        tokio::time::sleep(Duration::from_secs(5)).await;
    });
    let address = ServerAddress {
        host: "127.0.0.1".to_owned(),
        port,
    };
    assert_eq!(ping(&address, Duration::from_millis(200)).await, Err(PingError::Timeout));
}
