use anyhow::{Context, Result};
use std::process::Command;

pub fn scroll_down(key: &str, steps: u32) -> Result<()> {
    if which::which("wtype").is_ok() {
        match wtype_scroll(key, steps) {
            Ok(()) => return Ok(()),
            Err(e) => eprintln!("[scroll] wtype failed: {e}"),
        }
    }
    if which::which("ydotool").is_ok() {
        return ydotool_scroll(key, steps);
    }
    anyhow::bail!("no scroll backend available (need wtype or ydotool)");
}

fn wtype_scroll(key: &str, steps: u32) -> Result<()> {
    // If we're sending "Down", send it `steps` times with a tiny delay.
    // Otherwise, just send the key once.
    let count = if key == "Down" { steps } else { 1 };
    
    for _ in 0..count {
        let out = Command::new("wtype")
            .args(["-k", key])
            .output()
            .context("running wtype")?;
        if !out.status.success() {
            anyhow::bail!("wtype: {}", String::from_utf8_lossy(&out.stderr));
        }
        if key == "Down" {
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
    eprintln!("[scroll] wtype sent {key} x{count}");
    Ok(())
}

fn ydotool_scroll(key: &str, steps: u32) -> Result<()> {
    let code = match key {
        "Page_Down" | "PageDown" | "Next" => "117",
        "Down" => "108",
        _ => anyhow::bail!("unsupported key for ydotool: {key}"),
    };
    let count = if key == "Down" { steps } else { 1 };

    for _ in 0..count {
        let out = Command::new("ydotool")
            .args(["key", &format!("{code}:1"), &format!("{code}:0")])
            .output()
            .context("running ydotool")?;
        if !out.status.success() {
            anyhow::bail!("ydotool: {}", String::from_utf8_lossy(&out.stderr));
        }
        if key == "Down" {
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
    eprintln!("[scroll] ydotool sent {key} x{count}");
    Ok(())
}