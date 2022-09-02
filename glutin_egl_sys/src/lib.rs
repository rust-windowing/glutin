#![cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#![allow(
    clippy::manual_non_exhaustive,
    clippy::missing_safety_doc,
    clippy::unnecessary_cast,
    non_camel_case_types
)]
#![cfg_attr(feature = "cargo-clippy", deny(warnings))]

pub mod egl {
    pub type khronos_utime_nanoseconds_t = super::khronos_utime_nanoseconds_t;
    pub type khronos_uint64_t = super::khronos_uint64_t;
    pub type khronos_ssize_t = super::khronos_ssize_t;
    pub type EGLNativeDisplayType = super::EGLNativeDisplayType;
    pub type EGLNativePixmapType = super::EGLNativePixmapType;
    pub type EGLNativeWindowType = super::EGLNativeWindowType;
    pub type EGLint = super::EGLint;
    pub type NativeDisplayType = super::EGLNativeDisplayType;
    pub type NativePixmapType = super::EGLNativePixmapType;
    pub type NativeWindowType = super::EGLNativeWindowType;

    include!(concat!(env!("OUT_DIR"), "/egl_bindings.rs"));
}

pub use self::egl::types::EGLContext;
pub use self::egl::types::EGLDisplay;

use std::os::raw;

pub type khronos_utime_nanoseconds_t = khronos_uint64_t;
pub type khronos_uint64_t = u64;
pub type khronos_ssize_t = raw::c_long;
pub type EGLint = i32;
pub type EGLNativeDisplayType = *const raw::c_void;
pub type EGLNativePixmapType = *const raw::c_void; // FIXME: egl_native_pixmap_t instead

#[cfg(target_os = "windows")]
pub type EGLNativeWindowType = winapi::shared::windef::HWND;
#[cfg(target_os = "linux")]
pub type EGLNativeWindowType = *const raw::c_void;
#[cfg(target_os = "android")]
pub type EGLNativeWindowType = *const raw::c_void;
#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub type EGLNativeWindowType = *const raw::c_void;
