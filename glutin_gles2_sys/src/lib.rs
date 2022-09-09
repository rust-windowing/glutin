#![cfg(target_os = "ios")]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::manual_non_exhaustive)]
#![allow(clippy::unnecessary_cast)]

pub mod gles {
    include!(concat!(env!("OUT_DIR"), "/gles2_bindings.rs"));
}

use std::os::raw;

pub const UIViewAutoresizingFlexibleWidth: usize = 1 << 1;
pub const UIViewAutoresizingFlexibleHeight: usize = 1 << 4;

pub const GLKViewDrawableColorFormatRGBA8888: gles::types::GLint = 0;
pub const GLKViewDrawableColorFormatRGB565: gles::types::GLint = 1;
pub const GLKViewDrawableColorFormatSRGBA8888: gles::types::GLint = 2;

pub const GLKViewDrawableDepthFormatNone: gles::types::GLint = 0;
pub const GLKViewDrawableDepthFormat16: gles::types::GLint = 1;
pub const GLKViewDrawableDepthFormat24: gles::types::GLint = 2;

pub const GLKViewDrawableStencilFormatNone: gles::types::GLint = 0;
pub const GLKViewDrawableStencilFormat8: gles::types::GLint = 1;

pub const GLKViewDrawableMultisampleNone: gles::types::GLint = 0;
pub const GLKViewDrawableMultisample4X: gles::types::GLint = 1;

pub const kEAGLRenderingAPIOpenGLES1: usize = 1;
#[allow(dead_code)]
pub const kEAGLRenderingAPIOpenGLES2: usize = 2;
pub const kEAGLRenderingAPIOpenGLES3: usize = 3;

extern "C" {
    pub static kEAGLColorFormatRGB565: *const raw::c_void;
    // pub static kEAGLColorFormatRGBA8: *const raw::c_void;
    pub static kEAGLDrawablePropertyColorFormat: *const raw::c_void;
    pub static kEAGLDrawablePropertyRetainedBacking: *const raw::c_void;
}

pub const RTLD_LAZY: raw::c_int = 0x001;
pub const RTLD_GLOBAL: raw::c_int = 0x100;

extern "C" {
    pub fn dlopen(filename: *const raw::c_char, flag: raw::c_int) -> *mut raw::c_void;
    pub fn dlsym(handle: *mut raw::c_void, symbol: *const raw::c_char) -> *mut raw::c_void;
}
