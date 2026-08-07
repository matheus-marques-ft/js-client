use crate::api::{
    endpoint,
    request::{ApiRequestClient, ApiResponse},
};

pub struct UserService {
    api: ApiRequestClient,
}

impl UserService {
    /// Create the user service; no organization context is carried during the login stage
    pub fn new(origin: String, bearer_token: String) -> Result<Self, reqwest::Error> {
        Ok(Self {
            api: ApiRequestClient::with_origin(origin, bearer_token, String::new())?,
        })
    }

    /// Get the current user's profile
    pub async fn get_user_profile(&self) -> ApiResponse {
        let url = self.api.endpoint(endpoint::user::PROFILE);
        log::info!("Fetching user profile info: {}", url);
        self.api.get_with_response(&url).await
    }

    /// Get the list of organizations the current user has permission to access
    pub async fn get_permission_orgs(&self) -> ApiResponse {
        let url = self.api.endpoint(endpoint::user::PROFILE_PERMISSIONS);
        log::info!("Fetching authorized organizations: {}", url);
        self.api.get_with_response(&url).await
    }

    /// Get the current organization's info
    pub async fn get_current_org(&self) -> ApiResponse {
        let url = self.api.endpoint(endpoint::org::CURRENT);
        log::info!("Fetching current organization info: {}", url);
        self.api.get_with_response(&url).await
    }

    /// Get X-Pack info from the public settings
    pub async fn get_xpack_message(&self) -> ApiResponse {
        let url = self.api.endpoint(endpoint::settings::PUBLIC);
        log::info!("Fetching current public info: {}", url);
        self.api.get_with_response(&url).await
    }
}
