#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod osmesa;
// mod rawext;

// pub use self::rawext::*;

use crate::context::Context;
use crate::platform::ContextTraitExt;
pub use crate::platform_impl::{ContextPlatformAttributes, RawHandle, SurfacePlatformAttributes};

pub use glutin_egl_sys::EGLContext;
pub use glutin_glx_sys::glx::types::GLXContext;

use std::os::raw;

impl ContextTraitExt for Context {
    type Handle = RawHandle;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.0.raw_handle()
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        self.0.get_egl_display()
    }
}
