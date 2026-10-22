use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State};

mod capture;
mod scroll;
mod session;

use session::{Region, Session};

struct AppState {
    session: Mutex<Option<Session>>,
    cancel: Arc<AtomicBool>,
}

#[tauri::command]
async fn pick_region() -> Result<Option<Region>, String> {
    let out = Command::new("slurp")
        .output()
        .map_err(|e| format!("running slurp: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        return Ok(None);
    }

    let mut parts = text.split_whitespace();
    let xy = parts.next().ok_or_else(|| format!("bad slurp output: {text}"))?;
    let wh = parts.next().ok_or_else(|| format!("bad slurp output: {text}"))?;

    let xy_nums: Vec<i32> = xy
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let wh_nums: Vec<u32> = wh
        .split('x')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if xy_nums.len() != 2 || wh_nums.len() != 2 {
        return Err(format!("could not parse slurp output: {text}"));
    }

    Ok(Some(Region {
        x: xy_nums[0],
        y: xy_nums[1],
        w: wh_nums[0],
        h: wh_nums[1],
    }))
}

#[tauri::command]
async fn start_capture(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    region: Region,
    scroll_key: String,
    scroll_steps: u32,
    scroll_delay_ms: u64,
) -> Result<Option<String>, String> {
    state.cancel.store(false, Ordering::Relaxed);
    let cancel = state.cancel.clone();
    let session = Session::new(region, scroll_key, scroll_steps, scroll_delay_ms, cancel);
    let result = session.run(&app).await;
    let out = result.map_err(|e| e.to_string())?;
    *state.session.lock().unwrap() = Some(session);
    Ok(out.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
async fn capture_single_frame(app: tauri::AppHandle, region: Region) -> Result<String, String> {
    let pictures = dirs::picture_dir().or_else(dirs::home_dir).ok_or("no home dir")?;
    let folder = pictures.join("scrollshot-manual");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    
    let count = std::fs::read_dir(&folder).map(|d| d.count()).unwrap_or(0) + 1;
    let path = folder.join(format!("snap-{:03}.png", count));
    
    let scale = 1.0;
    let scaled = Region {
        x: (region.x as f64 * scale) as i32,
        y: (region.y as f64 * scale) as i32,
        w: (region.w as f64 * scale) as u32,
        h: (region.h as f64 * scale) as u32,
    };

    let frame = capture::capture_region(&scaled).map_err(|e| e.to_string())?;
    frame.save(&path).map_err(|e| e.to_string())?;
    
    let _ = app.emit("progress", serde_json::json!({ "phase": "manual", "frame": count }));
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn cancel_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    Command::new("xdg-open").arg(&path).spawn().map_err(|e| format!("xdg-open: {e}"))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            session: Mutex::new(None),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            pick_region,
            start_capture,
            capture_single_frame,
            cancel_capture,
            open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error running app");
}