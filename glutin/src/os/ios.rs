#![cfg(target_os = "ios")]

use crate::os::ContextTraitExt;
use crate::{Context, ContextCurrentState};

pub use winit::os::ios::{MonitorIdExt, WindowBuilderExt, WindowExt};

use std::os::raw;

impl<T: ContextCurrentState> ContextTraitExt for Context<T> {
    type Handle = *mut raw::c_void;
    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        self.context.raw_handle()
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        None
    }
}
