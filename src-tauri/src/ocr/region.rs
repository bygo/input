use std::fs;
use std::path::{Path, PathBuf};


pub fn capture_screen_rect(x: f64, y: f64, w: f64, h: f64) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        super::macos::capture_screen_rect(x, y, w, h, false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (x, y, w, h);
        Err("截屏取词目前仅支持 macOS".into())
    }
}


pub fn capture_region_retina(x: f64, y: f64, w: f64, h: f64) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        super::macos::capture_screen_rect(x, y, w, h, true)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (x, y, w, h);
        Err("截屏取词目前仅支持 macOS".into())
    }
}


pub fn crop_freeze_jpeg(freeze: &Path, x: f64, y: f64, w: f64, h: f64) -> Result<PathBuf, String> {
    use image::GenericImageView;
    if w < 2.0 || h < 2.0 {
        return Err("选区太小".into());
    }
    let img = image::open(freeze).map_err(|e| format!("读取定格图失败: {e}"))?;
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return Err("定格图无效".into());
    }
    let x0 = (x.floor() as i64).clamp(0, iw as i64 - 1) as u32;
    let y0 = (y.floor() as i64).clamp(0, ih as i64 - 1) as u32;
    let mut x1 = ((x + w).ceil() as i64).clamp(0, iw as i64) as u32;
    let mut y1 = ((y + h).ceil() as i64).clamp(0, ih as i64) as u32;
    if x1 <= x0 {
        x1 = (x0 + 1).min(iw);
    }
    if y1 <= y0 {
        y1 = (y0 + 1).min(ih);
    }
    let cw = x1 - x0;
    let ch = y1 - y0;
    if cw < 2 || ch < 2 {
        return Err("选区太小".into());
    }
    let cropped = img.crop_imm(x0, y0, cw, ch);
    let path = tmp_path("ocr-crop", "jpg")?;
    cropped
        .save_with_format(&path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("写入选区图失败: {e}"))?;
    Ok(path)
}

pub fn tmp_path(prefix: &str, ext: &str) -> Result<PathBuf, String> {
    let dir = crate::settings::dir()?.join("tmp");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(format!(
        "{prefix}-{}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        ext
    )))
}
