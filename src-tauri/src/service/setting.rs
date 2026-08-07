use crate::api::{
    endpoint,
    request::{ApiRequestClient, ApiResponse},
};

pub struct SettingService {
    api: ApiRequestClient,
}

impl SettingService {
    /// Create the settings service, reusing the API client the command layer built from the current session
    pub fn new(api: ApiRequestClient) -> Self {
        Self { api }
    }

    /// Get user settings under the Luna category
    pub async fn get_setting(&self) -> ApiResponse {
        let url = self.api.endpoint(endpoint::user::LUNA_PREFERENCE);
        self.api.get_with_response(&url).await
    }
}
