use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use crate::error::{AuthError, Result};

const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Authentication Complete</title></head>
<body style="font-family:system-ui,sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#1a1a2e;color:#e0e0e0">
<div style="text-align:center">
<h1>Authentication Successful</h1>
<p>You may close this tab and return to the launcher.</p>
<script>setTimeout(()=>window.close(),2000)</script>
</div>
</body>
</html>"#;

const MAX_REQUEST_SIZE: usize = 8192;

/// A temporary localhost TCP server that captures a single OAuth redirect callback.
pub struct CallbackServer {
    listener: TcpListener,
    redirect_uri: String,
}

impl CallbackServer {
    /// Bind to an OS-assigned ephemeral port on localhost.
    /// Returns the server and the redirect URI to use in the OAuth URL.
    pub async fn bind() -> Result<(Self, String)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://localhost:{port}");
        log::debug!("callback server listening on port {port}");
        let uri_clone = redirect_uri.clone();
        Ok((Self { listener, redirect_uri }, uri_clone))
    }

    /// Bind to a specific port on localhost.
    pub async fn bind_to(port: u16) -> Result<(Self, String)> {
        let listener = TcpListener::bind(("127.0.0.1", port)).await?;
        let redirect_uri = format!("http://localhost:{port}");
        log::debug!("callback server listening on port {port}");
        let uri_clone = redirect_uri.clone();
        Ok((Self { listener, redirect_uri }, uri_clone))
    }

    /// Wait for the OAuth redirect, extract the authorization code and state,
    /// send a success response to the browser, then shut down.
    pub async fn wait_for_callback(self, timeout: Duration) -> Result<(String, String)> {
        let result = tokio::time::timeout(timeout, self.accept_one()).await;
        match result {
            Ok(inner) => inner,
            Err(_) => Err(AuthError::Timeout(timeout)),
        }
    }

    async fn accept_one(self) -> Result<(String, String)> {
        let (mut stream, _addr) = self.listener.accept().await?;
        let mut buf = vec![0u8; MAX_REQUEST_SIZE];
        let mut total = 0;

        // Read until we see the end of HTTP headers or fill the buffer.
        loop {
            let n = stream.read(&mut buf[total..]).await?;
            if n == 0 {
                break;
            }
            total += n;
            if total >= MAX_REQUEST_SIZE || buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        let request = String::from_utf8_lossy(&buf[..total]);

        // Parse the request line: "GET /?code=...&state=... HTTP/1.1"
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .ok_or_else(|| AuthError::MissingParam("request path".into()))?;

        // Build a full URL so we can parse query params.
        let full_url = format!("{}{}", self.redirect_uri, path);
        let parsed = Url::parse(&full_url)?;

        // Check for OAuth error in the redirect.
        if let Some(error) = parsed.query_pairs().find(|(k, _)| k == "error") {
            let description = parsed
                .query_pairs()
                .find(|(k, _)| k == "error_description")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default();
            // Send a response before returning the error.
            let _ = send_response(&mut stream, "Authentication Failed", &description).await;
            return Err(AuthError::OAuth {
                error: error.1.into_owned(),
                description,
            });
        }

        let code = parsed
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.into_owned())
            .ok_or_else(|| AuthError::MissingParam("code".into()))?;

        let state = parsed
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.into_owned())
            .ok_or_else(|| AuthError::MissingParam("state".into()))?;

        // Send a friendly success page back to the browser.
        let _ = send_response_html(&mut stream, SUCCESS_HTML).await;

        Ok((code, state))
    }
}

async fn send_response_html(
    stream: &mut tokio::net::TcpStream,
    html: &str,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await
}

async fn send_response(
    stream: &mut tokio::net::TcpStream,
    title: &str,
    message: &str,
) -> std::io::Result<()> {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><title>{title}</title></head>
<body style="font-family:system-ui,sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#1a1a2e;color:#e0e0e0">
<div style="text-align:center">
<h1>{title}</h1>
<p>{message}</p>
</div>
</body>
</html>"#
    );
    send_response_html(stream, &html).await
}
