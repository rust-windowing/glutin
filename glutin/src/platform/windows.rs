#![cfg(target_os = "windows")]

//mod rawext;

use crate::platform::ContextTraitExt;
use crate::{SupportsPBuffersTrait, SupportsWindowSurfacesTrait, SupportsSurfacelessTrait};
pub use crate::platform_impl::{RawContextExt, RawHandle};
use crate::{Context, ContextCurrentState};
pub use glutin_egl_sys::EGLContext;

pub use winapi::shared::windef::HGLRC;
pub use winit::platform::windows::*;
//pub use self::rawext::*;

impl<CS: ContextCurrentState, PBS: SupportsPBuffersTrait, WST: SupportsWindowSurfacesTrait, ST: SupportsSurfacelessTrait> ContextTraitExt for Context<CS, PBS, WST, ST> {
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
