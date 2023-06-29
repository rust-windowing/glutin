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

    // TODO should upstream these:
    // EGL_EXT_platform_xcb
    pub const PLATFORM_XCB_EXT: super::EGLenum = 0x31DC;
    pub const PLATFORM_XCB_SCREEN_EXT: super::EGLenum = 0x31DE;
    // EGL_EXT_device_query_name
    pub const RENDERER_EXT: super::EGLenum = 0x335F;
    // EGL_ANGLE_platform_angle - https://chromium.googlesource.com/angle/angle/+/HEAD/extensions/EGL_ANGLE_platform_angle.txt
    pub const PLATFORM_ANGLE_ANGLE: super::EGLenum = 0x3202;
    pub const PLATFORM_ANGLE_TYPE_ANGLE: super::EGLenum = 0x3203;
    pub const PLATFORM_ANGLE_MAX_VERSION_MAJOR_ANGLE: super::EGLenum = 0x3204;
    pub const PLATFORM_ANGLE_MAX_VERSION_MINOR_ANGLE: super::EGLenum = 0x3205;
    pub const PLATFORM_ANGLE_DEBUG_LAYERS_ENABLED: super::EGLenum = 0x3451;
    pub const PLATFORM_ANGLE_NATIVE_PLATFORM_TYPE_ANGLE: super::EGLenum = 0x348F;
    pub const PLATFORM_ANGLE_TYPE_DEFAULT_ANGLE: super::EGLenum = 0x3206;
    pub const PLATFORM_ANGLE_DEVICE_TYPE_HARDWARE_ANGLE: super::EGLenum = 0x320A;
    pub const PLATFORM_ANGLE_DEVICE_TYPE_NULL_ANGLE: super::EGLenum = 0x345E;
}

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
