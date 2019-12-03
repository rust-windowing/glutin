#![cfg(target_os = "macos")]

use crate::platform::ContextTraitExt;
use crate::{Context, ContextCurrentState};
use crate::{SupportsPBuffersTrait, SupportsSurfacelessTrait, SupportsWindowSurfacesTrait};

use std::os::raw;

impl<
        IC: ContextCurrentState,
        PBT: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
    > ContextTraitExt for Context<IC, PBT, WST, ST>
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
