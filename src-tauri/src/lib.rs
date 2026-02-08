mod ocr_core;
mod translate;
mod paddle_ocr_engine;
mod config;

use config::{ConfigState, get_config, save_config};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState, Code};
use std::sync::Mutex;
use tauri::{Manager, State, Emitter};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

struct AppState {
    is_pinned: Mutex<bool>,
}

#[tauri::command]
fn toggle_pin(state: State<AppState>) -> bool {
    let mut pinned = state.is_pinned.lock().unwrap();
    *pinned = !*pinned;
    println!("DEBUG: Pin state toggled to: {}", *pinned);
    *pinned
}

#[tauri::command]
async fn capture_full_screen() -> Result<String, String> {
    ocr_core::capture_full_screen().await
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn ocr_capture_area(window: tauri::Window, config_state: State<'_, ConfigState>, x: i32, y: i32, width: u32, height: u32) -> Result<String, String> {
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;
    let window_pos = window.outer_position().map_err(|e| e.to_string())?;
    println!("Trace 1: Window outer_pos: {:?}, Scale: {}, Selection: logical x={}, y={}, w={}, h={}", 
        window_pos, scale_factor, x, y, width, height);

    // Get current config
    let config = config_state.config.lock().unwrap().clone();

    let res = tauri::async_runtime::spawn_blocking(move || {
        std::panic::catch_unwind(move || {
            // Absolute Screen Physical = Selection Logical * Scale
            // (Since the capture and overlay are both screen-fixed in this new mode)
            let abs_x = (x as f64 * scale_factor).round() as i32;
            let abs_y = (y as f64 * scale_factor).round() as i32;
            let abs_w = (width as f64 * scale_factor).round() as u32;
            let abs_h = (height as f64 * scale_factor).round() as u32;

            println!("Trace 2: Final Absolute Screen Physical: x={}, y={}, w={}, h={}", abs_x, abs_y, abs_w, abs_h);
            
            tauri::async_runtime::block_on(async {
                println!("Trace 3: Calling capture_area (crop from cache)...");
                let img = match ocr_core::capture_area(abs_x, abs_y, abs_w, abs_h).await {
                    Ok(i) => i,
                    Err(e) => return Err(format!("Capture failed: {}", e)),
                };
                
                println!("Trace 4: Calling run_ocr with engine: {}...", config.ocr_engine);
                match ocr_core::run_ocr(img, &config).await {
                    Ok(t) => Ok(t),
                    Err(e) => Err(format!("OCR failed: {}", e)),
                }
            })
        })
    }).await.map_err(|e| format!("JoinError: {}", e))?;

    match res {
        Ok(r) => r,
        Err(e) => {
            println!("Trace ERROR: {:?}", e);
            Err("Command panicked during execution".to_string())
        }
    }
}

#[tauri::command]
async fn translate_text(config_state: State<'_, ConfigState>, text: String, target_lang: String) -> Result<String, String> {
    let config = config_state.config.lock().unwrap().clone();
    translate::translate(text, &target_lang, &config).await
}

#[tauri::command]
async fn verify_youdao_id_and_key(app_key: String, app_secret: String) -> Result<String, String> {
    let mut config = config::AppConfig::default();
    config.youdao_app_key = app_key;
    config.youdao_app_secret = app_secret;
    
    // Attempt a small translation to verify credentials
    let test_text = "test".to_string();
    let target_lang = "zh-CN";
    
    match translate::translate_youdao(test_text, target_lang, &config).await {
        Ok(_) => Ok("验证成功！API 密钥配置正确。".to_string()),
        Err(e) => Err(format!("验证失败: {}", e)),
    }
}

#[tauri::command]
async fn resize_dashboard_window(window: tauri::Window, mode: String) -> Result<(), String> {
    if mode == "dashboard" {
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 850.0, height: 650.0 }));
    } else {
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 450.0, height: 550.0 }));
    }
    let _ = window.center();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    println!("DEBUG: Global shortcut pressed!");
                    if let Some(window) = app.get_webview_window("main") {
                        println!("DEBUG: Emitting shortcut-capture event...");
                        // We do NOT call window.show() here to prevent the window flash in screenshot.
                        // The frontend will call show() AFTER capture_full_screen is done.
                        let _ = window.emit("shortcut-capture", ());
                        let _ = window.set_focus();
                    } else {
                        println!("DEBUG: Main window NOT found!");
                    }
                }
            })
            .build()
        )
        .setup(|app| {
            // Initialize Config State
            let config_state = ConfigState::new(app.handle());
            app.manage(config_state);

            // Register global shortcut
            let shortcut = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyA);
            if let Err(e) = app.global_shortcut().register(shortcut) {
                println!("Warning: Failed to register global shortcut: {}", e);
            } else {
                println!("Global shortcut registered successfully.");
            }

            // Create tray menu
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &quit])?;

            // Build tray icon from specific path for better reliability
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                // Resize to dashboard size
                                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 850.0, height: 650.0 }));
                                let _ = window.center();
                                let _ = window.emit("open-settings", ());
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            app.manage(AppState {
                is_pinned: Mutex::new(false),
            });

            // Handle window events
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            println!("DEBUG: Close requested, hiding window...");
                            let _ = window_clone.hide();
                        }
                        tauri::WindowEvent::Focused(false) => {
                            let state = app_handle.state::<AppState>();
                            let is_pinned = state.is_pinned.lock().unwrap();
                            if !*is_pinned {
                                println!("DEBUG: Window lost focus and NOT pinned, hiding...");
                                let _ = window_clone.hide();
                            } else {
                                println!("DEBUG: Window lost focus but IS pinned, keeping visible.");
                            }
                        }
                        _ => {}
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ocr_capture_area, 
            translate_text, 
            capture_full_screen, 
            toggle_pin,
            get_config,
            save_config,
            verify_youdao_id_and_key,
            resize_dashboard_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
