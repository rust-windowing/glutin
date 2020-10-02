#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

use crate::platform::ContextTraitExt;
pub use crate::platform_impl::{HeadlessContextExt, RawContextExt, RawHandle};
use crate::{Context, ContextCurrentState};
pub use glutin_egl_sys::EGLContext;
#[cfg(feature = "x11")]
pub use glutin_glx_sys::GLXContext;

pub use winit::platform::unix::*;

use std::os::raw;

impl<T: ContextCurrentState> ContextTraitExt for Context<T> {
    type Handle = RawHandle;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        self.context.get_egl_display()
    }
}
