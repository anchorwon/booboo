use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{State, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub ocr_engine: String,       // "paddle" | "windows"
    pub translate_engine: String, // "google" | "youdao"
    pub youdao_app_key: String,
    pub youdao_app_secret: String,
    pub coze_api_key: String,     // Future proofing
    pub shortcut: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ocr_engine: "paddle".to_string(),
            translate_engine: "google".to_string(),
            youdao_app_key: "".to_string(),
            youdao_app_secret: "".to_string(),
            coze_api_key: "".to_string(),
            shortcut: "Alt+Shift+A".to_string(),
        }
    }
}

pub struct ConfigState {
    pub config: Mutex<AppConfig>,
    pub file_path: PathBuf,
}

impl ConfigState {
    pub fn new(app_handle: &tauri::AppHandle) -> Self {
        let config_dir = app_handle.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }
        let file_path = config_dir.join("booboo_config.json");
        
        let config = if file_path.exists() {
            let content = fs::read_to_string(&file_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            AppConfig::default()
        };

        Self {
            config: Mutex::new(config),
            file_path,
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let config = self.config.lock().map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(&*config).map_err(|e| e.to_string())?;
        fs::write(&self.file_path, content).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_config(state: State<ConfigState>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}
