#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]

use crate::os::ContextTraitExt;
pub use crate::platform::{OsMesaContextExt, RawContextExt, RawHandle};
use crate::Context;
pub use glutin_egl_sys::EGLContext;
pub use glutin_glx_sys::GLXContext;

pub use winit::os::unix::EventsLoopExt;
pub use winit::os::unix::MonitorIdExt;
pub use winit::os::unix::WindowBuilderExt;
pub use winit::os::unix::WindowExt;
pub use winit::os::unix::XNotSupported;
pub use winit::os::unix::XWindowType;

use std::os::raw;

impl ContextTraitExt for Context {
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
