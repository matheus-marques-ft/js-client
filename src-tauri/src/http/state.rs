use tauri::AppHandle;

#[derive(Clone)]
pub(crate) struct HttpServerState {
    app_handle: AppHandle,
}

// The app_handle is registered as shared state because Auth data needs to be pulled from it.
// When an axum route is called over HTTP from the browser, it only gets the HTTP request context — it doesn't automatically have the Tauri context.
// So the AppHandle must be deliberately put into axum's State.
impl HttpServerState {
    pub(crate) fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub(crate) fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }
}
