use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};
use std::cell::RefCell;
use std::ffi::CString;
use std::path::Path;
use std::process::{Child, Command};

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

thread_local! {
    static SYNTH: RefCell<Option<(*mut AnyObject, String)>> = const { RefCell::new(None) };
    static AFPLAY: RefCell<Option<Child>> = const { RefCell::new(None) };
}

pub fn stop_afplay() {
    AFPLAY.with(|slot| {
        if let Some(mut child) = slot.borrow_mut().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    });
}

fn afplay_playing() -> bool {
    AFPLAY.with(|slot| {
        let mut g = slot.borrow_mut();
        match g.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) | Err(_) => {
                    *g = None;
                    false
                }
            },
            None => false,
        }
    })
}

pub fn play_afplay(path: &Path) {
    stop_afplay();
    if let Ok(child) = Command::new("afplay").arg(path).spawn() {
        AFPLAY.with(|slot| *slot.borrow_mut() = Some(child));
    }
}

fn ns_string(s: &str) -> *mut AnyObject {
    let c = CString::new(s).unwrap_or_default();
    unsafe {
        let cls = AnyClass::get(c"NSString").unwrap();
        msg_send![cls, stringWithUTF8String: c.as_ptr()]
    }
}

fn synth(voice: Option<&str>) -> *mut AnyObject {
    SYNTH.with(|slot| {
        let key = voice.unwrap_or("").to_string();
        let mut g = slot.borrow_mut();
        if let Some((p, v)) = g.as_ref() {
            if *v == key && !p.is_null() {
                return *p;
            }
        }
        unsafe {
            let cls = AnyClass::get(c"NSSpeechSynthesizer").unwrap();
            let syn: *mut AnyObject = msg_send![cls, new];
            if let Some(want) = voice {
                let voices: *mut AnyObject = msg_send![cls, availableVoices];
                let n: usize = msg_send![voices, count];
                let name_key = ns_string("VoiceName");
                for i in 0..n {
                    let vid: *mut AnyObject = msg_send![voices, objectAtIndex: i];
                    let attrs: *mut AnyObject = msg_send![cls, attributesForVoice: vid];
                    if attrs.is_null() {
                        continue;
                    }
                    let name: *mut AnyObject = msg_send![attrs, objectForKey: name_key];
                    if name.is_null() {
                        continue;
                    }
                    let utf8: *const std::ffi::c_char = msg_send![name, UTF8String];
                    if utf8.is_null() {
                        continue;
                    }
                    let s = std::ffi::CStr::from_ptr(utf8).to_string_lossy();
                    if s.eq_ignore_ascii_case(want) {
                        let _: bool = msg_send![syn, setVoice: vid];
                        break;
                    }
                }
            }
            *g = Some((syn, key));
            syn
        }
    })
}

pub fn stop() {
    SYNTH.with(|slot| {
        if let Some((p, _)) = slot.borrow().as_ref() {
            if !p.is_null() {
                unsafe {
                    let _: () = msg_send![*p, stopSpeaking];
                }
            }
        }
    });
}

pub fn is_speaking() -> bool {
    let synth = SYNTH.with(|slot| {
        if let Some((p, _)) = slot.borrow().as_ref() {
            if !p.is_null() {
                let on: bool = unsafe { msg_send![*p, isSpeaking] };
                return on;
            }
        }
        false
    });
    synth || afplay_playing()
}

pub fn speak(word: &str, voice: Option<&str>) {
    let syn = synth(voice);
    let s = ns_string(word);
    unsafe {
        let _: () = msg_send![syn, stopSpeaking];
        let _: bool = msg_send![syn, startSpeakingString: s];
    }
}


pub fn warm(voice: Option<&str>) {
    let syn = synth(voice);
    let s = ns_string(" ");
    unsafe {
        let _: bool = msg_send![syn, startSpeakingString: s];
        let _: () = msg_send![syn, stopSpeaking];
    }
}
