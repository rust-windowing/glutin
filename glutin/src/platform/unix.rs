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

pub use crate::api::egl::ffi::EGLContext;
pub use crate::api::glx::ffi::glx::types::GLXContext;
use crate::context::Context;
use crate::platform::ContextTraitExt;
pub use crate::platform_impl::{
    BackingApi, ContextPlatformAttributes, DisplayPlatformAttributes, RawHandle,
    SurfacePlatformAttributes,
};

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
