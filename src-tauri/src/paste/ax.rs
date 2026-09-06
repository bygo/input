use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::{CFString, CFStringRef};
use std::ffi::c_void;
use std::ptr;

pub type AXUIElementRef = *const c_void;
const SUCCESS: i32 = 0;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
    fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: AXUIElementRef,
        parameterized_attribute: CFStringRef,
        parameter: CFTypeRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXValueCreate(type_: u32, value_ptr: *const c_void) -> CFTypeRef;
    fn AXValueGetValue(value: CFTypeRef, type_: u32, value_ptr: *mut c_void) -> u8;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(the_array: CFTypeRef) -> isize;
    fn CFArrayGetValueAtIndex(the_array: CFTypeRef, idx: isize) -> *const c_void;
}

const AX_CGPOINT: u32 = 1;
const AX_CGSIZE: u32 = 2;
const AX_CGRECT: u32 = 3;
const AX_CFRANGE: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
struct CfRange {
    location: isize,
    length: isize,
}

pub struct Elem(AXUIElementRef);

unsafe impl Send for Elem {}
unsafe impl Sync for Elem {}

impl Drop for Elem {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) }
        }
    }
}

impl Elem {
    pub fn focused() -> Option<Self> {
        Self::focused_from(unsafe { AXUIElementCreateSystemWide() }).or_else(|| {
            let pid = super::os::frontmost()?;
            Self::focused_from(unsafe { AXUIElementCreateApplication(pid) })
        })
    }

    fn focused_from(from: AXUIElementRef) -> Option<Self> {
        unsafe {
            if from.is_null() {
                return None;
            }
            let mut value: CFTypeRef = ptr::null();
            let err = AXUIElementCopyAttributeValue(
                from,
                CFString::new("AXFocusedUIElement").as_concrete_TypeRef(),
                &mut value,
            );
            CFRelease(from);
            if err != SUCCESS || value.is_null() {
                return None;
            }
            let el = value as AXUIElementRef;
            let mut pid = 0i32;
            if AXUIElementGetPid(el, &mut pid) == SUCCESS && pid == std::process::id() as i32 {
                CFRelease(el);
                return None;
            }
            Some(Elem(el))
        }
    }

    pub fn focus(&self) -> bool {
        unsafe {
            let mut window: CFTypeRef = ptr::null();
            if AXUIElementCopyAttributeValue(
                self.0,
                CFString::new("AXWindow").as_concrete_TypeRef(),
                &mut window,
            ) == SUCCESS
                && !window.is_null()
            {
                let _ = AXUIElementPerformAction(
                    window as AXUIElementRef,
                    CFString::new("AXRaise").as_concrete_TypeRef(),
                );
                CFRelease(window);
            }
            AXUIElementSetAttributeValue(
                self.0,
                CFString::new("AXFocused").as_concrete_TypeRef(),
                CFBoolean::true_value().as_CFTypeRef(),
            ) == SUCCESS
        }
    }

    fn set_attr_string(&self, name: &str, text: &str) -> bool {
        unsafe {
            AXUIElementSetAttributeValue(
                self.0,
                CFString::new(name).as_concrete_TypeRef(),
                CFString::new(text).as_CFTypeRef(),
            ) == SUCCESS
        }
    }

    pub fn insert(&self, text: &str) -> bool {
        self.set_attr_string("AXSelectedText", text)
    }

    pub fn value(&self) -> Option<String> {
        let v = self.copy_attr("AXValue")?;
        let s = unsafe { CFString::wrap_under_create_rule(v as CFStringRef) };
        Some(s.to_string())
    }

    fn set_value(&self, text: &str) -> bool {
        self.set_attr_string("AXValue", text)
    }

    fn set_selected_range(&self, range: CfRange) -> bool {
        unsafe {
            let v = AXValueCreate(AX_CFRANGE, &range as *const _ as *const c_void);
            if v.is_null() {
                return false;
            }
            let ok = AXUIElementSetAttributeValue(
                self.0,
                CFString::new("AXSelectedTextRange").as_concrete_TypeRef(),
                v,
            ) == SUCCESS;
            CFRelease(v);
            ok
        }
    }

    fn expected_after(before: &str, range: CfRange, text: &str) -> String {
        let u: Vec<u16> = before.encode_utf16().collect();
        let loc = range.location.clamp(0, u.len() as isize) as usize;
        let end = (loc + range.length.max(0) as usize).min(u.len());
        let mut out = String::from_utf16_lossy(&u[..loc]);
        out.push_str(text);
        out.push_str(&String::from_utf16_lossy(&u[end..]));
        out
    }

    fn in_web(&self) -> bool {
        if self.role().as_deref() == Some("AXWebArea") {
            return true;
        }
        let mut cur = self.parent();
        for _ in 0..16 {
            let Some(el) = cur else { break };
            if el.role().as_deref() == Some("AXWebArea") {
                return true;
            }
            cur = el.parent();
        }
        false
    }

    pub fn insert_text(&self, text: &str, replace: bool) -> bool {
        if self.in_web() && !replace {
            return false;
        }
        let before = self.value();
        let range = self.selected_range();
        if self.insert(text) {
            if replace || self.in_web() {
                return true;
            }
            if let Some(after) = self.value() {
                if let (Some(before), Some(range)) = (before.as_deref(), range) {
                    if after == Self::expected_after(before, range, text)
                        || after.contains(text)
                    {
                        return true;
                    }
                } else if after.contains(text) {
                    return true;
                }
            }
        }
        if self.in_web() {
            return false;
        }
        let Some(before) = self.value().or(before) else {
            return false;
        };
        let Some(range) = range.or_else(|| self.selected_range()) else {
            return false;
        };
        let expected = Self::expected_after(&before, range, text);
        if !self.set_value(&expected) {
            return false;
        }
        let caret = range.location.max(0) + text.encode_utf16().count() as isize;
        let _ = self.set_selected_range(CfRange {
            location: caret,
            length: 0,
        });
        self.value()
            .is_some_and(|v| v == expected || v.contains(text))
    }

    pub fn selected_text(&self) -> Option<String> {
        let v = self.copy_attr("AXSelectedText")?;
        let s = unsafe { CFString::wrap_under_create_rule(v as CFStringRef) };
        let t = s.to_string();
        let t = t.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }

    pub fn capture_selected(&self) -> Option<String> {
        if let Some(t) = self.selected_text() {
            return Some(t);
        }
        if let Some(range) = self.selected_range() {
            if range.length > 0 {
                if let Some(t) = self.string_for_range(range) {
                    return Some(t);
                }
            }
        }
        let mut cur = self.parent();
        for _ in 0..8 {
            let Some(el) = cur else { break };
            if let Some(t) = el.selected_text() {
                return Some(t);
            }
            if let Some(range) = el.selected_range() {
                if range.length > 0 {
                    if let Some(t) = el.string_for_range(range) {
                        return Some(t);
                    }
                }
            }
            cur = el.parent();
        }
        self.find_selected_in_descendants(40)
    }

    fn parent(&self) -> Option<Self> {
        let v = self.copy_attr("AXParent")?;
        Some(Elem(v as AXUIElementRef))
    }

    fn children(&self) -> Vec<Self> {
        unsafe {
            let Some(arr) = self.copy_attr("AXChildren") else {
                return Vec::new();
            };
            let n = CFArrayGetCount(arr);
            let mut out = Vec::new();
            if n > 0 {
                out.reserve(n as usize);
            }
            for i in 0..n {
                let item = CFArrayGetValueAtIndex(arr, i);
                if item.is_null() {
                    continue;
                }
                CFRetain(item);
                out.push(Elem(item as AXUIElementRef));
            }
            CFRelease(arr);
            out
        }
    }

    fn string_for_range(&self, range: CfRange) -> Option<String> {
        unsafe {
            let param = AXValueCreate(AX_CFRANGE, &range as *const _ as *const c_void);
            if param.is_null() {
                return None;
            }
            let mut out: CFTypeRef = ptr::null();
            let err = AXUIElementCopyParameterizedAttributeValue(
                self.0,
                CFString::new("AXStringForRange").as_concrete_TypeRef(),
                param,
                &mut out,
            );
            CFRelease(param);
            if err != SUCCESS || out.is_null() {
                return None;
            }
            let s = CFString::wrap_under_create_rule(out as CFStringRef);
            let t = s.to_string();
            let t = t.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
    }

    fn find_selected_in_descendants(&self, max: usize) -> Option<String> {
        let mut stack = self.children();
        let mut seen = 0usize;
        while let Some(el) = stack.pop() {
            seen += 1;
            if seen > max {
                break;
            }
            if let Some(t) = el.selected_text() {
                return Some(t);
            }
            if let Some(range) = el.selected_range() {
                if range.length > 0 {
                    if let Some(t) = el.string_for_range(range) {
                        return Some(t);
                    }
                }
            }
            stack.extend(el.children());
        }
        None
    }

    fn copy_attr(&self, name: &str) -> Option<CFTypeRef> {
        unsafe {
            let mut value: CFTypeRef = ptr::null();
            let err = AXUIElementCopyAttributeValue(
                self.0,
                CFString::new(name).as_concrete_TypeRef(),
                &mut value,
            );
            if err != SUCCESS || value.is_null() {
                None
            } else {
                Some(value)
            }
        }
    }

    fn ax_point(&self, name: &str) -> Option<core_graphics::geometry::CGPoint> {
        let v = self.copy_attr(name)?;
        let mut pt = core_graphics::geometry::CGPoint::new(0.0, 0.0);
        let ok = unsafe { AXValueGetValue(v, AX_CGPOINT, &mut pt as *mut _ as *mut c_void) };
        unsafe { CFRelease(v) };
        (ok != 0).then_some(pt)
    }

    fn ax_size(&self) -> Option<core_graphics::geometry::CGSize> {
        let v = self.copy_attr("AXSize")?;
        let mut size = core_graphics::geometry::CGSize::new(0.0, 0.0);
        let ok = unsafe { AXValueGetValue(v, AX_CGSIZE, &mut size as *mut _ as *mut c_void) };
        unsafe { CFRelease(v) };
        (ok != 0).then_some(size)
    }

    fn role(&self) -> Option<String> {
        let v = self.copy_attr("AXRole")?;
        let s = unsafe { CFString::wrap_under_create_rule(v as CFStringRef) };
        Some(s.to_string())
    }

    fn selected_range(&self) -> Option<CfRange> {
        let v = self.copy_attr("AXSelectedTextRange")?;
        let mut range = CfRange {
            location: 0,
            length: 0,
        };
        let ok = unsafe { AXValueGetValue(v, AX_CFRANGE, &mut range as *mut _ as *mut c_void) };
        unsafe { CFRelease(v) };
        (ok != 0).then_some(range)
    }

    fn bounds_for_range(&self, range: CfRange) -> Option<core_graphics::geometry::CGRect> {
        unsafe {
            let param = AXValueCreate(AX_CFRANGE, &range as *const _ as *const c_void);
            if param.is_null() {
                return None;
            }
            let mut out: CFTypeRef = ptr::null();
            let err = AXUIElementCopyParameterizedAttributeValue(
                self.0,
                CFString::new("AXBoundsForRange").as_concrete_TypeRef(),
                param,
                &mut out,
            );
            CFRelease(param);
            if err != SUCCESS || out.is_null() {
                return None;
            }
            let mut rect = core_graphics::geometry::CGRect::default();
            let ok = AXValueGetValue(out, AX_CGRECT, &mut rect as *mut _ as *mut c_void);
            CFRelease(out);
            (ok != 0).then_some(rect)
        }
    }

    fn caret_rect(&self) -> Option<core_graphics::geometry::CGRect> {
        let range = self.selected_range()?;
        if let Some(rect) = self.bounds_for_range(range) {
            if rect.size.height > 0.0 || range.length == 0 {
                return Some(rect);
            }
        }
        let loc = range.location.max(0);
        let nearby = [
            CfRange {
                location: loc,
                length: 1,
            },
            CfRange {
                location: loc.saturating_sub(1),
                length: 1,
            },
        ];
        for probe in nearby {
            if let Some(rect) = self.bounds_for_range(probe) {
                if rect.size.height <= 0.0 {
                    continue;
                }
                let x = if probe.location < loc {
                    rect.origin.x + rect.size.width
                } else {
                    rect.origin.x
                };
                return Some(core_graphics::geometry::CGRect {
                    origin: core_graphics::geometry::CGPoint {
                        x,
                        y: rect.origin.y,
                    },
                    size: core_graphics::geometry::CGSize {
                        width: 1.0,
                        height: rect.size.height,
                    },
                });
            }
        }
        None
    }

    pub fn input_rect(&self) -> Option<super::InputRect> {
        if let Some(rect) = self.caret_rect() {
            if rect.origin.x.is_finite() && rect.origin.y.is_finite() {
                return Some(super::InputRect::from_ax(
                    rect.origin.x,
                    rect.origin.y,
                    rect.size.width,
                    rect.size.height,
                ));
            }
        }
        let pos = self.ax_point("AXPosition")?;
        let size = self.ax_size()?;
        if !pos.x.is_finite() || !pos.y.is_finite() || size.width < 4.0 || size.height < 4.0 {
            return None;
        }
        let role = self.role().unwrap_or_default();
        let is_text = matches!(
            role.as_str(),
            "AXTextField"
                | "AXTextArea"
                | "AXComboBox"
                | "AXSearchField"
                | "AXStaticText"
                | "AXEditableText"
        ) || role.contains("Text");
        if !is_text && (role == "AXWebArea" || role == "AXWindow" || role == "AXGroup") {
            return None;
        }
        if size.height > 480.0 && !is_text {
            return None;
        }
        let height = if size.height > 80.0 {
            22.0
        } else {
            size.height
        };
        Some(super::InputRect::from_ax(pos.x, pos.y, size.width, height))
    }
}
