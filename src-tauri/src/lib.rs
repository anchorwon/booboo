mod ocr_core;
mod translate;
mod paddle_ocr_engine;
mod config;

use config::{ConfigState, get_config, AppConfig};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState, Code};
use std::sync::Mutex;
use tauri::{Manager, State, Emitter};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

struct AppState {
    is_pinned: Mutex<bool>,
    is_dashboard_open: Mutex<bool>,
    is_capturing: Mutex<bool>,
    is_processing: Mutex<bool>,
    pin_cache: Mutex<std::collections::HashMap<String, String>>,
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
            let abs_x_raw = (x as f64 * scale_factor).round() as i32;
            let abs_y_raw = (y as f64 * scale_factor).round() as i32;
            let abs_w_raw = (width as f64 * scale_factor).round() as u32;
            let abs_h_raw = (height as f64 * scale_factor).round() as u32;

            // Add padding to help OCR context (e.g. 15px visual margin)
            // But verify we don't go negative on x/y. 
            // Width/height clamping happens in ocr_core::capture_area automatically.
            let padding = (15.0 * scale_factor).round() as i32;
            
            let abs_x = (abs_x_raw - padding).max(0);
            let abs_y = (abs_y_raw - padding).max(0);
            
            // Adjust width/height to compensate for the shift + extra margin on right/bottom
            // Total width increase = padding_left + padding_right
            // Total height increase = padding_top + padding_bottom
            // Note: If x was 0, padding_left was partly ignored, but we still add full padding_right?
            // Let's keep it simple: Expand outward.
            let abs_w = abs_w_raw + (2 * padding as u32);
            let abs_h = abs_h_raw + (2 * padding as u32);

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
fn set_processing_state(state: State<AppState>, processing: bool) {
    let mut is_proc = state.is_processing.lock().unwrap();
    *is_proc = processing;
    println!("DEBUG: Processing state set to: {}", processing);
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
fn log_message(msg: String) {
    println!("FRONTEND LOG: {}", msg);
}


#[tauri::command]
fn enter_capture_mode(window: tauri::Window, state: State<'_, AppState>) {
    let start = std::time::Instant::now();
    println!("DEBUG: Entering capture mode (Backend) at {:?}", start);
    
    // 1. Force window state clean
    let _ = window.set_decorations(false);
    // set_skip_taskbar is already true in config, skipping to save time
    let _ = window.set_fullscreen(false);
    // let _ = window.set_resizable(false); // Skipping for speed
    
    let t1 = start.elapsed();
    
    // 2. Manual Fullscreen: Match monitor size
    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let position = monitor.position();
        
        let _ = window.set_position(*position);
        let _ = window.set_size(*size);
    }
    
    let t2 = start.elapsed();
    
    // 3. Show and Focus
    let _ = window.show();
    let _ = window.set_focus();
    
    let end = start.elapsed();
    println!("DEBUG: enter_capture_mode timing: setup={:?}, access_monitor_and_resize={:?}, total={:?}", t1, t2 - t1, end);
    
    let mut is_capturing = state.is_capturing.lock().unwrap();
    *is_capturing = true;
}

#[tauri::command]
fn exit_capture_mode(window: tauri::Window, state: State<'_, AppState>) {
    println!("DEBUG: Exiting capture mode (Backend - Restore)");
    let mut is_capturing = state.is_capturing.lock().unwrap();
    *is_capturing = false;
    
    // let _ = window.set_always_on_top(false);
    let _ = window.set_resizable(true);
    let _ = window.set_fullscreen(false); // Just in case
    let _ = window.set_decorations(false); 
    
    // Frontend will call resize_dashboard_window next, usually
}

#[tauri::command]
async fn create_pin_window(app: tauri::AppHandle, label: String, width: f64, height: f64) -> Result<(), String> {
    use tauri::{WebviewWindowBuilder, WebviewUrl};
    
    println!("Creating pin window: {}, {}x{}", label, width, height);
    let win = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::App("pin".into())
    )
    .inner_size(width, height)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .shadow(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn pin_selection(app: tauri::AppHandle, window: tauri::Window, x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;
    
    // 1. Capture the area
    // Reuse logic from ocr_capture_area but for pinning
    // We need absolute physical coordinates
    
    let abs_x = (x as f64 * scale_factor).round() as i32;
    let abs_y = (y as f64 * scale_factor).round() as i32;
    let abs_w = (width as f64 * scale_factor).round() as u32;
    let abs_h = (height as f64 * scale_factor).round() as u32;
    
    let img = ocr_core::capture_area(abs_x, abs_y, abs_w, abs_h).await.map_err(|e| e.to_string())?;
    
    // 2. Convert to base64
    let base64_img = ocr_core::image_to_base64(&img);
    if base64_img.is_empty() {
        return Err("Failed to encode image".to_string());
    }
    
    let img_src = format!("data:image/png;base64,{}", base64_img);
    
    // 3. Create window
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let label = format!("pin_{}", timestamp);
    
    // Use logical size for window creation
    create_pin_window(app.clone(), label.clone(), width as f64, height as f64).await?;
    
    // 4. Store image data in cache
    {
        let state = app.state::<AppState>();
        let mut cache = state.pin_cache.lock().unwrap();
        cache.insert(label.clone(), img_src);
    }

    Ok(())
}

#[tauri::command]
async fn get_pin_image(state: State<'_, AppState>, label: String) -> Result<String, String> {
    let mut cache = state.pin_cache.lock().unwrap();
    if let Some(img) = cache.remove(&label) {
        Ok(img)
    } else {
        Err("Image not found in cache".to_string())
    }
}


#[tauri::command]
async fn resize_dashboard_window(window: tauri::Window, state: State<'_, AppState>, mode: String) -> Result<(), String> {
    let mut is_dashboard_open_lock = state.is_dashboard_open.lock().unwrap();

    if mode == "dashboard" {
        *is_dashboard_open_lock = true;
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 850.0, height: 650.0 }));
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        Ok(())
    } else {
        // --- CLOSING DASHBOARD ---
        *is_dashboard_open_lock = false;
        
        let is_processing = *state.is_processing.lock().unwrap();

        if !is_processing {
            // 1. Hide immediately if we are truly closing the dashboard to tray
            let _ = window.hide();
            
            // 2. Perform resize/center in background to avoid any visual jitter
            let window_clone = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let _ = window_clone.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 450.0, height: 550.0 }));
                let _ = window_clone.center();
            });
        } else {
            // If we are processing, just resize and show (avoid the hide flicker)
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 450.0, height: 550.0 }));
            let _ = window.center();
            let _ = window.show();
            let _ = window.set_focus();
        }
        
        Ok(())
    }
}

#[tauri::command]
async fn close_window(window: tauri::Window) -> Result<(), String> {
    println!("Closing window: {}", window.label());
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                println!("DEBUG: Global shortcut event received: {:?}", shortcut);
                if event.state() == ShortcutState::Pressed {
                    println!("DEBUG: Global shortcut pressed!");
                    if let Some(window) = app.get_webview_window("main") {
                        let config = app.state::<ConfigState>().config.lock().unwrap().clone();
                        use std::str::FromStr;
                        if let Ok(pin_shortcut) = Shortcut::from_str(&config.shortcut_pin) {
                            if shortcut == &pin_shortcut {
                                let _ = window.emit("shortcut-pin", ());
                                return;
                            }
                        }
                        
                        if let Ok(ocr_shortcut) = Shortcut::from_str(&config.shortcut) {
                            if shortcut == &ocr_shortcut {
                                let _ = window.emit("shortcut-capture", ());
                                return;
                            }
                        }
                    }
                }
            })
            .build()
        )
        // ... (rest of setup)
        .setup(|app| {
            // Initialize Config State
            let config_state = ConfigState::new(app.handle());
            app.manage(config_state);

            // Register global shortcut
            let config = app.state::<ConfigState>().config.lock().unwrap().clone();
            let shortcut_str = if config.shortcut.is_empty() { "Alt+Shift+A".to_string() } else { config.shortcut.clone() };
            let pin_str = if config.shortcut_pin.is_empty() { "Alt+Shift+S".to_string() } else { config.shortcut_pin.clone() };

            use std::str::FromStr;
            
            // Register OCR
            if let Ok(s) = Shortcut::from_str(&shortcut_str) {
                 if let Err(e) = app.global_shortcut().register(s) {
                     println!("Failed to register OCR shortcut '{}': {}", shortcut_str, e);
                 }
            }
            
            // Register Pin
             if let Ok(s) = Shortcut::from_str(&pin_str) {
                 if let Err(e) = app.global_shortcut().register(s) {
                     println!("Failed to register Pin shortcut '{}': {}", pin_str, e);
                 }
            }
            
            // Create tray menu
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &quit])?;

            // Build tray icon
            // Build tray icon
            let icon_bytes = include_bytes!("../../static/logo.png");
            
            // Process image to add rounded corners
            let tray_icon = {
                use image::{GenericImageView, ImageBuffer, Rgba};
                let img = image::load_from_memory(icon_bytes).map_err(|e| e.to_string())?;
                let (width, height) = img.dimensions();
                let radius = width as f32 * 0.2; // 20% radius
                let center_x = width as f32 / 2.0;
                let center_y = height as f32 / 2.0;
                
                let mut buffer = ImageBuffer::new(width, height);
                
                for (x, y, pixel) in img.pixels() {
                    let cx = if (x as f32) < radius { radius } else if (x as f32) > width as f32 - radius { width as f32 - radius } else { x as f32 };
                    let cy = if (y as f32) < radius { radius } else if (y as f32) > height as f32 - radius { height as f32 - radius } else { y as f32 };
                    
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let distance = (dx * dx + dy * dy).sqrt();
                    
                    if distance <= radius + 0.5 {
                        buffer.put_pixel(x, y, pixel);
                    } else {
                        // Transparent
                         buffer.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                    }
                }
                
                // Encode back to PNG to get an owned Image via from_bytes (bypassing lifetime issues with Image::new generic)
                use std::io::Cursor;
                use image::ImageFormat;
                
                let mut cursor = Cursor::new(Vec::new());
                buffer.write_to(&mut cursor, ImageFormat::Png).map_err(|e| e.to_string())?;
                let png_bytes = cursor.into_inner();
                
                tauri::image::Image::from_bytes(&png_bytes).unwrap_or_else(|_| {
                    app.default_window_icon().unwrap().clone()
                })
            };

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                // Sync internal state
                                {
                                    let state = app.state::<AppState>();
                                    let mut is_db = state.is_dashboard_open.lock().unwrap();
                                    *is_db = true;
                                }

                                // Reset and show dash
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
                is_dashboard_open: Mutex::new(false),
                is_capturing: Mutex::new(false),
                is_processing: Mutex::new(false),
                pin_cache: Mutex::new(std::collections::HashMap::new()),
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
                            let is_pinned = *state.is_pinned.lock().unwrap();
                            let is_dashboard_open = *state.is_dashboard_open.lock().unwrap();
                            let is_capturing = *state.is_capturing.lock().unwrap();
                            let is_processing = *state.is_processing.lock().unwrap();

                            println!("DEBUG: Window lost focus. Flags: pinned={}, dashboard={}, capturing={}, processing={}", is_pinned, is_dashboard_open, is_capturing, is_processing);

                            if !is_pinned && !is_dashboard_open && !is_capturing && !is_processing {
                                println!("DEBUG: Hiding window...");
                                let _ = window_clone.hide();
                            } else {
                                println!("DEBUG: Keeping window visible.");
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
            resize_dashboard_window,
            log_message,
            set_processing_state, // Added
            enter_capture_mode,
            exit_capture_mode,
            create_pin_window, // Added
            pin_selection, // Added
            get_pin_image, // Added
            close_window  // New command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, state: State<ConfigState>, new_config: AppConfig) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    
    // Check if shortcut changed
    let mut shortcuts_changed = false;
    
    if config.shortcut != new_config.shortcut {
        println!("Shortcut changed from '{}' to '{}'.", config.shortcut, new_config.shortcut);
        shortcuts_changed = true;
    }
    
    if config.shortcut_pin != new_config.shortcut_pin {
        println!("Pin Shortcut changed from '{}' to '{}'.", config.shortcut_pin, new_config.shortcut_pin);
        shortcuts_changed = true;
    }
    
    if shortcuts_changed {
        // Unregister all 
        let _ = app.global_shortcut().unregister_all();
        
        use std::str::FromStr;
        
        // Re-register OCR
        if let Ok(shortcut) = Shortcut::from_str(&new_config.shortcut) {
             if let Err(e) = app.global_shortcut().register(shortcut) {
                 println!("Failed to register new OCR shortcut: {}", e);
             }
        }
        
        // Re-register Pin
        if let Ok(shortcut) = Shortcut::from_str(&new_config.shortcut_pin) {
             if let Err(e) = app.global_shortcut().register(shortcut) {
                 println!("Failed to register new Pin shortcut: {}", e);
             }
        }
    }
    
    *config = new_config;
    drop(config); // unlock before saving to file
    state.save()
}
