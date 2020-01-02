use crate::api::osmesa;
use crate::config::{Api, Version};
use crate::context::ContextBuilderWrapper;

use winit_types::dpi;
use winit_types::error::Error;

use std::os::raw;

#[derive(Debug)]
pub struct OsMesaContext {
    pub(crate) context: osmesa::OsMesaContext,
}

pub trait OsMesaContextExt {
    fn build_osmesa(self, version: (Api, Version)) -> Result<OsMesaContext, Error>
    where
        Self: Sized;
}

impl<'a> OsMesaContextExt for OsMesaContextBuilder<'a> {
    #[inline]
    fn build_osmesa(self, version: (Api, Version)) -> Result<OsMesaContext, Error>
    where
        Self: Sized,
    {
        let cb = self.map_sharing(|ctx| &ctx.context);
        osmesa::OsMesaContext::new(cb, version).map(|context| OsMesaContext { context })
    }
}

pub type OsMesaContextBuilder<'a> = ContextBuilderWrapper<&'a OsMesaContext>;

#[derive(Debug)]
pub struct OsMesaBuffer {
    pub(crate) buffer: osmesa::OsMesaBuffer,
}

impl OsMesaContext {
    /// Returns the address of an OpenGL function.
    pub fn get_proc_address(&self, addr: &str) -> *const raw::c_void {
        self.context.get_proc_address(addr)
    }

    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }

    pub unsafe fn make_current_osmesa_buffer(&self, buffer: &OsMesaBuffer) -> Result<(), Error> {
        self.context.make_current_osmesa_buffer(buffer.inner())
    }

    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.context.make_not_current()
    }

    pub(crate) fn inner(&self) -> &osmesa::OsMesaContext {
        &self.context
    }
}

impl OsMesaBuffer {
    pub(crate) fn inner(&self) -> &osmesa::OsMesaBuffer {
        &self.buffer
    }

    pub fn new(ctx: &OsMesaContext, size: dpi::PhysicalSize) -> Result<OsMesaBuffer, Error> {
        let ctx = ctx.inner();
        osmesa::OsMesaBuffer::new(ctx, size).map(|buffer| OsMesaBuffer { buffer })
    }
}
