#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use super::ffi;
use libc;

pub type khronos_utime_nanoseconds_t = khronos_uint64_t;
pub type khronos_uint64_t = libc::uint64_t;
pub type khronos_ssize_t = libc::c_long;
pub type EGLNativeDisplayType = *mut ffi::Display;
pub type EGLNativePixmapType = ffi::Pixmap;
pub type EGLNativeWindowType = ffi::Window;
pub type EGLint = libc::int32_t;
pub type NativeDisplayType = *mut ffi::Display;
pub type NativePixmapType = ffi::Pixmap;
pub type NativeWindowType = ffi::Window;

include!(concat!(env!("OUT_DIR"), "/egl_bindings.rs"));

#[link(name = "EGL")]
#[link(name = "GLESv2")]
extern {}
