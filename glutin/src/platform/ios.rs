#![cfg(target_os = "ios")]

use crate::platform::ContextTraitExt;
use crate::{Context, ContextCurrentState};
use crate::{
    SupportsPBuffersTrait, SupportsSurfacelessTrait,
    SupportsWindowSurfacesTrait,
};

pub use winit::platform::ios::*;

use std::os::raw;

impl<
        CS: ContextCurrentState,
        PBS: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
    > ContextTraitExt for Context<CS, PBS, WST, ST>
{
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
