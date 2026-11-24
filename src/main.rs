use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "m4v", "webm"];

// --- JSON Deserialization Structs ---

#[derive(Deserialize, Debug)]
struct FfprobeOutput {
    streams: Option<Vec<Stream>>,
}

#[derive(Deserialize, Debug)]
struct Stream {
    index: usize,
    codec_type: Option<String>,
    channels: Option<u32>,
    // ffprobe outputs bit_rate as a string in JSON (or omits it for lossless codecs like TrueHD/DTS-HD)
    bit_rate: Option<String>,
    tags: Option<HashMap<String, String>>,
}

// --- Core Logic ---

fn get_media_info(file_path: &Path) -> Option<FfprobeOutput> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            "-show_format",
        ])
        .arg(file_path)
        .output()
        .ok()?;

    if !output.status.success() {
        eprintln!("Error probing {:?}", file_path.file_name().unwrap_or_default());
        return None;
    }

    serde_json::from_slice(&output.stdout).ok()
}

fn find_best_english_audio(streams: &[Stream]) -> Option<usize> {
    streams
        .iter()
        .filter(|s| s.codec_type.as_deref() == Some("audio"))
        .filter(|s| {
            s.tags
                .as_ref()
                .and_then(|tags| {
                    // Check both lowercase "language" and uppercase "LANGUAGE" (common in MKVs)
                    tags.get("language").or_else(|| tags.get("LANGUAGE"))
                })
                .map(|lang| matches!(lang.to_lowercase().as_str(), "eng" | "en"))
                .unwrap_or(false)
        })
        .max_by_key(|s| {
            let channels = s.channels.unwrap_or(2);
            let bitrate = s
                .bit_rate
                .as_deref()
                .and_then(|b| b.parse::<u64>().ok())
                .unwrap_or(0);
            (channels, bitrate)
        })
        .map(|s| s.index)
}

fn process_video(file_path: &Path) {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    println!("\nProcessing: {}", file_name);

    let media_info = match get_media_info(file_path) {
        Some(info) => info,
        None => {
            println!("Skipping {} due to probe failure.", file_name);
            return;
        }
    };

    let streams = match media_info.streams {
        Some(s) => s,
        None => {
            println!("Skipping {}: no streams found.", file_name);
            return;
        }
    };

    let best_audio_index = match find_best_english_audio(&streams) {
        Some(idx) => idx,
        None => {
            println!("-> Warning: No English audio track found for {}. Skipping.", file_name);
            return;
        }
    };

    println!("-> Selected English audio stream index: {}", best_audio_index);

    let temp_path = file_path.with_file_name(format!("temp_{}", file_name));

    let output = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(file_path)
        .args([
            "-map", "0:v:0",
            "-map", &format!("0:{}", best_audio_index),
            "-c:v", "copy",
            "-c:a", "aac",
            "-b:a", "192k",
            "-ac", "2",
            "-ar", "48000",
            "-map_metadata", "-1",
            "-map_chapters", "-1",
        ])
        .arg(&temp_path)
        .output();

    match output {
        Ok(res) if res.status.success() => {
            // Calculate sizes and atomically replace original file
            let orig_size = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
            let new_size = fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);
            let saved_bytes = orig_size as i64 - new_size as i64;
            let saved_mb = saved_bytes as f64 / (1024.0 * 1024.0);

            if let Err(e) = fs::rename(&temp_path, file_path) {
                eprintln!("-> Failed to replace original file: {}", e);
                let _ = fs::remove_file(&temp_path);
            } else {
                println!("-> Success! Saved {:.2} MB ({} bytes).", saved_mb, saved_bytes);
            }
        }
        Ok(res) => {
            eprintln!("-> Error processing {} with ffmpeg.", file_name);
            eprintln!("-> FFmpeg Error Output:\n{}", String::from_utf8_lossy(&res.stderr));
            let _ = fs::remove_file(&temp_path);
        }
        Err(e) => {
            eprintln!("-> Failed to execute ffmpeg: {}", e);
            let _ = fs::remove_file(&temp_path);
        }
    }
}

fn main() {
    let current_dir = Path::new(".");
    let mut video_files: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                        video_files.push(path);
                    }
                }
            }
        }
    }

    if video_files.is_empty() {
        println!("No supported video files found in the current directory.");
        return;
    }

    // Sort files alphabetically for predictable queue ordering
    video_files.sort();

    println!("Found {} video file(s) to process. Queuing one by one...", video_files.len());

    for video_file in &video_files {
        process_video(video_file);
    }

    println!("\nAll tasks completed!");
}
