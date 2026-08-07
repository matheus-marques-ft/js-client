use std::collections::HashMap;
use std::sync::RwLock;

const CURRENT_SESSION_KEY: &str = "current_session_key";

#[derive(Default)]
pub struct ApiSessionStore {
    values: RwLock<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct ApiSessionContext {
    pub origin: String,
    pub bearer_token: String,
    pub org_id: String,
}

impl ApiSessionStore {
    /// Set the current site, account, token, and organization context
    pub fn set_current_session(
        &self,
        session_key: String,
        origin: String,
        bearer_token: String,
        org_id: String,
    ) {
        let mut values = self.values.write().expect("api session lock poisoned");

        values.insert(CURRENT_SESSION_KEY.to_string(), session_key.clone());
        values.insert(Self::field_key(&session_key, "origin"), origin);
        values.insert(Self::field_key(&session_key, "bearer_token"), bearer_token);
        values.insert(Self::field_key(&session_key, "org_id"), org_id);
    }

    /// Update the organization ID used by the current session
    pub fn set_current_org(&self, org_id: String) -> Result<(), String> {
        let mut values = self.values.write().expect("api session lock poisoned");
        let session_key = values
            .get(CURRENT_SESSION_KEY)
            .cloned()
            .ok_or_else(|| "missing current api session".to_string())?;

        values.insert(Self::field_key(&session_key, "org_id"), org_id);
        Ok(())
    }

    /// Update the bearer token stored by the current session
    pub fn update_current_bearer_token(&self, bearer_token: String) -> Result<(), String> {
        let mut values = self.values.write().expect("api session lock poisoned");
        let session_key = values
            .get(CURRENT_SESSION_KEY)
            .cloned()
            .ok_or_else(|| "missing current api session".to_string())?;

        values.insert(Self::field_key(&session_key, "bearer_token"), bearer_token);
        Ok(())
    }

    /// Read the current session context; returns None if any key field is missing
    pub fn current_context(&self) -> Option<ApiSessionContext> {
        let values = self.values.read().expect("api session lock poisoned");
        let session_key = values.get(CURRENT_SESSION_KEY)?.clone();

        Some(ApiSessionContext {
            origin: values
                .get(&Self::field_key(&session_key, "origin"))?
                .clone(),
            bearer_token: values
                .get(&Self::field_key(&session_key, "bearer_token"))?
                .clone(),
            org_id: values
                .get(&Self::field_key(&session_key, "org_id"))?
                .clone(),
        })
    }

    /// Generate the internal HashMap key for a session field
    fn field_key(session_key: &str, field: &str) -> String {
        format!("session:{}:{}", session_key, field)
    }
}
