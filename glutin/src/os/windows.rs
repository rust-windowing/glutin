#![cfg(target_os = "windows")]

use crate::os::ContextTraitExt;
pub use crate::platform::{RawContextExt, RawHandle};
use crate::{Context, ContextCurrentState};
pub use glutin_egl_sys::EGLContext;

pub use winapi::shared::windef::HGLRC;
pub use winit::os::windows::{
    DeviceIdExt, MonitorIdExt, WindowBuilderExt, WindowExt,
};

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
