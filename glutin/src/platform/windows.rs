#![cfg(target_os = "windows")]

// mod rawext;

use crate::platform::ContextTraitExt;
pub use crate::platform_impl::{RawContextExt, RawHandle};
use crate::{Context, ContextCurrentState};
use crate::{
    SupportsPBuffersTrait, SupportsSurfacelessTrait,
    SupportsWindowSurfacesTrait,
};
pub use glutin_egl_sys::EGLContext;

pub use winapi::shared::windef::HGLRC;
pub use winit::platform::windows::*;
// pub use self::rawext::*;

impl<
        IC: ContextCurrentState,
        PBS: SupportsPBuffersTrait,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
    > ContextTraitExt for Context<IC, PBS, WST, ST>
{
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
