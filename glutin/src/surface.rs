use super::*;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

#[derive(Debug)]
pub struct PBuffer {
    pub(crate) pbuffer: platform_impl::PBuffer,
}

impl PBuffer {
    #[inline]
    pub unsafe fn new<TE>(
        el: &EventLoopWindowTarget<TE>,
        ctx: &Context,
        size: dpi::PhysicalSize,
    ) -> Result<PBuffer, CreationError> {
        platform_impl::PBuffer::new(el, ctx.inner(), size)
            .map(|pbuffer| PBuffer { pbuffer })
    }

    #[inline]
    pub(crate) fn inner(&self) -> &platform_impl::PBuffer {
        &self.pbuffer
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.pbuffer.is_current()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pbuffer.get_pixel_format()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        self.pbuffer.make_not_current()
    }
}

#[derive(Debug)]
pub struct WindowSurface {
    pub(crate) surface: platform_impl::WindowSurface,
}

impl WindowSurface {
    #[inline]
    pub unsafe fn new<TE>(
        el: &EventLoopWindowTarget<TE>,
        ctx: &Context,
        wb: WindowBuilder,
    ) -> Result<(Window, WindowSurface), CreationError> {
        platform_impl::WindowSurface::new(el, ctx.inner(), wb)
            .map(|(window, surface)| (window, WindowSurface { surface }))
    }

    #[inline]
    pub(crate) fn inner(&self) -> &platform_impl::WindowSurface {
        &self.surface
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.surface.is_current()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        self.surface.make_not_current()
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.surface.swap_buffers()
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        #![cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        self.surface.update_after_resize(size);
    }
}
