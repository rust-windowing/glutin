#![cfg(target_os = "macos")]

pub use winit::os::macos::ActivationPolicy;
pub use winit::os::macos::MonitorIdExt;
pub use winit::os::macos::WindowBuilderExt;
pub use winit::os::macos::WindowExt;

use os::ContextTraitExt;
use Context;

use std::os::raw::c_void;

impl ContextTraitExt for Context {
    type Handle = *mut c_void;

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const c_void> {
        None
    }
}
