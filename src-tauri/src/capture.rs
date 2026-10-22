use anyhow::{Context, Result};
use image::RgbaImage;
use std::process::Command;

use crate::session::Region;

pub fn capture_region(region: &Region) -> Result<RgbaImage> {
    let geom = format!("{},{} {}x{}", region.x, region.y, region.w, region.h);
    let out = Command::new("grim")
        .args(["-g", &geom, "-"])
        .output()
        .context("running grim — is it on PATH?")?;
    if !out.status.success() {
        anyhow::bail!("grim failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(image::load_from_memory(&out.stdout)?.to_rgba8())
}