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
        "tencent" => translate_tencent(text, target_lang).await,
        "google" | _ => translate_google(text, target_lang).await,
    }
}

async fn translate_tencent(text: String, target_lang: &str) -> Result<String, String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Hardcoded credentials as requested
    let secret_id = "AKIDhXAbsxqYFoEW4Xqx6EDctysb9VhhV7Wl";
    let secret_key = "55tVAev0vd8TbWFvYzEH7IH4e39IFyWr";
    
    let service = "tmt";
    let host = "tmt.tencentcloudapi.com";
    let region = "ap-guangzhou";
    let action = "TextTranslate";
    let version = "2018-03-21";
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let target = if target_lang == "zh-CN" { "zh" } else { "en" };
    let payload = serde_json::json!({
        "Source": "auto",
        "Target": target,
        "ProjectId": 0,
        "SourceText": text
    }).to_string();

    // 1. Canonical Request
    let http_method = "POST";
    let canonical_uri = "/";
    let canonical_querystring = "";
    let canonical_headers = format!("content-type:application/json\nhost:{}\n", host);
    let signed_headers = "content-type;host";
    
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    let hashed_payload = hex::encode(hasher.finalize());
    
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        http_method, canonical_uri, canonical_querystring, canonical_headers, signed_headers, hashed_payload
    );

    // 2. String to Sign
    let algorithm = "TC3-HMAC-SHA256";
    let credential_scope = format!("{}/{}/tc3_request", date, service);
    
    let mut hasher = Sha256::new();
    hasher.update(canonical_request.as_bytes());
    let hashed_canonical_request = hex::encode(hasher.finalize());
    
    let string_to_sign = format!(
        "{}\n{}\n{}\n{}",
        algorithm, timestamp, credential_scope, hashed_canonical_request
    );

    // 3. Calculate Signature
    fn sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(msg);
        mac.finalize().into_bytes().to_vec()
    }

    let k_date = sign(format!("TC3{}", secret_key).as_bytes(), date.as_bytes());
    let k_service = sign(&k_date, service.as_bytes());
    let k_signing = sign(&k_service, b"tc3_request");
    let signature = hex::encode(sign(&k_signing, string_to_sign.as_bytes()));

    // 4. Authorization Header
    let authorization = format!(
        "{} Credential={}/{}, SignedHeaders={}, Signature={}",
        algorithm, secret_id, credential_scope, signed_headers, signature
    );

    let client = reqwest::Client::new();
    let response = client.post(format!("https://{}", host))
        .header("Authorization", authorization)
        .header("Content-Type", "application/json")
        .header("Host", host)
        .header("X-TC-Action", action)
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Version", version)
        .header("X-TC-Region", region)
        .body(payload)
        .send()
        .await
        .map_err(|e| format!("Tencent network error: {}", e))?;

    let json: Value = response.json().await.map_err(|e| format!("Tencent parse error: {}", e))?;
    
    if let Some(resp) = json.get("Response") {
        if let Some(err) = resp.get("Error") {
            return Err(format!("Tencent API error: {} ({})", err["Message"], err["Code"]));
        }
        if let Some(target_text) = resp.get("TargetText").and_then(|t| t.as_str()) {
            return Ok(target_text.to_string());
        }
    }

    Err("Invalid response from Tencent Cloud TMT".to_string())
}

async fn translate_google(text: String, target_lang: &str) -> Result<String, String> {
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
