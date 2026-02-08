use std::process::{Command, Stdio, Child};
use std::io::{Write, BufReader, BufRead};
use serde::Deserialize;
use std::sync::Mutex;
use once_cell::sync::Lazy;

#[derive(Debug, Deserialize)]
pub struct PaddleResult {
    pub code: i32,
    #[serde(default)]
    pub message: String,
    pub data: Option<Vec<PaddleData>>,
}

#[derive(Debug, Deserialize)]
pub struct PaddleData {
    pub text: String,
    pub box_points: Vec<Vec<i32>>,
    pub score: f64,
}

// Global singleton to hold the persistent process
static OCR_PROCESS: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

fn get_or_spawn_process() -> Result<std::process::Child, String> {
    // This function acts as a factory. 
    // It spawns a NEW process. The caller is responsible for putting it into the Mutex if needed, 
    // or we can just return the handle.
    // Actually, to interact with a mutex-guarded process, we need to access it IN PLACE.
    // So this helper might just return a Child.
    
    let engine_path = r"C:\Users\Administrator\Desktop\Booboo\ocr-engine\PaddleOCR-json_v1.4.1\PaddleOCR-json.exe";
    let engine_dir = std::path::Path::new(engine_path).parent().ok_or("Invalid engine path")?;
    
    println!("PaddleOCR: Spawning new process...");
    let child = Command::new(engine_path)
        .current_dir(engine_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn PaddleOCR: {}", e))?;
        
    Ok(child)
}

pub fn run_paddle_ocr(image_path: &str) -> Result<String, String> {
    // Use a lock to ensure only one thread accesses the OCR process at a time
    let mut process_guard = OCR_PROCESS.lock().map_err(|e| format!("Mutex poisoned: {}", e))?;

    // Check if process exists and is alive; if not, spawn it
    if process_guard.is_none() {
        let child = get_or_spawn_process()?;
        *process_guard = Some(child);
    }

    let child = process_guard.as_mut().unwrap();

    // Prepare payload
    let payload_json = serde_json::json!({
        "image_path": image_path
    });
    let payload_str = payload_json.to_string();
    println!("Paddle PAYLOAD: {}", payload_str);

    // Send to stdin
    if let Some(stdin) = child.stdin.as_mut() {
        if let Err(e) = stdin.write_all(payload_str.as_bytes()) {
            // Write failed? Process might be dead.
            println!("PaddleOCR: Write failed ({}), killing process.", e);
            let _ = child.kill();
            *process_guard = None; // Reset so next call respawns
            return Err(format!("Failed to write to OCR process: {}", e));
        }
        if let Err(e) = stdin.write_all(b"\n") {
             println!("PaddleOCR: Flush failed ({}), killing process.", e);
             let _ = child.kill();
             *process_guard = None;
             return Err(format!("Failed to flush to OCR process: {}", e));
        }
        if let Err(e) = stdin.flush() {
             println!("PaddleOCR: Flush failed ({}), killing process.", e);
             let _ = child.kill();
             *process_guard = None;
             return Err(format!("Failed to flush stdin: {}", e));
        }
    } else {
        *process_guard = None;
        return Err("Child process stdin not captured".to_string());
    }

    // Read from stdout
    // CRITICAL: We need to read lines until we get a valid JSON result.
    // The engine might print init messages like "PaddleOCR-json v1.4.1" on startup.
    if let Some(stdout) = child.stdout.as_mut() {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF - Process died
                    println!("PaddleOCR: EOF reading stdout, process died.");
                    let _ = child.kill();
                    *process_guard = None;
                    return Err("OCR process closed unexpected (EOF)".to_string());
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() { continue; }
                    
                    println!("Paddle RAW: {}", trimmed);
                    
                    match serde_json::from_str::<serde_json::Value>(&line) {
                        Ok(json_val) => {
                            let code = json_val["code"].as_i64().unwrap_or(0);
                            if code == 100 {
                                // Success: data is array of objects
                                if let Ok(result) = serde_json::from_value::<PaddleResult>(json_val) {
                                     if let Some(data) = result.data {
                                        let full_text = data.into_iter()
                                            .map(|d| d.text)
                                            .collect::<Vec<String>>()
                                            .join("\n");
                                        return Ok(full_text);
                                    }
                                    return Ok("".to_string());
                                }
                            } else {
                                 // Error/No text: data is likely a string
                                 let msg = json_val["data"].as_str().unwrap_or("Unknown error");
                                 // We don't need to kill the process for logical errors (like no text found)
                                 // But for now, returning Err is fine.
                                 // Wait, if it's just "No text found", maybe we should return Ok("")?
                                 if code == 101 {
                                     return Ok("".to_string());
                                 }
                                 return Err(format!("PaddleOCR Error {}: {}", code, msg));
                            }
                            // Fallback if parsing fails inside
                            return Err(format!("Failed to parse PaddleOCR response: {}", line));
                        },
                        Err(_) => {
                             // Likely the welcome message or debug info. Ignore and keep waiting for JSON.
                             println!("PaddleOCR: Ignored non-JSON output: {}", trimmed);
                             continue;
                        }
                    }
                }
                Err(e) => {
                    println!("PaddleOCR: Read error ({}), killing.", e);
                     let _ = child.kill();
                     *process_guard = None;
                     return Err(format!("Failed to read from OCR process: {}", e));
                }
            }
        }
    } else {
        *process_guard = None;
        return Err("Child process stdout not captured".to_string());
    }
}
