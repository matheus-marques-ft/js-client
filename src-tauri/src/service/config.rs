use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::Manager;

use crate::service::plugin::PluginService;

pub struct ConfigService;

impl ConfigService {
    /// Get the config.json path inside the user config directory
    fn get_user_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        // Use the system config directory + the custom app name "jumpserver-client"
        let config_dir = app
            .path()
            .config_dir()
            .map_err(|e| format!("Failed to get config directory: {}", e))?
            .join("jumpserver-client");

        // Make sure the config directory exists
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
            log::info!("Created config directory: {:?}", config_dir);
        }

        Ok(config_dir.join("config.json"))
    }

    /// Get the config.json path inside the resource directory (used as the default template)
    fn resolve_resource_path(app: &tauri::AppHandle) -> Option<PathBuf> {
        app.path()
            .resolve(
                "resources/bin/config.json",
                tauri::path::BaseDirectory::Resource,
            )
            .ok()
            .filter(|p| p.is_file())
    }

    /// Config path used in the dev environment
    fn resolve_dev_path() -> Option<PathBuf> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        log::info!("Current working directory: {:?}", cwd);

        let candidates = [
            cwd.join("resources/bin/config.json"),
            cwd.join("../config.json"),
            cwd.join("../../config.json"),
            cwd.join("../../../config.json"),
        ];
        let result = candidates.into_iter().find(|p| p.is_file());
        log::info!("Selected dev config path: {:?}", result);
        result
    }

    /// Get the config version number
    fn get_config_version(config: &Value) -> i64 {
        config.get("version").and_then(|v| v.as_i64()).unwrap_or(1)
    }

    /// Merge custom settings from the user's config items (e.g. match_first)
    fn merge_app_items(user_items: &Value, default_items: &Value) -> Value {
        if !user_items.is_array() || !default_items.is_array() {
            return default_items.clone();
        }

        let user_arr = user_items.as_array().unwrap();
        let default_arr = default_items.as_array().unwrap();

        let mut result = default_arr.clone();

        // Walk the user config, keeping custom fields like the user's match_first
        for user_item in user_arr {
            if let Some(user_name) = user_item.get("name").and_then(|v| v.as_str()) {
                // Look up the matching item in the default config
                for result_item in result.iter_mut() {
                    if let Some(result_name) = result_item.get("name").and_then(|v| v.as_str()) {
                        if result_name == user_name {
                            // Keep the user's match_first setting
                            if let Some(match_first) = user_item.get("match_first") {
                                result_item
                                    .as_object_mut()
                                    .unwrap()
                                    .insert("match_first".to_string(), match_first.clone());
                            }
                            // Keep the user's path setting (if they customized the path)
                            if let Some(user_path) = user_item.get("path") {
                                if let Some(user_path_str) = user_path.as_str() {
                                    if !user_path_str.is_empty() {
                                        // Check whether the default config's path differs
                                        let default_path = result_item
                                            .get("path")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        // Keep it if the user set a different path
                                        if user_path_str != default_path {
                                            result_item
                                                .as_object_mut()
                                                .unwrap()
                                                .insert("path".to_string(), user_path.clone());
                                        }
                                    }
                                }
                            }
                            // Keep the user's is_set state
                            if let Some(is_set) = user_item.get("is_set") {
                                result_item
                                    .as_object_mut()
                                    .unwrap()
                                    .insert("is_set".to_string(), is_set.clone());
                            }
                            break;
                        }
                    }
                }
            }
        }

        Value::Array(result)
    }

    /// Merge OS-level config
    fn merge_os_config(user_os: &Value, default_os: &Value) -> Value {
        if !user_os.is_object() || !default_os.is_object() {
            return default_os.clone();
        }

        let mut result = default_os.clone();
        let result_obj = result.as_object_mut().unwrap();

        // Merge each category (terminal, remotedesktop, filetransfer, databases)
        for category in ["terminal", "remotedesktop", "filetransfer", "databases"] {
            if let (Some(user_items), Some(default_items)) =
                (user_os.get(category), default_os.get(category))
            {
                let merged = Self::merge_app_items(user_items, default_items);
                result_obj.insert(category.to_string(), merged);
            }
        }

        result
    }

    /// Merge config files, keeping the user's custom settings
    fn merge_configs(user_config: Value, default_config: Value) -> Value {
        let default_version = Self::get_config_version(&default_config);

        let mut merged = default_config.clone();
        let merged_obj = merged.as_object_mut().unwrap();

        // Update the version number to the default config's version
        merged_obj.insert("version".to_string(), json!(default_version));

        // Keep core structural fields (from the default config)
        for key in ["filename", "windowBounds", "defaultSetting"] {
            if let Some(value) = default_config.get(key) {
                merged_obj.insert(key.to_string(), value.clone());
            }
        }

        // Merge each OS's config
        for os_key in ["windows", "macos", "linux"] {
            if let (Some(user_os), Some(default_os)) =
                (user_config.get(os_key), default_config.get(os_key))
            {
                let merged_os = Self::merge_os_config(user_os, default_os);
                merged_obj.insert(os_key.to_string(), merged_os);
            } else if let Some(default_os) = default_config.get(os_key) {
                // If the user config doesn't have this OS, use the default config
                merged_obj.insert(os_key.to_string(), default_os.clone());
            }
        }

        merged
    }

    /// Update the user config (if the default config's version is newer)
    fn update_user_config_if_needed(
        user_config_path: &PathBuf,
        default_config_path: &PathBuf,
    ) -> Result<(), String> {
        // Read the default config
        let default_content = std::fs::read_to_string(default_config_path)
            .map_err(|e| format!("Failed to read default config: {}", e))?;
        let default_config: Value = serde_json::from_str(&default_content)
            .map_err(|e| format!("Failed to parse default config: {}", e))?;

        // Read the user config
        let user_content = std::fs::read_to_string(user_config_path)
            .map_err(|e| format!("Failed to read user config: {}", e))?;
        let user_config: Value = serde_json::from_str(&user_content)
            .map_err(|e| format!("Failed to parse user config: {}", e))?;

        let default_version = Self::get_config_version(&default_config);
        let user_version = Self::get_config_version(&user_config);
        let should_migrate_plugins =
            default_config.get("_plugins").is_some() && user_config.get("_plugins").is_none();

        log::info!(
            "Config versions - User: {}, Default: {}",
            user_version,
            default_version
        );

        // Merge configs if the default config's version is newer, or if this version introduces plugin config.
        if default_version > user_version || should_migrate_plugins {
            log::info!(
                "Upgrading config from version {} to {} (migrate_plugins={})",
                user_version,
                default_version,
                should_migrate_plugins
            );

            let merged_config = Self::merge_configs(user_config, default_config);

            // Write the merged config
            let pretty = serde_json::to_string_pretty(&merged_config)
                .map_err(|e| format!("Failed to serialize merged config: {}", e))?;
            std::fs::write(user_config_path, pretty)
                .map_err(|e| format!("Failed to write merged config: {}", e))?;

            log::info!(
                "Config upgraded successfully to version {}",
                default_version
            );
        } else {
            log::info!("User config is up to date, no upgrade needed");
        }

        Ok(())
    }

    /// Make sure the user config file exists; copy it from the template if it doesn't
    fn ensure_user_config(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        let user_config_path = Self::get_user_config_path(app)?;
        let template_path = Self::resolve_resource_path(app)
            .or_else(Self::resolve_dev_path)
            .ok_or_else(|| "config.json template not found (resource/dev)".to_string())?;

        // Copy from the template if the user config file doesn't exist
        if !user_config_path.exists() {
            log::info!(
                "Copying config template from {:?} to {:?}",
                template_path,
                user_config_path
            );
            std::fs::copy(&template_path, &user_config_path)
                .map_err(|e| format!("Failed to copy config template: {}", e))?;
            log::info!("Initial config created successfully");
        } else {
            // If the user config already exists, check whether it needs an update
            log::info!(
                "User config exists at {:?}, checking for updates",
                user_config_path
            );
            if let Err(e) = Self::update_user_config_if_needed(&user_config_path, &template_path) {
                log::warn!("Failed to update user config: {}", e);
                // Don't block the flow; keep using the existing config even if the update fails
            }
        }

        Ok(user_config_path)
    }

    pub fn get_app_config(app: &tauri::AppHandle) -> Result<Value, String> {
        let path = Self::ensure_user_config(app)?;

        log::info!("Reading config from: {:?}", path);

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("read config.json failed: {}", e))?;
        let json: Value = serde_json::from_str(&content)
            .map_err(|e| format!("parse config.json failed: {}", e))?;

        if PluginService::is_plugins_enabled(&json) {
            let config_dir = path
                .parent()
                .ok_or_else(|| "invalid config directory".to_string())?;
            return PluginService::build_app_config(app, config_dir);
        }

        let os_key = match std::env::consts::OS {
            "macos" => "macos",
            "windows" => "windows",
            "linux" => "linux",
            other => other,
        };

        let per_os = json
            .get(os_key)
            .cloned()
            .ok_or_else(|| format!("config.json missing key for current OS: {}", os_key))?;

        Ok(per_os)
    }

    pub fn update_selection(
        app: &tauri::AppHandle,
        category: &str,
        protocol: &str,
        name: &str,
        new_path: Option<String>,
        enabled: bool,
    ) -> Result<Value, String> {
        let config_path = Self::ensure_user_config(app)?;

        log::info!("Updating config at: {:?}", config_path);

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("read config.json failed: {}", e))?;
        let json: Value = serde_json::from_str(&content)
            .map_err(|e| format!("parse config.json failed: {}", e))?;

        if PluginService::is_plugins_enabled(&json) {
            let config_dir = config_path
                .parent()
                .ok_or_else(|| "invalid config directory".to_string())?;
            return PluginService::update_selection(
                app, config_dir, category, protocol, name, new_path, enabled,
            );
        }

        let mut json = json;
        let os_key = match std::env::consts::OS {
            "macos" => "macos",
            "windows" => "windows",
            "linux" => "linux",
            other => other,
        };

        let arr = json
            .get_mut(os_key)
            .and_then(|os| os.get_mut(category))
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| format!("invalid config path: {}.{}", os_key, category))?;

        // If a path was passed in, only update that item's path and is_set, without changing match_first
        if let Some(p) = new_path.clone() {
            let trimmed = p.trim().to_string();
            if !trimmed.is_empty() {
                let mut found = false;
                for item in arr.iter_mut() {
                    let item_name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    if item_name == name {
                        found = true;
                        // Update the path
                        item.as_object_mut()
                            .unwrap()
                            .insert("path".into(), Value::String(trimmed.clone()));
                        // Mark as set
                        item.as_object_mut()
                            .unwrap()
                            .insert("is_set".into(), Value::Bool(true));
                        break;
                    }
                }

                if !found {
                    return Err(format!(
                        "selected item '{}' not found under {}.{}",
                        name, os_key, category
                    ));
                }

                let pretty = serde_json::to_string_pretty(&json)
                    .map_err(|e| format!("serialize config.json failed: {}", e))?;
                std::fs::write(&config_path, pretty)
                    .map_err(|e| format!("write config.json failed: {}", e))?;

                log::info!("Config path updated successfully at: {:?}", config_path);

                return Ok(json.get(os_key).cloned().ok_or_else(|| {
                    format!("config.json missing key for current OS: {}", os_key)
                })?);
            }
        }

        let mut found_target = arr
            .iter()
            .any(|item| item.get("name").and_then(|v| v.as_str()).unwrap_or("") == name);

        if enabled {
            for item in arr.iter() {
                let item_name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if item_name != name {
                    continue;
                }

                found_target = true;
                let is_internal = item
                    .get("is_internal")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if !is_internal {
                    let path = item
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if path.is_empty() || !std::path::Path::new(path).is_file() {
                        return Err(format!("executable not found: {}", path));
                    }
                }
                break;
            }
        }

        for item in arr.iter_mut() {
            if let Some(mf) = item.get_mut("match_first") {
                if let Some(list) = mf.as_array_mut() {
                    list.retain(|v| v.as_str().map(|s| s != protocol).unwrap_or(true));
                }
            }
        }

        if enabled {
            for item in arr.iter_mut() {
                let item_name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if item_name == name {
                    if !item.get("match_first").is_some() {
                        item.as_object_mut()
                            .unwrap()
                            .insert("match_first".into(), Value::Array(vec![]));
                    }
                    if let Some(list) = item.get_mut("match_first").and_then(|v| v.as_array_mut()) {
                        list.push(Value::String(protocol.to_string()));
                    }
                    break;
                }
            }
        }

        if !found_target {
            return Err(format!(
                "selected item '{}' not found under {}.{}",
                name, os_key, category
            ));
        }

        let pretty = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("serialize config.json failed: {}", e))?;
        std::fs::write(&config_path, pretty)
            .map_err(|e| format!("write config.json failed: {}", e))?;

        log::info!("Config updated successfully at: {:?}", config_path);

        Ok(json
            .get(os_key)
            .cloned()
            .ok_or_else(|| format!("config.json missing key for current OS: {}", os_key))?)
    }
}
