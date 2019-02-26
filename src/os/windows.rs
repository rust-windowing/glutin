#![cfg(target_os = "windows")]

pub use winapi::shared::windef::HGLRC;
pub use winit::os::windows::{
    DeviceIdExt, MonitorIdExt, WindowBuilderExt, WindowExt,
};

pub use api::egl::ffi::EGLContext;
pub use platform::RawHandle;

use std::os::raw;

use os::ContextTraitExt;
use Context;

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
