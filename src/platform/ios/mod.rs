#![cfg(target_os = "ios")]

pub use api::ios::*;

pub use cocoa::base::id;

use GlAttributes;
use CreationError;
use PixelFormat;
use PixelFormatRequirements;
use ContextError;
use os::GlContextExt;

impl GlContextExt for Context {
    type Handle = id;

    #[inline]
    unsafe fn as_mut_ptr(&self) -> Self::Handle {
        *self.eagl_context.deref()
    }
}

#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

pub struct HeadlessContext(i32);

impl HeadlessContext {
    /// See the docs in the crate root file.
    pub fn new(_: (u32, u32), _: &PixelFormatRequirements, _: &GlAttributes<&HeadlessContext>,
               _: &PlatformSpecificHeadlessBuilderAttributes)
               -> Result<HeadlessContext, CreationError>
    {
        unimplemented!()
    }

    /// See the docs in the crate root file.
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        unimplemented!()
    }

    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        unimplemented!()
    }

    /// See the docs in the crate root file.
    pub fn is_current(&self) -> bool {
        unimplemented!()
    }

    /// See the docs in the crate root file.
    pub fn get_proc_address(&self, _addr: &str) -> *const () {
        unimplemented!()
    }

    pub fn get_api(&self) -> ::Api {
        ::Api::OpenGlEs
    }

    pub fn get_pixel_format(&self) -> PixelFormat {
        unimplemented!()
    }
}

unsafe impl Send for HeadlessContext {}
unsafe impl Sync for HeadlessContext {}

impl GlContextExt for HeadlessContext {
    type Handle = i32;

    unsafe fn as_mut_ptr(&self) -> Self::Handle {
        unimplemented!()
    }
}
