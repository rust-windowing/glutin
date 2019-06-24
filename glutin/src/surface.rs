use super::*;

use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub trait Surface {
    type Inner;

    fn inner(&self) -> &Self::Inner;
    fn inner_mut(&mut self) -> &mut Self::Inner;
    /// Returns the pixel format of the main framebuffer of the context.
    fn get_pixel_format(&self) -> PixelFormat;

    fn is_current(&self) -> bool;
}

pub trait IsPBuffer: Surface {}
pub trait IsWindowSurface: Surface {}

pub type WindowSurface = WindowSurfaceWrapper<Window>;
pub type RawWindowSurface = WindowSurfaceWrapper<()>;

pub struct WindowSurfaceWrapper<W> {
    pub(crate) surface: platform_impl::WindowSurface,
    pub(crate) window: W,
}

impl<W> Surface for WindowSurfaceWrapper<W> {
    type Inner = platform_impl::WindowSurface;

    fn inner(&self) -> &Self::Inner {
        &self.surface
    }
    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.surface
    }

    fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }

    fn is_current(&self) -> bool {
        self.surface.is_current()
    }
}

impl WindowSurface {
    pub fn new<
        TE,
        CS: ContextCurrentState,
        PBT: SupportsPBuffersTrait,
        ST: SupportsSurfacelessTrait,
    >(
        el: &EventLoop<TE>,
        ctx: &Context<CS, PBT, SupportsWindowSurfaces::Yes, ST>,
        wb: WindowBuilder,
    ) -> Result<Self, CreationError> {
        let ctx = ctx.inner();
        platform_impl::WindowSurface::new(el, ctx, wb)
            .map(|(surface, window)| WindowSurface { surface, window })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }
    pub unsafe fn split(self) -> (RawWindowSurface, Window) {
        (
            WindowSurfaceWrapper {
                surface: self.surface,
                window: (),
            },
            self.window,
        )
    }

    /// Update the context after the underlying surface resizes.
    ///
    /// Wayland requires updating the context when the underlying surface
    /// resizes.
    ///
    /// The easiest way of doing this is to take every [`Resized`] window event
    /// that is received with a [`LogicalSize`] and convert it to a
    /// [`PhysicalSize`] and pass it into this function.
    ///
    /// Note: You still have to call the [`Context`]'s
    /// [`update_after_resize`] function for MacOS.
    ///
    /// [`LogicalSize`]: dpi/struct.LogicalSize.html
    /// [`PhysicalSize`]: dpi/struct.PhysicalSize.html
    /// [`Resized`]: event/enum.WindowEvent.html#variant.Resized
    /// FIXME: links
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

impl<W> IsWindowSurface for WindowSurfaceWrapper<W> {}

pub struct PBuffer {
    pub(crate) surface: platform_impl::PBuffer,
}

impl Surface for PBuffer {
    type Inner = platform_impl::PBuffer;

    fn inner(&self) -> &Self::Inner {
        &self.surface
    }
    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.surface
    }

    fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }

    fn is_current(&self) -> bool {
        self.surface.is_current()
    }
}

impl PBuffer {
    pub fn new<
        TE,
        CS: ContextCurrentState,
        WST: SupportsWindowSurfacesTrait,
        ST: SupportsSurfacelessTrait,
    >(
        el: &EventLoop<TE>,
        ctx: &Context<CS, SupportsPBuffers::Yes, WST, ST>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        let ctx = ctx.inner();
        platform_impl::PBuffer::new(el, ctx, size)
            .map(|surface| PBuffer { surface })
    }
}

impl IsPBuffer for PBuffer {}

impl Drop for PBuffer {
    fn drop(&mut self) {
        if self.is_current() {
            warn!("User dropped PBuffer that is still current. Future operations that modify and/or depend on the pbuffer will cause UB.");
        }
    }
}

impl<T> Drop for WindowSurfaceWrapper<T> {
    fn drop(&mut self) {
        if self.is_current() {
            warn!("User dropped WindowSurfaceWrapper that is still current. Future operations that modify and/or depend on the surface will cause UB.");
        }
    }
}
