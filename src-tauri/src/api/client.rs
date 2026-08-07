use reqwest::{redirect, Client, ClientBuilder};
use std::net::IpAddr;
use url::Url;

/// Build the default HTTP client used for JumpServer API requests
///
/// Deliberately accepts invalid certificates, since existing code needs to support self-signed or other non-public certificates
pub(crate) fn api_client() -> Result<Client, reqwest::Error> {
    instance_client_builder().build()
}

/// Build an HTTP client bound to the given site origin.
///
/// For local loopback addresses like localhost / 127.0.0.1 / ::1, explicitly bypass
/// the system proxy, to avoid local proxy software intercepting the request and
/// breaking local dev site requests.
pub(crate) fn api_client_for_origin(origin: &str) -> Result<Client, reqwest::Error> {
    instance_client_builder_for_origin(origin).build()
}

/// Build an HTTP client for OAuth requests that doesn't allow redirects.
///
/// OAuth code exchange currently needs redirect handling disabled, while still
/// keeping the same certificate behavior as regular API requests
pub(crate) fn oauth_client() -> Result<Client, reqwest::Error> {
    instance_client_builder()
        .redirect(redirect::Policy::none())
        .build()
}

/// Build an OAuth HTTP client bound to the given site origin.
///
/// OAuth code/token exchange also needs to bypass the proxy for local loopback addresses.
pub(crate) fn oauth_client_for_origin(origin: &str) -> Result<Client, reqwest::Error> {
    instance_client_builder_for_origin(origin)
        .redirect(redirect::Policy::none())
        .build()
}

fn instance_client_builder() -> ClientBuilder {
    Client::builder().danger_accept_invalid_certs(true)
}

fn instance_client_builder_for_origin(origin: &str) -> ClientBuilder {
    let builder = instance_client_builder();

    if should_bypass_proxy(origin) {
        return builder.no_proxy();
    }

    builder
}

fn should_bypass_proxy(origin: &str) -> bool {
    let Ok(url) = Url::parse(origin) else {
        return false;
    };

    let Some(host) = url.host_str() else {
        return false;
    };

    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    host.parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}
