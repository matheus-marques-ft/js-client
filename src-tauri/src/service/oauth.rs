use crate::api::client::oauth_client;
use crate::api::endpoint;
use crate::service::token::TokenService;
use anyhow::Result;
use chrono::{Duration, Utc};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, EndpointNotSet,
    EndpointSet, ErrorResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken,
    RequestTokenError, RevocationUrl, Scope, StandardRevocableToken, TokenResponse, TokenUrl,
};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Mutex;
use tokio::sync::oneshot;
use url::Url;

pub type JumpServerOAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

/// Data for the OAuth authorization request stage.
///
/// Returned to the command layer after the authorization URL is created:
/// - `auth_url` is sent to the frontend to open the browser
/// - `pkce_verifier` and `csrf` are stashed in `AuthFlowState`, awaiting validation and token exchange at callback time
pub struct OAuthAuthorizationRequest {
    pub auth_url: String,
    pub pkce_verifier: PkceCodeVerifier,
    pub csrf: CsrfToken,
}

/// An OAuth authorization flow that's been registered and is awaiting its callback.
///
/// Once the command layer has this, it sends `auth_url` to the frontend first, then
/// awaits `callback_rx` to receive the authorization code parsed from a deep link or
/// dev HTTP callback.
pub struct PendingAuthorization {
    pub auth_url: String,
    pub callback_rx: oneshot::Receiver<CallbackParams>,
}

/// Result of an OAuth token exchange or refresh.
///
/// Generated both on successful login and during the refresh-token stage, then
/// written to local token storage.
pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

/// Parameters parsed out during the OAuth callback stage.
///
/// `code` is used to exchange for a token; `state`, `pkce_verifier`, and `csrf` are
/// used to complete CSRF and PKCE validation.
pub struct CallbackParams {
    pub code: AuthorizationCode,
    pub state: Option<String>,
    pub pkce_verifier: PkceCodeVerifier,
    pub csrf: CsrfToken,
}

/// JumpServer OAuth server configuration.
///
/// Fetched from the well-known endpoint at the start of login; only `client_id` is
/// really used right now, the other fields are kept to describe server capabilities
/// and for future compatibility.
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct OAuthConfig {
    pub issuer: String,
    pub client_id: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub revocation_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub revocation_endpoint_auth_methods_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub response_modes_supported: Vec<String>,
    pub token_expires_in: i64,
    pub refresh_token_expires_in: i64,
}

/// OAuth context stashed in memory after login is initiated.
///
/// This is an internal structure that only exists while waiting for the callback;
/// once the callback arrives it's taken out and sent to the waiting login task.
struct PendingAuth {
    pkce_verifier: PkceCodeVerifier,
    csrf: CsrfToken,
    tx: oneshot::Sender<CallbackParams>,
}

/// Shared state for the OAuth login flow.
///
/// Managed by Tauri; both the deep link and the dev HTTP callback hand the
/// authorization result back to the waiting `auth_login` through this.
#[derive(Default)]
pub struct AuthFlowState {
    pending: Mutex<Option<PendingAuth>>,
}

impl OAuthTokenSet {
    /// Write the OAuth token to local secure storage.
    pub async fn persist(&self, site: &str, client_id: &str) -> Result<()> {
        let token_service = TokenService::new(site.to_string());

        token_service
            .persist(
                &self.access_token,
                self.refresh_token.as_deref(),
                self.expires_at,
                Some(client_id),
            )
            .await
    }
}

impl AuthFlowState {
    /// Called to wake up the login flow once a deep link or dev HTTP callback has parsed a code/state.
    pub fn handle_callback(&self, raw_url: &str) {
        let Ok(url) = Url::parse(raw_url) else {
            return;
        };

        let mut code = None;
        let mut state = None;

        for (key, value) in url.query_pairs() {
            // query_pairs returns Cow<str>; convert to &str here for easy matching against string literals.
            match key.as_ref() {
                "code" => code = Some(value.to_string()),
                "state" => state = Some(value.to_string()),
                _ => {}
            }
        }

        let Some(code) = code else {
            return;
        };

        if let Ok(mut guard) = self.pending.lock() {
            if let Some(pending) = guard.take() {
                let _ = pending.tx.send(CallbackParams {
                    code: AuthorizationCode::new(code),
                    state,
                    pkce_verifier: pending.pkce_verifier,
                    csrf: pending.csrf,
                });
            }
        }
    }

    /// Register an OAuth authorization flow that is now awaiting its callback.
    pub fn register_authorization(
        &self,
        authorize: OAuthAuthorizationRequest,
    ) -> PendingAuthorization {
        let (tx, callback_rx) = oneshot::channel();

        {
            let mut guard = self.pending.lock().expect("lock poisoned");
            *guard = Some(PendingAuth {
                pkce_verifier: authorize.pkce_verifier,
                csrf: authorize.csrf,
                tx,
            })
        }

        PendingAuthorization {
            auth_url: authorize.auth_url,
            callback_rx,
        }
    }

    /// Cancel the OAuth authorization flow currently awaiting its callback.
    pub fn cancel(&self) {
        if let Ok(mut guard) = self.pending.lock() {
            let _ = guard.take();
        }
    }
}

/// Fetch the OAuth server configuration from JumpServer.
pub async fn fetch_oauth_config(site: &str, client: &Client) -> Result<OAuthConfig> {
    let config_url = format!("{}{}", site, endpoint::oauth::WELL_KNOWN);

    let response = client.get(config_url).send().await?;
    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        anyhow::bail!("OAuth config endpoint returned {}: {}", status, text);
    }

    let config = serde_json::from_str::<OAuthConfig>(&text)?;

    Ok(config)
}

/// Build the JumpServer OAuth client.
pub fn build_oauth_client(site: &str, client_id: &str) -> Result<JumpServerOAuthClient> {
    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        // Set the authorization endpoint: the user gets redirected here to log in and authorize
        .set_auth_uri(AuthUrl::new(format!(
            "{}{}",
            site,
            endpoint::oauth::AUTHORIZE
        ))?)
        // Set the token endpoint: used afterward to exchange a code or refresh_token for an access_token.
        .set_token_uri(TokenUrl::new(format!(
            "{}{}",
            site,
            endpoint::oauth::TOKEN
        ))?)
        // Debug mode uses the local HTTP callback, Release mode uses the deep link callback.
        .set_redirect_uri(RedirectUrl::new(oauth_redirect_uri().to_string())?);

    Ok(client)
}

/// Create the OAuth authorization request, generating the authorization URL, PKCE verifier, and CSRF token.
pub fn create_authorization_request(client: &JumpServerOAuthClient) -> OAuthAuthorizationRequest {
    // Generate PKCE
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the authorization URL
    let (auth_url, csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("write".to_string()))
        .add_scope(Scope::new("read".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    OAuthAuthorizationRequest {
        auth_url: auth_url.to_string(),
        pkce_verifier,
        csrf,
    }
}

fn format_token_exchange_error<E, T>(error: &RequestTokenError<E, T>) -> String
where
    E: std::error::Error,
    T: std::fmt::Debug + ErrorResponse,
{
    match error {
        RequestTokenError::ServerResponse(resp) => {
            format!("Server returned error response: {:?}", resp)
        }
        RequestTokenError::Request(req_err) => {
            format!("Token exchange request failed: {}", req_err)
        }
        RequestTokenError::Parse(parse_err, body) => {
            let body_text = String::from_utf8_lossy(body);
            format!(
                "Failed to parse server response: {}; raw body: {}",
                parse_err, body_text
            )
        }
        RequestTokenError::Other(msg) => format!("Token exchange error: {}", msg),
    }
}

fn log_token_exchange_error<E, T>(error: &RequestTokenError<E, T>)
where
    E: std::error::Error,
    T: std::fmt::Debug + ErrorResponse,
{
    log::error!("{}", format_token_exchange_error(error));
}

/// Exchange the code + PKCE verifier from an OAuth callback for a token.
pub async fn exchange_authorization_code(
    client: &JumpServerOAuthClient,
    http_client: &Client,
    callback: CallbackParams,
) -> Result<OAuthTokenSet> {
    // Validate state, to prevent CSRF.
    if let Some(state) = callback.state.as_ref() {
        if state != callback.csrf.secret() {
            anyhow::bail!("state mismatch");
        }
    }

    let token_result = client
        .exchange_code(callback.code)
        .set_pkce_verifier(callback.pkce_verifier)
        .request_async(http_client)
        .await
        .inspect_err(log_token_exchange_error)
        .map_err(|error| anyhow::anyhow!(format_token_exchange_error(&error)))?;

    let access_token = token_result.access_token().secret().to_owned();
    let refresh_token = token_result
        .refresh_token()
        .map(|token| token.secret().to_owned());
    let expires_at = expires_at_timestamp(token_result.expires_in());

    Ok(OAuthTokenSet {
        access_token,
        refresh_token,
        expires_at,
    })
}

/// Revoke and delete the locally stored OAuth token.
pub async fn revoke_and_clear_tokens(site: &str) -> Result<()> {
    let token_service = TokenService::new(site.to_string());

    if let Some(entry) = token_service.load().await? {
        if let Some(refresh_token) = entry.refresh_token {
            let client_id = entry.client_id.unwrap_or_default();
            let http_client = oauth_client()?;

            if let Err(error) =
                revoke_refresh_token(&site, &client_id, &refresh_token, &http_client).await
            {
                log::error!("revocation request failed: {}", error);
            }
        }

        token_service.delete().await?
    }

    Ok(())
}

/// Ensure an access_token is available; if it's about to expire, refresh it with the refresh_token and write it back to local storage.
pub async fn ensure_fresh_token(site: &str, provided: Option<&str>) -> Result<String> {
    let token_service = TokenService::new(site.to_string());
    let entry = token_service.load().await?;

    let stored_access = entry.as_ref().map(|token| token.access_token.clone());
    let stored_refresh = entry.as_ref().and_then(|token| token.refresh_token.clone());
    let expires_at = entry.as_ref().and_then(|token| token.expires_at);
    let client_id = entry
        .as_ref()
        .and_then(|token| token.client_id.clone())
        .unwrap_or_default();

    let mut access = stored_access.or_else(|| provided.map(str::to_string));

    if should_refresh_token(expires_at) {
        let refresh_token = stored_refresh
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("refresh_token missing for site {}", site))?;

        let http_client = oauth_client()?;
        let tokens = refresh_access_token(site, &client_id, refresh_token, &http_client).await?;

        tokens.persist(site, &client_id).await?;

        access = Some(tokens.access_token);
    }

    access.ok_or_else(|| anyhow::anyhow!("no access token available for site {}", site))
}

/// Check whether the token needs a preemptive refresh.
fn should_refresh_token(expires_at: Option<i64>) -> bool {
    expires_at
        .map(|timestamp| timestamp <= Utc::now().timestamp() + 60)
        .unwrap_or(false)
}

/// Convert the expiry duration returned by OAuth into a local timestamp.
fn expires_at_timestamp(expires_in: Option<std::time::Duration>) -> Option<i64> {
    expires_in
        .map(|duration| {
            Utc::now() + Duration::from_std(duration).unwrap_or_else(|_| Duration::seconds(0))
        })
        .map(|datetime| datetime.timestamp())
}

/// Return the OAuth redirect_uri based on the current run mode.
fn oauth_redirect_uri() -> &'static str {
    if cfg!(debug_assertions) {
        "http://127.0.0.1:14876/auth/callback"
    } else {
        "jms://auth/callback"
    }
}

/// Refresh the access_token using the refresh_token.
async fn refresh_access_token(
    site: &str,
    client_id: &str,
    refresh_token: &str,
    http_client: &Client,
) -> Result<OAuthTokenSet> {
    let client = BasicClient::new(ClientId::new(client_id.to_string())).set_token_uri(
        TokenUrl::new(format!("{}{}", site, endpoint::oauth::TOKEN))?,
    );

    let token_result = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(http_client)
        .await?;

    let access_token = token_result.access_token().secret().to_owned();
    let refresh_token = token_result
        .refresh_token()
        .map(|token| token.secret().to_owned())
        .unwrap_or_else(|| refresh_token.to_string());
    let expires_at = expires_at_timestamp(token_result.expires_in());

    Ok(OAuthTokenSet {
        access_token,
        refresh_token: Some(refresh_token),
        expires_at,
    })
}

/// Send a revocation request to the server using the refresh_token.
async fn revoke_refresh_token(
    site: &str,
    client_id: &str,
    refresh_token: &str,
    http_client: &Client,
) -> Result<()> {
    let client = BasicClient::new(ClientId::new(client_id.to_string())).set_revocation_url(
        RevocationUrl::new(format!("{}{}", site, endpoint::oauth::REVOKE))?,
    );

    let request = client
        .revoke_token(StandardRevocableToken::RefreshToken(RefreshToken::new(
            refresh_token.to_string(),
        )))
        .map_err(|error| anyhow::anyhow!("build revocation request failed: {}", error))?;

    request.request_async(http_client).await?;

    Ok(())
}
