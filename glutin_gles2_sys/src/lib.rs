#![cfg(target_os = "ios")]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

pub mod gles {
    include!(concat!(env!("OUT_DIR"), "/gles2_bindings.rs"));
}

use objc::runtime::Object;
use objc::{Encode, Encoding};

use std::os::raw;

pub type id = *mut Object;
pub const nil: id = 0 as id;

pub const UIViewAutoresizingFlexibleWidth: NSUInteger = 1 << 1;
pub const UIViewAutoresizingFlexibleHeight: NSUInteger = 1 << 4;

#[cfg(target_pointer_width = "32")]
pub type CGFloat = f32;
#[cfg(target_pointer_width = "64")]
pub type CGFloat = f64;

#[cfg(target_pointer_width = "32")]
pub type NSUInteger = u32;
#[cfg(target_pointer_width = "64")]
pub type NSUInteger = u64;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

unsafe impl Encode for CGRect {
    fn encode() -> Encoding {
        #[cfg(target_pointer_width = "32")]
        unsafe {
            Encoding::from_str("{CGRect={CGPoint=ff}{CGSize=ff}}")
        }
        #[cfg(target_pointer_width = "64")]
        unsafe {
            Encoding::from_str("{CGRect={CGPoint=dd}{CGSize=dd}}")
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

pub const GLKViewDrawableColorFormatRGBA8888: NSUInteger = 0;
pub const GLKViewDrawableColorFormatRGB565: NSUInteger = 1;
pub const GLKViewDrawableColorFormatSRGBA8888: NSUInteger = 2;

pub const GLKViewDrawableDepthFormatNone: NSUInteger = 0;
pub const GLKViewDrawableDepthFormat16: NSUInteger = 1;
pub const GLKViewDrawableDepthFormat24: NSUInteger = 2;

pub const GLKViewDrawableStencilFormatNone: NSUInteger = 0;
pub const GLKViewDrawableStencilFormat8: NSUInteger = 1;

pub const GLKViewDrawableMultisampleNone: NSUInteger = 0;
pub const GLKViewDrawableMultisample4X: NSUInteger = 1;

pub const kEAGLRenderingAPIOpenGLES1: NSUInteger = 1;
#[allow(dead_code)]
pub const kEAGLRenderingAPIOpenGLES2: NSUInteger = 2;
pub const kEAGLRenderingAPIOpenGLES3: NSUInteger = 3;

extern "C" {
    pub static kEAGLColorFormatRGB565: id;
    // pub static kEAGLColorFormatRGBA8: id;
    pub static kEAGLDrawablePropertyColorFormat: id;
    pub static kEAGLDrawablePropertyRetainedBacking: id;
}

pub const RTLD_LAZY: raw::c_int = 0x001;
pub const RTLD_GLOBAL: raw::c_int = 0x100;

extern "C" {
    pub fn dlopen(filename: *const raw::c_char, flag: raw::c_int) -> *mut raw::c_void;
    pub fn dlsym(handle: *mut raw::c_void, symbol: *const raw::c_char) -> *mut raw::c_void;
}
