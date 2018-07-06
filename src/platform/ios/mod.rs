#![cfg(target_os = "ios")]

pub use api::ios::*;

use GlAttributes;
use CreationError;
use PixelFormat;
use PixelFormatRequirements;
use ContextError;

use std::os::raw::c_void;

#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

pub struct HeadlessContext(i32);

#[allow(dead_code)]
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

    pub unsafe fn raw_handle(&self) -> *mut c_void {
        unimplemented!()
    }
}

unsafe impl Send for HeadlessContext {}
unsafe impl Sync for HeadlessContext {}
