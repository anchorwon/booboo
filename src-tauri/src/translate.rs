use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest;
use serde_json::Value;

pub async fn translate(text: String, target_lang: &str, config: &crate::config::AppConfig) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok("".to_string());
    }

    match config.translate_engine.as_str() {
        "youdao" => translate_youdao(text, target_lang, config).await,
        "google" | _ => translate_google(text, target_lang).await,
    }
}

async fn translate_google(text: String, target_lang: &str) -> Result<String, String> {
    // ... existing translate_google code ...
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}",
        target_lang,
        urlencoding::encode(&text)
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let mut translated_text = String::new();
    if let Some(outer_array) = json.as_array() {
        if let Some(inner_array) = outer_array.get(0).and_then(|v| v.as_array()) {
            for entry in inner_array {
                if let Some(line_array) = entry.as_array() {
                    if let Some(txt) = line_array.get(0).and_then(|v| v.as_str()) {
                        translated_text.push_str(txt);
                    }
                }
            }
        }
    }

    if translated_text.is_empty() {
        Ok("Translation failed or empty result".to_string())
    } else {
        Ok(translated_text)
    }
}

fn truncate_q(q: &str) -> String {
    let len = q.chars().count();
    if len <= 20 {
        return q.to_string();
    }
    let chars: Vec<char> = q.chars().collect();
    let first_10: String = chars[0..10].iter().collect();
    let last_10: String = chars[len - 10..len].iter().collect();
    format!("{}{}{}", first_10, len, last_10)
}

pub async fn translate_youdao(text: String, target_lang: &str, config: &crate::config::AppConfig) -> Result<String, String> {
    if config.youdao_app_key.is_empty() || config.youdao_app_secret.is_empty() {
        return Err("Youdao ID or Secret is missing. Please set them in Settings.".to_string());
    }

    let app_key = &config.youdao_app_key;
    let app_secret = &config.youdao_app_secret;
    let salt = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string();
    let curtime = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()).to_string();
    
    let q_truncated = truncate_q(&text);
    let sign_str = format!("{}{}{}{}{}", app_key, q_truncated, salt, curtime, app_secret);
    
    let mut hasher = Sha256::new();
    hasher.update(sign_str.as_bytes());
    let sign = hex::encode(hasher.finalize());

    let mut params = vec![
        ("q", text.as_str()),
        ("from", "auto"),
        ("to", target_lang),
        ("appKey", app_key.as_str()),
        ("salt", salt.as_str()),
        ("sign", sign.as_str()),
        ("signType", "v3"),
        ("curtime", curtime.as_str()),
    ];
    
    // Adjust target_lang to Youdao's format if needed (e.g., 'zh-CN' to 'zh-CHS')
    if target_lang == "zh-CN" {
        params[2].1 = "zh-CHS";
    }

    let client = reqwest::Client::new();
    let response = client.post("https://openapi.youdao.com/api")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Youdao network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Youdao API returned status {}", response.status()));
    }

    let json: Value = response.json().await.map_err(|e| format!("Youdao parse error: {}", e))?;

    if let Some(error_code) = json["errorCode"].as_str() {
        if error_code != "0" {
            return Err(format!("Youdao API error code: {}", error_code));
        }
    }

    if let Some(translation_array) = json["translation"].as_array() {
        if let Some(first_translation) = translation_array.get(0).and_then(|v| v.as_str()) {
            return Ok(first_translation.to_string());
        }
    }

    Err("Invalid response format from Youdao".to_string())
}
