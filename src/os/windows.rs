#![cfg(target_os = "windows")]

pub use winapi::shared::windef::HGLRC;
pub use winit::os::windows::{DeviceIdExt, WindowBuilderExt, WindowExt, MonitorIdExt};

pub use api::egl::ffi::EGLContext;
pub use platform::RawHandle;

use Context;
use os::GlContextExt;

impl GlContextExt for Context {
    type Handle = RawHandle;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }
}
