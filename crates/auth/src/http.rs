//! oauth2 `AsyncHttpClient` adapter for the workspace's reqwest 0.13 client.
//!
//! oauth2 5.0's optional `reqwest` integration pins reqwest 0.12; enabling it
//! would vendor a second HTTP stack next to the workspace's 0.13. This adapter
//! (~40 lines) keeps a single stack. The client is a newtype because coherence
//! forbids implementing a foreign trait for `&reqwest::Client`.

use std::future::Future;
use std::pin::Pin;

use oauth2::{AsyncHttpClient, HttpClientError, HttpRequest, HttpResponse};

/// A reqwest client wrapper implementing oauth2's `AsyncHttpClient`.
#[derive(Debug, Clone)]
pub(crate) struct Oauth2HttpClient(pub(crate) reqwest::Client);

impl<'c> AsyncHttpClient<'c> for Oauth2HttpClient {
    type Error = HttpClientError<reqwest::Error>;

    type Future = Pin<Box<dyn Future<Output = Result<HttpResponse, Self::Error>> + Send + Sync + 'c>>;

    fn call(&'c self, request: HttpRequest) -> Self::Future {
        Box::pin(async move {
            let request = reqwest::Request::try_from(request).map_err(|error| HttpClientError::Other(error.to_string()))?;
            let response = self.0.execute(request).await.map_err(|error| HttpClientError::Reqwest(Box::new(error)))?;

            let mut builder = http::Response::builder().status(response.status());
            for (name, value) in response.headers() {
                builder = builder.header(name, value);
            }

            let body = response.bytes().await.map_err(|error| HttpClientError::Reqwest(Box::new(error)))?;
            builder.body(body.to_vec()).map_err(|error| HttpClientError::Other(error.to_string()))
        })
    }
}
