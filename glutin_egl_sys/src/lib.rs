#![cfg(any(
    windows,
    target_os = "linux",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::manual_non_exhaustive)]
#![allow(clippy::unnecessary_cast)]

pub mod egl;

pub use self::egl::types::{EGLContext, EGLDisplay};

use std::os::raw;

pub type khronos_utime_nanoseconds_t = khronos_uint64_t;
pub type khronos_uint64_t = u64;
pub type khronos_ssize_t = raw::c_long;
pub type EGLint = i32;
pub type EGLenum = raw::c_uint;
pub type EGLNativeDisplayType = *const raw::c_void;

// FIXME: egl_native_pixmap_t instead
#[cfg(windows)]
pub type EGLNativePixmapType = windows_sys::Win32::Graphics::Gdi::HBITMAP;
#[cfg(not(windows))]
pub type EGLNativePixmapType = *const raw::c_void;

#[cfg(windows)]
pub type EGLNativeWindowType = windows_sys::Win32::Foundation::HWND;
#[cfg(not(windows))]
pub type EGLNativeWindowType = *const raw::c_void;
