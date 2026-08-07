use crate::api::{
    endpoint,
    request::{ApiRequestClient, ApiResponse},
};

pub struct VersionService {
    api: ApiRequestClient,
}

impl VersionService {
    /// Create the version service; it hits a public endpoint and needs no token or organization context
    pub fn new(origin: String) -> Result<Self, reqwest::Error> {
        Ok(Self {
            api: ApiRequestClient::with_origin(origin, String::new(), String::new())?,
        })
    }

    /// Get client version info
    pub async fn get_version_message(&self) -> ApiResponse {
        let url = self.api.endpoint(endpoint::settings::CLIENT_VERSIONS);
        log::info!("Fetching current version info: {}", url);
        // This endpoint is public and doesn't require a bearer_token
        self.api.get_with_response(&url).await
    }
}
