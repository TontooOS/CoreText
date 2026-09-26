//! Minimal C ABI for CoreText (version + measure).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use vello::peniko::Color;

/// Library version string (borrowed; do not free).
#[no_mangle]
pub extern "C" fn coretext_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

/// Measure `text` at `size` px (regular weight). Writes logical
/// width/height to `out_w`/`out_h`. Returns 1 on success.
#[no_mangle]
pub extern "C" fn coretext_measure(
    text: *const c_char,
    size: f32,
    scale: f32,
    out_w: *mut f32,
    out_h: *mut f32,
) -> i32 {
    if text.is_null() || out_w.is_null() || out_h.is_null() {
        return 0;
    }
    let content = unsafe { CStr::from_ptr(text).to_string_lossy().into_owned() };
    let mut setter = crate::typeset::CTFramesetter::new(scale.max(0.5));
    let (w, h) = setter.measure(&content, size, Color::WHITE, 400.0, None);
    unsafe {
        *out_w = w;
        *out_h = h;
    }
    1
}

/// Free a string returned by CoreText (for future heap returns).
#[no_mangle]
pub extern "C" fn coretext_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}
