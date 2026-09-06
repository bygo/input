use std::sync::Mutex;
use windows::core::PCWSTR;
use windows::Win32::Media::Speech::{
    ISpVoice, SpVoice, SPF_ASYNC, SPF_IS_NOT_XML, SPF_PURGEBEFORESPEAK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};


struct Voice(ISpVoice);
unsafe impl Send for Voice {}
unsafe impl Sync for Voice {}

static VOICE: Mutex<Option<Voice>> = Mutex::new(None);

fn voice() -> Result<ISpVoice, String> {

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut g = VOICE.lock().map_err(|e| e.to_string())?;
    if let Some(v) = g.as_ref() {
        return Ok(v.0.clone());
    }
    unsafe {
        let v: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_ALL)
            .map_err(|e| format!("初始化语音失败: {e}"))?;
        *g = Some(Voice(v.clone()));
        Ok(v)
    }
}

pub fn warm() {
    let _ = voice();
}

pub fn speak(word: &str) -> Result<(), String> {
    let v = voice()?;
    let wide: Vec<u16> = word.encode_utf16().chain(std::iter::once(0)).collect();
    let flags = (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0 | SPF_IS_NOT_XML.0) as u32;
    unsafe {
        v.Speak(PCWSTR(wide.as_ptr()), flags, None)
            .map_err(|e| format!("发音失败: {e}"))?;
    }
    Ok(())
}

pub fn stop() -> Result<(), String> {
    let v = voice()?;
    let empty: Vec<u16> = vec![0];
    let flags = SPF_PURGEBEFORESPEAK.0 as u32;
    unsafe {
        v.Speak(PCWSTR(empty.as_ptr()), flags, None)
            .map_err(|e| format!("停止发音失败: {e}"))?;
    }
    Ok(())
}
