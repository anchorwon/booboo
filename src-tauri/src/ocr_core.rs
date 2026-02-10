use screenshots::Screen;
use image::{DynamicImage, ImageBuffer};
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
use std::sync::Mutex;
use once_cell::sync::Lazy;

static LAST_SCREENSHOT: Lazy<Mutex<Option<DynamicImage>>> = Lazy::new(|| Mutex::new(None));

pub async fn capture_full_screen() -> Result<String, String> {
    let start_time = std::time::Instant::now();
    let screens = Screen::all().map_err(|e| e.to_string())?;
    let screen = screens.into_iter().next().ok_or("No screen found")?;
    
    println!("ocr_core: Screen::all took {:?}", start_time.elapsed());
    let capture_start = std::time::Instant::now();
    let image = screen.capture().map_err(|e| e.to_string())?;
    println!("ocr_core: screen.capture() took {:?}", capture_start.elapsed());
    
    let dynamic_image = DynamicImage::ImageRgba8(
        ImageBuffer::from_raw(image.width(), image.height(), image.into_raw())
            .ok_or("Failed to create image buffer")?
    );

    let cache_start = std::time::Instant::now();
    // Cache the FULL resolution image for subsequent accurate cropping
    {
        let mut cache = LAST_SCREENSHOT.lock().unwrap();
        *cache = Some(dynamic_image.clone());
    }
    println!("ocr_core: caching took {:?}", cache_start.elapsed());

    // Convert to BMP file for zero-latency transfer
    let start_encode = std::time::Instant::now();
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("booboo_capture.bmp");
    
    // BMP encoding is nearly zero-cost (no compression). 
    // Disk write for ~15-30MB on SSD is much faster than JPEG software encoding.
    dynamic_image.save_with_format(&file_path, image::ImageFormat::Bmp)
        .map_err(|e| e.to_string())?;

    println!("ocr_core: BMP save took {:?}", start_encode.elapsed());
    println!("ocr_core: Total capture_full_screen backend took {:?}", start_time.elapsed());
    
    Ok(file_path.to_string_lossy().to_string())
}

pub async fn capture_area(x: i32, y: i32, width: u32, height: u32) -> Result<DynamicImage, String> {
    println!("ocr_core: capture_area (crop) start: x={}, y={}, w={}, h={}", x, y, width, height);
    
    let cached_image = {
        let cache = LAST_SCREENSHOT.lock().unwrap();
        cache.clone().ok_or("No cached screenshot found. Please capture full screen first.")?
    };

    // Coordinates are absolute physical from lib.rs
    let px = x.max(0) as u32;
    let py = y.max(0) as u32;
    let pw = width;
    let ph = height;
    
    // Safety check: ensure crop area is within bounds
    let crop_x = px.min(cached_image.width() - 1);
    let crop_y = py.min(cached_image.height() - 1);
    let crop_w = pw.min(cached_image.width() - crop_x);
    let crop_h = ph.min(cached_image.height() - crop_y);

    println!("ocr_core: cropping cached image: x={}, y={}, w={}, h={}", crop_x, crop_y, crop_w, crop_h);
    
    let cropped = cached_image.crop_imm(crop_x, crop_y, crop_w, crop_h);
    
    println!("ocr_core: crop done. size: {}x{}", cropped.width(), cropped.height());
    Ok(cropped)
}

pub async fn run_ocr(image: DynamicImage, config: &crate::config::AppConfig) -> Result<String, String> {
    println!("ocr_core: run_ocr start. Input size: {}x{}, Engine: {}", image.width(), image.height(), config.ocr_engine);
    
    match config.ocr_engine.as_str() {
        "paddle" => {
            // 1. Save temp image for PaddleOCR (it needs a file path)
            let temp_dir = std::env::temp_dir();
            let temp_file = temp_dir.join("ocr_paddle_input.png");
            let file_path_str = temp_file.to_string_lossy().to_string();
            
            image.save_with_format(&temp_file, image::ImageFormat::Png).map_err(|e| e.to_string())?;

            // 2. Try PaddleOCR
            println!("ocr_core: Attempting PaddleOCR...");
            match crate::paddle_ocr_engine::run_paddle_ocr(&file_path_str) {
                Ok(text) => {
                    println!("ocr_core: PaddleOCR success. Length: {}", text.len());
                    Ok(text)
                },
                Err(e) => {
                    println!("ocr_core: PaddleOCR failed: {}. Falling back to Windows OCR.", e);
                    // Fallback to Windows OCR if Paddle fails (transient error or process issue)
                    run_windows_ocr_logic(image).await
                }
            }
        },
        "windows" => {
            run_windows_ocr_logic(image).await
        },
        _ => {
            // Default or unknown -> Windows fallback
            println!("ocr_core: Unknown engine '{}', defaulting to Windows OCR.", config.ocr_engine);
            run_windows_ocr_logic(image).await
        }
    }
}

async fn run_windows_ocr_logic(image: DynamicImage) -> Result<String, String> {
    println!("ocr_core: Running Windows OCR logic...");

    // 1. Scale up by 2.5x using Lanczos3
    // Triangle at 3x was too blurry for dense text.
    // Lanczos3 provides the best trade-off between sharpness (for glitch font) and artifact control (for punctuation).
    let new_w = (image.width() as f64 * 2.5) as u32;
    let new_h = (image.height() as f64 * 2.5) as u32;
    let upscaled = image.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
    
    // 2. Convert to grayscale
    let luma = upscaled.to_luma8();
    
    // 3. Prepare Inverted Copy (for dark mode support)
    let mut luma_inverted = luma.clone();
    for p in luma_inverted.pixels_mut() {
        p.0 = [255 - p.0[0]];
    }
    
    // 4. Run OCR on BOTH strategies PARALLEL
    // Save debug images for troubleshooting (Commented out for production)
    // let _ = luma.save("C:/Users/Administrator/Desktop/ocr_debug_normal.png");
    // let _ = luma_inverted.save("C:/Users/Administrator/Desktop/ocr_debug_inverted.png");

    let mut png_normal = Vec::new();
    DynamicImage::ImageLuma8(luma).write_to(&mut std::io::Cursor::new(&mut png_normal), image::ImageFormat::Png).map_err(|e| e.to_string())?;

    let mut png_inverted = Vec::new();
    DynamicImage::ImageLuma8(luma_inverted).write_to(&mut std::io::Cursor::new(&mut png_inverted), image::ImageFormat::Png).map_err(|e| e.to_string())?;

    // Run in parallel for speed
    let task_normal = tauri::async_runtime::spawn_blocking(move || {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        tauri::async_runtime::block_on(async {
            run_windows_native_ocr_v2(png_normal).await
        })
    });

    let task_inverted = tauri::async_runtime::spawn_blocking(move || {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        tauri::async_runtime::block_on(async {
            run_windows_native_ocr_v2(png_inverted).await
        })
    });

    // Wait for both (sequential await on handles is fine loop-free)
    let res_normal_r = task_normal.await;
    let res_inverted_r = task_inverted.await;

    // Handle join errors and inner errors
    let res_normal = res_normal_r.map_err(|e| e.to_string())??;
    let res_inverted = res_inverted_r.map_err(|e| e.to_string())??;

    println!("ocr_core: Result Normal len: {}, Inverted len: {}", res_normal.len(), res_inverted.len());

    // heuristic: pick the longer one
    if res_inverted.len() > res_normal.len() {
        println!("ocr_core: Selected INVERTED result.");
        Ok(res_inverted)
    } else {
        println!("ocr_core: Selected NORMAL result.");
        Ok(res_normal)
    }
}

fn clean_ocr_text(text: &str) -> String {
    let mut cleaned = String::new();
    let chars: Vec<char> = text.chars().collect();
    
    for i in 0..chars.len() {
        let c = chars[i];
        if c == ' ' {
            // Check if surrounded by CJK characters
            if i > 0 && i < chars.len() - 1 {
                let prev = chars[i-1];
                let next = chars[i+1];
                if is_cjk(prev) && is_cjk(next) {
                    // Skip space between CJK
                    continue;
                }
            }
        }
        cleaned.push(c);
    }
    cleaned
}

fn is_cjk(c: char) -> bool {
    // Basic CJK Unified Ideographs block
    (c >= '\u{4E00}' && c <= '\u{9FFF}') ||
    // CJK Symbols and Punctuation
    (c >= '\u{3000}' && c <= '\u{303F}') ||
    // Fullwidth forms
    (c >= '\u{FF00}' && c <= '\u{FFEF}')
}

async fn run_windows_native_ocr_v2(png_bytes: Vec<u8>) -> Result<String, String> {
    let stream = InMemoryRandomAccessStream::new().map_err(|e: windows::core::Error| e.to_string())?;
    let writer = stream.GetOutputStreamAt(0).map_err(|e: windows::core::Error| e.to_string())?;
    let data_writer = DataWriter::CreateDataWriter(&writer).map_err(|e: windows::core::Error| e.to_string())?;
    data_writer.WriteBytes(&png_bytes).map_err(|e: windows::core::Error| e.to_string())?;
    data_writer.StoreAsync().map_err(|e: windows::core::Error| e.to_string())?.await.map_err(|e: windows::core::Error| e.to_string())?;
    data_writer.FlushAsync().map_err(|e: windows::core::Error| e.to_string())?.await.map_err(|e: windows::core::Error| e.to_string())?;

    let decoder = BitmapDecoder::CreateAsync(&stream).map_err(|e: windows::core::Error| e.to_string())?.await.map_err(|e: windows::core::Error| e.to_string())?;
    let bitmap = decoder.GetSoftwareBitmapAsync().map_err(|e: windows::core::Error| e.to_string())?.await.map_err(|e: windows::core::Error| e.to_string())?;

    let mut results = Vec::new();

    // Strategy: Try User Profile first
    if let Ok(engine) = OcrEngine::TryCreateFromUserProfileLanguages() {
        if let Ok(result) = engine.RecognizeAsync(&bitmap).map_err(|e| e.to_string())?.await {
            // Use Lines() instead of Text() to ensure we get explicit line breaks
            if let Ok(lines) = result.Lines() {
                let mut full_text = String::new();
                for line in lines {
                    if let Ok(line_text) = line.Text() {
                         let raw_line = line_text.to_string();
                         let cleaned = clean_ocr_text(&raw_line);
                         if !cleaned.trim().is_empty() {
                             if !full_text.is_empty() {
                                 full_text.push('\n');
                             }
                             full_text.push_str(&cleaned);
                         }
                    }
                }
                println!("ocr_core: UserProfile Text Len: {}", full_text.len());
                if !full_text.is_empty() {
                    results.push((full_text.len(), full_text));
                }
            }
        }
    }

    // Attempt: Chinese specific
    if let Ok(lang) = windows::Globalization::Language::CreateLanguage(&windows::core::HSTRING::from("zh-Hans")) {
        if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) {
            if let Ok(result) = engine.RecognizeAsync(&bitmap).map_err(|e| e.to_string())?.await {
                if let Ok(lines) = result.Lines() {
                    let mut full_text = String::new();
                    for line in lines {
                        if let Ok(line_text) = line.Text() {
                             let raw_line = line_text.to_string();
                             let cleaned = clean_ocr_text(&raw_line);
                             if !cleaned.trim().is_empty() {
                                 if !full_text.is_empty() {
                                     full_text.push('\n');
                                 }
                                 full_text.push_str(&cleaned);
                             }
                        }
                    }
                    println!("ocr_core: zh-Hans Text Len: {}", full_text.len());
                    if !full_text.is_empty() {
                        results.push((full_text.len(), full_text));
                    }
                }
            }
        }
    }

    // Attempt: English specific
    if let Ok(lang) = windows::Globalization::Language::CreateLanguage(&windows::core::HSTRING::from("en-US")) {
        if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) {
             if let Ok(result) = engine.RecognizeAsync(&bitmap).map_err(|e| e.to_string())?.await {
                if let Ok(lines) = result.Lines() {
                    let mut full_text = String::new();
                    for line in lines {
                        if let Ok(line_text) = line.Text() {
                             let s = line_text.to_string();
                             if !s.trim().is_empty() {
                                 if !full_text.is_empty() {
                                     full_text.push('\n');
                                 }
                                 full_text.push_str(&s);
                             }
                        }
                    }
                    println!("ocr_core: en-US Text Len: {}", full_text.len());
                    if !full_text.is_empty() {
                        results.push((full_text.len(), full_text));
                    }
                }
            }
        }
    }

    // Pick the one with the longest text
    results.sort_by(|a, b| b.0.cmp(&a.0));
    
    if let Some((_, text)) = results.into_iter().next() {
        println!("ocr_core: best result selected. length: {}", text.len());
        Ok(text)
    } else {
        println!("ocr_core: no recognition results from any engine");
        Ok(String::new())
    }
}

pub fn image_to_base64(image: &DynamicImage) -> String {
    use std::io::Cursor;
    use base64::{Engine as _, engine::general_purpose};
    
    let mut buffer = Cursor::new(Vec::new());
    if let Err(e) = image.write_to(&mut buffer, image::ImageFormat::Png) {
        println!("Error encoding image to base64: {}", e);
        return "".to_string();
    }
    
    let vec = buffer.into_inner();
    general_purpose::STANDARD.encode(&vec)
}
