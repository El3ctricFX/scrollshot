use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::{capture, scroll};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub struct Session {
    pub region: Region,
    pub scroll_key: String,
    pub scroll_steps: u32,
    pub scroll_delay_ms: u64,
    pub max_frames: usize,
    pub cancel: Arc<AtomicBool>,
}

impl Session {
    pub fn new(
        region: Region,
        scroll_key: String,
        scroll_steps: u32,
        scroll_delay_ms: u64,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        Self {
            region,
            scroll_key,
            scroll_steps,
            scroll_delay_ms,
            max_frames: 200,
            cancel,
        }
    }

    pub async fn run(&self, app: &AppHandle) -> Result<Option<PathBuf>> {
        let scale = output_scale_at(self.region.x, self.region.y);
        
        let scaled = Region {
            x: (self.region.x as f64 * scale) as i32,
            y: (self.region.y as f64 * scale) as i32,
            w: (self.region.w as f64 * scale) as u32,
            h: (self.region.h as f64 * scale) as u32,
        };
        eprintln!("[session] starting scale={scale}, steps={}", self.scroll_steps);

        let pictures = dirs::picture_dir()
            .or_else(dirs::home_dir)
            .ok_or_else(|| anyhow::anyhow!("no home directory"))?;
        let stamp = date_stamp();
        let session_dir = pictures.join(format!("scrollshot-{stamp}"));
        std::fs::create_dir_all(&session_dir)?;

        let mut frame_count: usize = 0;
        let mut identical_streak = 0usize;
        let required_streak = 5;
        let mut last_frame: Option<RgbaImage> = None;

        for i in 0..self.max_frames {
            if self.cancel.load(Ordering::Relaxed) {
                break;
            }

            let frame = capture::capture_region(&scaled)?;

            if let Some(prev) = &last_frame {
                if frames_identical(prev, &frame) {
                    identical_streak += 1;
                    eprintln!("[session] iter {i}: identical (streak {}/{})", identical_streak, required_streak);
                    if identical_streak >= required_streak {
                        eprintln!("[session] confirmed reached bottom");
                        break;
                    }
                    scroll::scroll_down(&self.scroll_key, self.scroll_steps)?;
                    tokio::time::sleep(Duration::from_millis(self.scroll_delay_ms + 300)).await;
                    continue;
                } else {
                    identical_streak = 0;
                }
            }

            frame_count += 1;
            let path = session_dir.join(format!("frame-{:03}.png", frame_count));
            frame.save(&path)?;
            eprintln!("[session] saved {}", path.display());

            app.emit("progress", serde_json::json!({ "phase": "capture", "frame": frame_count })).ok();

            last_frame = Some(frame);

            scroll::scroll_down(&self.scroll_key, self.scroll_steps)?;
            tokio::time::sleep(Duration::from_millis(self.scroll_delay_ms)).await;
        }

        if frame_count == 0 {
            bail!("nothing captured");
        }

        Ok(Some(session_dir))
    }
}

fn frames_identical(a: &RgbaImage, b: &RgbaImage) -> bool {
    if a.dimensions() != b.dimensions() { return false; }
    let total = (a.width() as u64) * (a.height() as u64);
    let step = (total / 1000).max(1);
    let mut diffs = 0u32;
    let mut i = 0u64;
    while i < total {
        let x = (i % a.width() as u64) as u32;
        let y = (i / a.width() as u64) as u32;
        let pa = a.get_pixel(x, y);
        let pb = b.get_pixel(x, y);
        if pa[0].abs_diff(pb[0]) > 8 || pa[1].abs_diff(pb[1]) > 8 || pa[2].abs_diff(pb[2]) > 8 {
            diffs += 1;
            if diffs > 20 { return false; }
        }
        i += step;
    }
    true
}

fn date_stamp() -> String {
    if let Ok(out) = Command::new("date").args(["+%Y-%m-%d-%H%M%S"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() { return s; }
        }
    }
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

fn output_scale_at(x: i32, y: i32) -> f64 {
    let out = match Command::new("swaymsg").args(["-t", "get_outputs", "-r"]).output() {
        Ok(o) => o,
        Err(_) => return 1.0,
    };
    let json: serde_json::Value = match serde_json::from_slice(&out.stdout) {
        Ok(j) => j,
        Err(_) => return 1.0,
    };
    if let Some(outputs) = json.as_array() {
        for o in outputs {
            let ox = o["rect"]["x"].as_i64().unwrap_or(0) as i32;
            let oy = o["rect"]["y"].as_i64().unwrap_or(0) as i32;
            let ow = o["rect"]["width"].as_i64().unwrap_or(0) as i32;
            let oh = o["rect"]["height"].as_i64().unwrap_or(0) as i32;
            if x >= ox && x < ox + ow && y >= oy && y < oy + oh {
                return o["scale"].as_f64().unwrap_or(1.0);
            }
        }
    }
    1.0
}