use log::{error, info};
use std::error::Error;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{image::Image, tray::TrayIconBuilder, App, AppHandle, Manager, Runtime};

use super::consts::menu_labels;
use super::menu::{open_about_window, open_settings_window};

/// Create a Tauri Image from byte data (uses the raw image directly)
/// Only used on macOS
#[cfg(target_os = "macos")]
fn create_image_from_bytes(icon_bytes: &[u8], platform: &str) -> Option<Image<'static>> {
    match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let image = Image::new_owned(rgba_img.into_raw(), width, height);
            info!(
                "Loaded custom tray icon for {} ({}x{})",
                platform, width, height
            );
            Some(image)
        }
        Err(_e) => None,
    }
}

/// Load a custom tray icon (macOS only)
fn load_custom_tray_icon() -> Option<Image<'static>> {
    #[cfg(target_os = "macos")]
    {
        let icon_bytes = include_bytes!("../../icons/tray-mac.png");
        create_image_from_bytes(icon_bytes, "macOS")
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Don't load a custom icon on non-macOS platforms
        None
    }
}

/// Create the system tray
pub fn setup_tray<R: Runtime>(menu: &Menu<R>, app: &App<R>) -> Result<(), Box<dyn Error>>
where
    App<R>: Manager<R>,
    AppHandle<R>: Manager<R>,
{
    let app_handle = app.app_handle().clone();

    // Try to load a custom tray icon, falling back to the default icon on failure
    let icon = load_custom_tray_icon().unwrap_or_else(|| {
        info!("Using default window icon for tray");
        app_handle
            .default_window_icon()
            .ok_or("Failed to get default window icon")
            .unwrap()
            .clone()
    });

    let tray_menu = build_tray_menu(&app_handle).unwrap_or_else(|_| menu.clone());

    let tray_result = TrayIconBuilder::new()
        .menu(&tray_menu)
        .show_menu_on_left_click(true)
        .icon(icon)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show-main" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "open-settings" => open_settings_window(app),
            "about" => open_about_window(app),
            "quit" => app.exit(0),
            other => println!("menu item {} not handled", other),
        })
        .build(app);

    match tray_result {
        Ok(_tray) => {
            // On macOS, set the icon as a template image so the system can automatically adjust its color based on the menu bar background
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = _tray.set_icon_as_template(true) {
                    error!("Failed to set tray icon as template: {}", e);
                } else {
                    info!("Tray icon set as template for macOS");
                }
            }
            info!("System tray created successfully!");
            Ok(())
        }
        Err(e) => {
            error!("Failed to create system tray: {}", e);
            Err(Box::new(e))
        }
    }
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>>
where
    AppHandle<R>: Manager<R>,
{
    let use_zh = prefers_zh();
    let app_name = app.package_info().name.clone();
    let labels = menu_labels(use_zh, &app_name);

    let show_main = if use_zh {
        "显示主窗口"
    } else {
        "Show Main Window"
    };

    let show_main_i = MenuItem::with_id(app, "show-main", show_main, true, None::<&str>)?;
    let settings_i = MenuItem::with_id(
        app,
        "open-settings",
        labels.settings_label.as_str(),
        true,
        None::<&str>,
    )?;
    let about_i = MenuItem::with_id(
        app,
        "about",
        labels.about_label.as_str(),
        true,
        None::<&str>,
    )?;
    let quit_i = MenuItem::with_id(app, "quit", labels.quit_label.as_str(), true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &show_main_i,
            &settings_i,
            &about_i,
            &PredefinedMenuItem::separator(app)?,
            &quit_i,
        ],
    )
}

fn prefers_zh() -> bool {
    tauri_plugin_os::locale()
        .or_else(|| std::env::var("LANG").ok())
        .map(|lang| lang.to_lowercase().starts_with("zh"))
        .unwrap_or(false)
}
