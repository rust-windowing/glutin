#![cfg(any(target_os = "android"))]

use crate::platform::ContextTraitExt;
use crate::{Context, ContextCurrentState};
use crate::{
    SupportsPBuffersTrait, SupportsSurfacelessTrait,
    SupportsWindowSurfacesTrait,
};

pub use glutin_egl_sys::EGLContext;
pub use winit::platform::android::*;

use std::os::raw;

impl<
        IC: ContextCurrentState,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
    > ContextTraitExt for Context<IC, PBT, WST, ST>
{
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
