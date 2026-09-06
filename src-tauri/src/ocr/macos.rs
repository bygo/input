use super::tmp_path;
use core_foundation::base::TCFType;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::{kCFURLPOSIXPathStyle, CFURL, CFURLRef};
use core_graphics::display::{
    kCGNullWindowID, kCGWindowImageBestResolution, kCGWindowImageNominalResolution,
    kCGWindowListOptionOnScreenOnly, CGPoint, CGRect, CGSize,
};
use core_graphics::image::CGImage;
use foreign_types::ForeignType;
use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGWindowListCreateImage(
        screenBounds: CGRect,
        listOption: u32,
        windowID: u32,
        imageOption: u32,
    ) -> core_graphics::sys::CGImageRef;
}

#[link(name = "ImageIO", kind = "framework")]
extern "C" {
    fn CGImageDestinationCreateWithURL(
        url: CFURLRef,
        type_: CFStringRef,
        count: usize,
        options: *const c_void,
    ) -> *mut c_void;
    fn CGImageDestinationAddImage(
        dest: *mut c_void,
        image: core_graphics::sys::CGImageRef,
        properties: CFDictionaryRef,
    );
    fn CGImageDestinationFinalize(dest: *mut c_void) -> bool;
}


const JPEG_QUALITY_KEY: &str = "kCGImageDestinationLossyCompressionQuality";

pub fn ensure_screen_permission() -> Result<(), String> {
    unsafe {
        if CGPreflightScreenCaptureAccess() {
            return Ok(());
        }
        let _ = CGRequestScreenCaptureAccess();
        if CGPreflightScreenCaptureAccess() {
            return Ok(());
        }
    }
    super::open_screen_recording_settings();
    Err(
        "需要「屏幕录制」权限：系统设置 → 隐私与安全性 → 屏幕录制，开启 input 后完全退出再开"
            .into(),
    )
}

pub fn capture_screen_rect(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    retina: bool,
) -> Result<PathBuf, String> {
    ensure_screen_permission()?;
    if w < 2.0 || h < 2.0 {
        return Err("选区太小".into());
    }
    let rect = CGRect::new(&CGPoint::new(x, y), &CGSize::new(w, h));

    let opt = if retina {
        kCGWindowImageBestResolution
    } else {
        kCGWindowImageNominalResolution
    };
    let image_ptr = unsafe {
        CGWindowListCreateImage(rect, kCGWindowListOptionOnScreenOnly, kCGNullWindowID, opt)
    };
    if image_ptr.is_null() {
        super::open_screen_recording_settings();
        return Err(
            "截屏失败：请到「系统设置 → 隐私与安全性 → 屏幕录制」开启本应用（input）权限，然后完全退出并重开"
                .into(),
        );
    }
    let image = unsafe { CGImage::from_ptr(image_ptr) };
    let path = tmp_path(if retina { "ocr-crop" } else { "ocr-freeze" }, "jpg")?;

    save_jpeg(&image, &path, if retina { 0.9 } else { 0.8 })?;
    Ok(path)
}

fn save_jpeg(image: &CGImage, path: &Path, quality: f32) -> Result<(), String> {
    let path_str = path.to_str().ok_or_else(|| "路径无效".to_string())?;
    let cf_path = CFString::new(path_str);
    let url = CFURL::from_file_system_path(cf_path, kCFURLPOSIXPathStyle, false);
    let uti = CFString::new("public.jpeg");
    let q_key = CFString::new(JPEG_QUALITY_KEY);
    let q_val = CFNumber::from(quality);
    let props: CFDictionary<CFString, CFNumber> =
        CFDictionary::from_CFType_pairs(&[(q_key, q_val)]);

    unsafe {
        let dest = CGImageDestinationCreateWithURL(
            url.as_concrete_TypeRef(),
            uti.as_concrete_TypeRef(),
            1,
            std::ptr::null(),
        );
        if dest.is_null() {
            return Err("无法创建 JPEG 写入器".into());
        }
        CGImageDestinationAddImage(dest, image.as_ptr(), props.as_concrete_TypeRef());
        let ok = CGImageDestinationFinalize(dest);

        core_foundation::base::CFRelease(dest as *const c_void);
        if !ok {
            return Err("JPEG 编码失败".into());
        }
    }
    Ok(())
}

pub fn recognize_file(image: &Path) -> Result<String, String> {
    let script = script_path()?;
    let out = Command::new("swift")
        .arg(&script)
        .arg(image)
        .output()
        .map_err(|e| format!("无法运行 OCR（需要 Xcode Command Line Tools）: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let code = out.status.code().unwrap_or(-1);
        if code == 4 {
            return Err("未识别到文字，请框选更清晰的区域".into());
        }
        return Err(format!("OCR 失败: {err}"));
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        Err("未识别到文字，请框选更清晰的区域".into())
    } else {
        Ok(text)
    }
}

fn script_path() -> Result<PathBuf, String> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ocr.swift"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/ocr.swift"),
    ];
    for p in candidates {
        if p.is_file() {
            return Ok(p);
        }
    }
    let dir = crate::settings::dir()?.join("tmp");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let p = dir.join("ocr.swift");
    fs::write(&p, include_str!("../../resources/ocr.swift")).map_err(|e| e.to_string())?;
    Ok(p)
}
