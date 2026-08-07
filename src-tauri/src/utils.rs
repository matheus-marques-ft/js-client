use anyhow::Result;
use chrono::{Local, Offset};
use log::{error, warn};
use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};
use tauri_plugin_store::StoreExt;
use url::Url;

const MIN_WINDOW_WIDTH: f64 = 600.0;
const MIN_WINDOW_HEIGHT: f64 = 400.0;
const MAX_WINDOW_WIDTH: f64 = 1800.0;
const MAX_WINDOW_HEIGHT: f64 = 1000.0;
const WINDOW_SIZE_UNIT_LOGICAL: &str = "logical";

/// Check whether this is an OAuth callback deeplink
pub fn is_auth_callback(raw_url: &str) -> bool {
    if let Ok(url) = Url::parse(raw_url) {
        return url.scheme() == "jms"
            && url
                .host_str()
                .map(|h| h.eq_ignore_ascii_case("auth"))
                .unwrap_or(false)
            && url.path().starts_with("/callback");
    }
    false
}

/// Get the local timezone offset string
pub fn tz_offset_string() -> String {
    let local_offset = Local::now().offset().fix().local_minus_utc();
    let hours = local_offset / 3600;
    let minutes = (local_offset % 3600) / 60;

    format!("{:+03}:{:02}", hours, minutes)
}

/// Initialize and persist the window size (stored as logical DIP size), to avoid
/// visual size changes caused by cross-monitor scaling.
/// - Storage: logical pixel width/height (width/height, DIP)
///
/// - Restore:
///   set_size directly using the logical size; if only an old physical-pixel record
///   exists, convert it to logical size using the current scale before setting it.
///   Tauri/the underlying windowing system automatically converts the logical size
///   into physical pixels based on the current screen's scale factor
///
/// - Principle: logical pixels × scale factor = physical pixels; the scale factor
///   only matters at “size change / app open” time — what's stored in between is
///   always the logical size
pub fn setup_window_size_persistence(win: WebviewWindow) {
    // Restore the last saved size
    if let Err(e) = restore_window_size(&win) {
        warn!("restore_window_size failed: {}", e);
    }

    // Listen for window changes and record the DIP size
    let h = win.app_handle().clone();
    let win_for_events = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Resized(size) = event {
            // Skip size changes while minimized/maximized, to avoid storing an unreasonable size
            if win_for_events.is_minimized().ok().unwrap_or(false)
                || win_for_events.is_maximized().ok().unwrap_or(false)
            {
                return;
            }

            // Convert the physical size from the event into a logical size: logical = physical / scale_factor
            let factor = win_for_events.scale_factor().ok().unwrap_or(1.0);
            let width_logical = (size.width as f64 / factor).max(1.0);
            let height_logical = (size.height as f64 / factor).max(1.0);

            if width_logical < MIN_WINDOW_WIDTH || height_logical < MIN_WINDOW_HEIGHT {
                return;
            }

            if let Err(e) = save_window_logical_size(&h, width_logical, height_logical) {
                error!("save_window_size (logical) failed: {}", e);
            }
        }
    });
}

fn save_window_logical_size(app: &AppHandle, width: f64, height: f64) -> Result<(), String> {
    let store = app
        .store("app_data.json")
        .map_err(|e| format!("open store failed: {}", e))?;

    store.set(
        "window_size",
        serde_json::json!({
            "width": width,
            "height": height,
            "unit": WINDOW_SIZE_UNIT_LOGICAL,
        }),
    );

    store
        .save()
        .map_err(|e| format!("store save failed: {}", e))
}

fn restore_window_size(win: &WebviewWindow) -> Result<(), String> {
    let app: &AppHandle = win.app_handle();
    let store = app
        .store("app_data.json")
        .map_err(|e| format!("open store failed: {}", e))?;

    let Some(v) = store.get("window_size") else {
        return Ok(());
    };

    let factor = win
        .scale_factor()
        .map_err(|e| format!("scale_factor failed: {}", e))?;

    let is_logical = v
        .get("unit")
        .and_then(|x| x.as_str())
        .is_some_and(|unit| unit == WINDOW_SIZE_UNIT_LOGICAL);

    let (width_logical, height_logical) = if let (Some(width), Some(height)) = (
        v.get("width").and_then(|x| x.as_f64()),
        v.get("height").and_then(|x| x.as_f64()),
    ) {
        if is_logical || (width <= MAX_WINDOW_WIDTH && height <= MAX_WINDOW_HEIGHT) {
            (width, height)
        } else {
            // Old data without a unit may have physical pixels written into width/height
            (width / factor, height / factor)
        }
    } else if let (Some(wpx), Some(hpx)) = (
        v.get("width_px").and_then(|x| x.as_f64()),
        v.get("height_px").and_then(|x| x.as_f64()),
    ) {
        // Convert from physical pixels to logical size
        (wpx / factor, hpx / factor)
    } else {
        return Ok(());
    };

    // Clamp window size: width 600-1800, height 400-1000
    let w = width_logical.clamp(MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH);
    let h = height_logical.clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT);

    win.set_size(tauri::Size::Logical(LogicalSize::new(w, h)))
        .map_err(|e| format!("set_size failed: {}", e))?;

    let _ = save_window_logical_size(app, w, h);

    Ok(())
}
