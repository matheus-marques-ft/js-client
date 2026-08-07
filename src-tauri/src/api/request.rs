use crate::{
    api::{
        client::api_client_for_origin,
        context::{apply_org_header, ApiContext},
        response::into_api_response,
    },
    utils::tz_offset_string,
};
use log::info;
use reqwest::{header::AUTHORIZATION, Client, Method, RequestBuilder, Response};
use serde::Serialize;
use url::Url;

pub(crate) use crate::api::response::ApiResponse;
use crate::api::session::ApiSessionContext;

pub struct ApiRequestClient {
    client: Client,
    origin: String,
    bearer_token: String,
    org_id: String,
}

impl ApiRequestClient {
    /// Create an API client bound to the site origin, so endpoints can later be concatenated directly into full URLs
    pub fn with_origin(
        origin: String,
        bearer_token: String,
        org_id: String,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: api_client_for_origin(&origin)?,
            origin,
            bearer_token,
            org_id,
        })
    }

    /// Create a request client from the current API session context
    pub fn from_session(context: &ApiSessionContext) -> Result<Self, reqwest::Error> {
        Self::with_origin(
            context.origin.clone(),
            context.bearer_token.clone(),
            context.org_id.clone(),
        )
    }

    /// Concatenate a centrally-defined API path into a full URL under the current site
    pub fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.origin.trim_end_matches('/'), path)
    }

    /// Send a GET request and convert it into the unified ApiResponse
    pub async fn get_with_response(&self, url: &str) -> ApiResponse {
        info!("GET {}", url);

        self.send_with_response(Method::GET, url, |request| request)
            .await
    }

    /// Send a GET request with query parameters and convert it into the unified ApiResponse
    pub async fn get_with_query_response<T>(&self, url: &str, query: &T) -> ApiResponse
    where
        T: Serialize + ?Sized,
    {
        info!("GET WITH QUERY {}", url);
        self.send_with_response(Method::GET, url, |request| request.query(query))
            .await
    }

    /// Send a JSON POST request and convert it into the unified ApiResponse
    pub async fn post_json_with_response<T>(&self, url: &str, body: &T) -> ApiResponse
    where
        T: Serialize + ?Sized,
    {
        info!("POST WITH BODY {}", url);
        log_json_body(body);

        self.send_with_response(Method::POST, url, |request| request.json(body))
            .await
    }

    /// Send a DELETE request and convert it into the unified ApiResponse
    pub async fn delete_with_response(&self, url: &str) -> ApiResponse {
        info!("DELETE {}", url);
        self.send_with_response(Method::DELETE, url, |request| request)
            .await
    }

    /// Build and execute the underlying reqwest request
    async fn send<F>(&self, method: Method, url: &str, apply: F) -> Result<Response, reqwest::Error>
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder,
    {
        let request = apply(self.base_request(method, url)).build()?;
        self.client.execute(request).await // execute sends the already-built request
    }

    /// Convert the token and organization info stored inside the client into a request context
    fn context(&self) -> ApiContext<'_> {
        ApiContext {
            bearer_token: &self.bearer_token,
            org_id: &self.org_id,
        }
    }

    /// Create a base request with common headers
    fn base_request(&self, method: Method, url: &str) -> RequestBuilder {
        let context = self.context();
        let mut request = self
            .client
            .request(method, url)
            .header("X-TZ", tz_offset_string());

        if !context.bearer_token.is_empty() {
            request = request.header(AUTHORIZATION, format!("Bearer {}", context.bearer_token));
        }

        if let Some(referer) = referer_from(url) {
            request = request.header("Referer", referer);
        }

        if context.org_id.is_empty() {
            request
        } else {
            apply_org_header(request, &context)
        }
    }

    /// Execute the request and convert it into the unified response structure
    async fn send_with_response<F>(&self, method: Method, url: &str, apply: F) -> ApiResponse
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder,
    {
        into_api_response(url, self.send(method, url, apply).await).await
    }
}

/// Extract the Referer header value from a URL, keeping only the scheme, host, and port
fn referer_from(url: &str) -> Option<String> {
    Url::parse(url).ok().and_then(|url| match url.scheme() {
        "http" | "https" => {
            let host = url.host_str()?;
            let mut origin = format!("{}://{}", url.scheme(), host);

            if let Some(port) = url.port() {
                origin.push(':');
                origin.push_str(&port.to_string());
            }

            Some(origin)
        }
        _ => None,
    })
}

/// Log the request body content
fn log_json_body<T>(body: &T)
where
    T: Serialize + ?Sized,
{
    if let Ok(body) = serde_json::to_string(body) {
        info!("request body: {}", body);
    }
}
