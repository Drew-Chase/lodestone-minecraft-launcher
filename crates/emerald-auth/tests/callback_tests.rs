use std::time::Duration;

use emerald_auth::callback::CallbackServer;
use emerald_auth::AuthError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Helper: connect to the callback server and send a raw HTTP GET request.
async fn send_request(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();

    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await.unwrap();
    String::from_utf8_lossy(&buf[..n]).to_string()
}

/// Extract the port from a redirect URI like "http://localhost:12345".
fn port_from_uri(uri: &str) -> u16 {
    uri.strip_prefix("http://localhost:")
        .unwrap()
        .parse()
        .unwrap()
}

#[tokio::test]
async fn bind_returns_localhost_redirect_uri() {
    let (server, uri) = CallbackServer::bind().await.unwrap();
    assert!(uri.starts_with("http://localhost:"));
    let port = port_from_uri(&uri);
    assert!(port > 0);
    drop(server);
}

#[tokio::test]
async fn bind_to_specific_port() {
    // Use a high ephemeral port unlikely to conflict.
    let port = 39182;
    let result = CallbackServer::bind_to(port).await;
    match result {
        Ok((server, uri)) => {
            assert_eq!(uri, format!("http://localhost:{port}"));
            drop(server);
        }
        Err(_) => {
            // Port might be in use on CI — that's acceptable.
        }
    }
}

#[tokio::test]
async fn callback_extracts_code_and_state() {
    let (server, uri) = CallbackServer::bind().await.unwrap();
    let port = port_from_uri(&uri);

    let server_handle = tokio::spawn(async move {
        server
            .wait_for_callback(Duration::from_secs(5))
            .await
    });

    // Simulate the browser redirect (Microsoft redirects to /?code=...&state=...).
    let response = send_request(port, "/?code=abc123&state=xyz789").await;

    let (code, state) = server_handle.await.unwrap().unwrap();
    assert_eq!(code, "abc123");
    assert_eq!(state, "xyz789");
    assert!(response.contains("200 OK"));
    assert!(response.contains("Authentication Successful"));
}

#[tokio::test]
async fn callback_extracts_url_encoded_params() {
    let (server, uri) = CallbackServer::bind().await.unwrap();
    let port = port_from_uri(&uri);

    let server_handle = tokio::spawn(async move {
        server
            .wait_for_callback(Duration::from_secs(5))
            .await
    });

    // Code with special characters (URL-encoded).
    send_request(port, "/?code=M.C507_BL2.-CRd5%2B&state=my%20state").await;

    let (code, state) = server_handle.await.unwrap().unwrap();
    assert_eq!(code, "M.C507_BL2.-CRd5+");
    assert_eq!(state, "my state");
}

#[tokio::test]
async fn callback_returns_oauth_error() {
    let (server, uri) = CallbackServer::bind().await.unwrap();
    let port = port_from_uri(&uri);

    let server_handle = tokio::spawn(async move {
        server
            .wait_for_callback(Duration::from_secs(5))
            .await
    });

    let response = send_request(
        port,
        "/?error=access_denied&error_description=The+user+denied+access",
    )
    .await;

    let err = server_handle.await.unwrap().unwrap_err();
    match err {
        AuthError::OAuth { error, description } => {
            assert_eq!(error, "access_denied");
            assert_eq!(description, "The user denied access");
        }
        other => panic!("expected OAuth error, got: {other}"),
    }
    assert!(response.contains("Authentication Failed"));
}

#[tokio::test]
async fn callback_missing_code_returns_error() {
    let (server, uri) = CallbackServer::bind().await.unwrap();
    let port = port_from_uri(&uri);

    let server_handle = tokio::spawn(async move {
        server
            .wait_for_callback(Duration::from_secs(5))
            .await
    });

    send_request(port, "/?state=xyz789").await;

    let err = server_handle.await.unwrap().unwrap_err();
    match err {
        AuthError::MissingParam(param) => assert_eq!(param, "code"),
        other => panic!("expected MissingParam, got: {other}"),
    }
}

#[tokio::test]
async fn callback_missing_state_returns_error() {
    let (server, uri) = CallbackServer::bind().await.unwrap();
    let port = port_from_uri(&uri);

    let server_handle = tokio::spawn(async move {
        server
            .wait_for_callback(Duration::from_secs(5))
            .await
    });

    send_request(port, "/?code=abc123").await;

    let err = server_handle.await.unwrap().unwrap_err();
    match err {
        AuthError::MissingParam(param) => assert_eq!(param, "state"),
        other => panic!("expected MissingParam, got: {other}"),
    }
}

#[tokio::test]
async fn callback_timeout() {
    let (server, _uri) = CallbackServer::bind().await.unwrap();

    let timeout = Duration::from_millis(100);
    let err = server.wait_for_callback(timeout).await.unwrap_err();
    match err {
        AuthError::Timeout(d) => assert_eq!(d, timeout),
        other => panic!("expected Timeout, got: {other}"),
    }
}
