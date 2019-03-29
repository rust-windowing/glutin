#![cfg(not(any(
    target_os = "ios",
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "emscripten",
)))]

use crate::{
    Api, ContextError, CreationError, GlAttributes, PixelFormat,
    PixelFormatRequirements,
};

use winit::dpi;

#[derive(Debug)]
pub enum Context {}

impl Context {
    #[inline]
    pub fn new_windowed(
        _: winit::WindowBuilder,
        _: &winit::EventsLoop,
        _: &PixelFormatRequirements,
        _: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError> {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub fn new_headless(
        _: &winit::EventsLoop,
        _: &PixelFormatRequirements,
        _: &GlAttributes<&Context>,
        _: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub fn resize(&self, _: u32, _: u32) {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub fn get_proc_address(&self, _: &str) -> *const () {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }
}
