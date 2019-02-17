#![cfg(any(target_os = "android"))]

pub use winit::os::android::{WindowBuilderExt, WindowExt};

pub use api::egl::ffi::EGLContext;

use os::ContextTraitExt;
use Context;

use std::os::raw;

impl ContextTraitExt for Context {
    type Handle = EGLContext;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        Some(self.context.get_egl_display())
    }
}
