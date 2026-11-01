//! C ABI (include/uim.h). Every call is panic-safe: a panic resets the engine instead of
//! unwinding into the shell.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::Uim;

pub struct UimHandle {
    uim: Uim,
    layout: String,
    candidates: Vec<String>,
}

fn with<T>(h: *mut UimHandle, default: T, f: impl FnOnce(&mut UimHandle) -> T) -> T {
    // SAFETY: the shell passes a pointer from uim_new that it hasn't freed.
    let Some(h) = (unsafe { h.as_mut() }) else { return default };
    match catch_unwind(AssertUnwindSafe(|| f(h))) {
        Ok(v) => v,
        Err(_) => {
            h.uim = Uim::new();
            default
        }
    }
}

fn refresh(h: &mut UimHandle) {
    h.layout = h.uim.layout_text();
    h.candidates = h.uim.candidates();
}

unsafe fn text(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: the caller passes len readable bytes.
    String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(ptr, len) }).into_owned()
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_new() -> *mut UimHandle {
    let mut h = UimHandle { uim: Uim::new(), layout: String::new(), candidates: Vec::new() };
    refresh(&mut h);
    Box::into_raw(Box::new(h))
}

/// # Safety
/// `h` must come from `uim_new` and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uim_free(h: *mut UimHandle) {
    if !h.is_null() {
        // SAFETY: per the contract above.
        drop(unsafe { Box::from_raw(h) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_set_wide(h: *mut UimHandle, wide: u32) {
    with(h, (), |h| {
        h.uim.set_wide(wide != 0);
        refresh(h)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_set_bottom(h: *mut UimHandle, bottom: u32) {
    with(h, (), |h| {
        h.uim.set_bottom(bottom as u8);
        refresh(h)
    })
}

/// # Safety
/// `ptr` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uim_load_words(h: *mut UimHandle, ptr: *const u8, len: usize) {
    // SAFETY: per the contract above.
    let t = unsafe { text(ptr, len) };
    with(h, (), |h| h.uim.load_words(&t))
}

/// # Safety
/// `ptr` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uim_load_bigrams(h: *mut UimHandle, ptr: *const u8, len: usize) {
    // SAFETY: per the contract above.
    let t = unsafe { text(ptr, len) };
    with(h, (), |h| h.uim.load_bigrams(&t))
}

/// # Safety
/// `ptr` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uim_load_acronyms(h: *mut UimHandle, ptr: *const u8, len: usize) {
    // SAFETY: per the contract above.
    let t = unsafe { text(ptr, len) };
    with(h, (), |h| h.uim.load_acronyms(&t))
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_press(h: *mut UimHandle, row: u32, col: u32, dir: u32) {
    with(h, (), |h| {
        h.uim.press(row as usize, col as usize, dir as usize);
        refresh(h)
    })
}

/// A HOLD key went down (1) or up (0); the tap, if any, is a separate `uim_press`.
#[unsafe(no_mangle)]
pub extern "C" fn uim_hold(h: *mut UimHandle, on: u32) {
    with(h, (), |h| {
        h.uim.hold(on != 0);
        refresh(h)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_drag(h: *mut UimHandle, steps: i32) {
    with(h, (), |h| {
        h.uim.drag(steps);
        refresh(h)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_candidate(h: *mut UimHandle, index: u32) {
    with(h, (), |h| {
        h.uim.candidate(index as usize);
        refresh(h)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_flush(h: *mut UimHandle) {
    with(h, (), |h| {
        h.uim.flush();
        refresh(h)
    })
}

macro_rules! string_getter {
    ($ptr:ident, $len:ident, $get:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $ptr(h: *mut UimHandle) -> *const u8 {
            with(h, std::ptr::null(), |h| { let f: fn(&UimHandle) -> &str = $get; f(h).as_ptr() })
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn $len(h: *mut UimHandle) -> usize {
            with(h, 0, |h| { let f: fn(&UimHandle) -> &str = $get; f(h).len() })
        }
    };
}

string_getter!(uim_commit_ptr, uim_commit_len, |h| &h.uim.output().commit);
string_getter!(uim_preedit_ptr, uim_preedit_len, |h| &h.uim.output().preedit);
string_getter!(uim_layout_ptr, uim_layout_len, |h| &h.layout);

#[unsafe(no_mangle)]
pub extern "C" fn uim_action(h: *mut UimHandle) -> u32 {
    with(h, 0, |h| h.uim.output().action as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_action_arg(h: *mut UimHandle) -> i32 {
    with(h, 0, |h| h.uim.output().arg)
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_candidate_count(h: *mut UimHandle) -> u32 {
    with(h, 0, |h| h.candidates.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_candidate_ptr(h: *mut UimHandle, i: u32) -> *const u8 {
    with(h, std::ptr::null(), |h| h.candidates.get(i as usize).map_or(std::ptr::null(), |s| s.as_ptr()))
}

#[unsafe(no_mangle)]
pub extern "C" fn uim_candidate_len(h: *mut UimHandle, i: u32) -> usize {
    with(h, 0, |h| h.candidates.get(i as usize).map_or(0, |s| s.len()))
}

// ---- JA kana-kanji conversion data (data/kanji.bin)

/// Parses `data/kanji.bin` into a shareable handle, or null if it doesn't parse. Touches no
/// engine, so the host can call it off the UI thread, then hand the result to `uim_set_kanji`.
///
/// # Safety
/// `ptr` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uim_kanji_new(ptr: *const u8, len: usize) -> *const crate::kanji::Kanji {
    if ptr.is_null() {
        return std::ptr::null();
    }
    // SAFETY: per the contract above.
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    match catch_unwind(|| crate::kanji::Kanji::parse(data)) {
        Ok(Ok(k)) => std::sync::Arc::into_raw(std::sync::Arc::new(k)),
        _ => std::ptr::null(),
    }
}

/// Releases the host's reference; engines that were given it keep their own.
///
/// # Safety
/// `k` must come from `uim_kanji_new` (or be null) and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uim_kanji_free(k: *const crate::kanji::Kanji) {
    if !k.is_null() {
        // SAFETY: per the contract above.
        drop(unsafe { std::sync::Arc::from_raw(k) });
    }
}

/// Gives the engine the converter (null: none). The engine takes its own reference.
///
/// # Safety
/// `k` must be null or a live handle from `uim_kanji_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uim_set_kanji(h: *mut UimHandle, k: *const crate::kanji::Kanji) {
    let kanji = if k.is_null() {
        None
    } else {
        // SAFETY: per the contract above; the increment pairs with the Arc made here.
        unsafe {
            std::sync::Arc::increment_strong_count(k);
            Some(std::sync::Arc::from_raw(k))
        }
    };
    with(h, (), |h| {
        h.uim.set_kanji(kanji);
        refresh(h)
    })
}
