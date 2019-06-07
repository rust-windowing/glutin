use super::*;

use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub trait Surface {
    type Inner;

    fn inner(&self) -> &Self::Inner;
    fn inner_mut(&mut self) -> &mut Self::Inner;
    /// Returns the pixel format of the main framebuffer of the context.
    fn get_pixel_format(&self) -> PixelFormat;
}

pub trait IsPBuffer {}
pub trait IsWindowSurface {}

pub type WindowSurface = WindowSurfaceWrapper<Window>;
pub type RawWindowSurface = WindowSurfaceWrapper<()>;

pub struct WindowSurfaceWrapper<W> {
    pub(crate) surface: platform_impl::WindowSurface,
    pub(crate) window: W,
}

impl<W> Surface for WindowSurfaceWrapper<W> {
    type Inner = platform_impl::WindowSurface;

    fn inner(&self) -> &Self::Inner { &self.surface }
    fn inner_mut(&mut self) -> &mut Self::Inner { &mut self.surface }

    fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }
}

impl WindowSurface {
    pub fn new<TE, CS: ContextCurrentState, PBT: SupportsPBuffersTrait, ST: SupportsSurfacelessTrait>(el: &EventLoop<TE>, ctx: &Context<CS, PBT, SupportsWindowSurfaces::Yes, ST>, wb: WindowBuilder) -> Result<Self, CreationError> {
        let ctx = ctx.inner();
        platform_impl::WindowSurface::new(el, ctx, wb)
            .map(|(surface, window)| WindowSurface {
                surface,
                window,
            })
    }

    pub fn window(&self) -> &Window { &self.window }
    pub fn window_mut(&mut self) -> &mut Window { &mut self.window }
    pub unsafe fn split(self) -> (RawWindowSurface, Window) { (WindowSurfaceWrapper {surface: self.surface, window: ()}, self.window) }
}

impl<W> IsWindowSurface for WindowSurfaceWrapper<W> {}

pub struct PBuffer {
    pub(crate) surface: platform_impl::PBuffer,
}

impl Surface for PBuffer {
    type Inner = platform_impl::PBuffer;

    fn inner(&self) -> &Self::Inner { &self.surface }
    fn inner_mut(&mut self) -> &mut Self::Inner { &mut self.surface }

    fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }
}

impl PBuffer {
    pub fn new<TE, CS: ContextCurrentState, WST: SupportsWindowSurfacesTrait, ST: SupportsSurfacelessTrait>(el: &EventLoop<TE>, ctx: &Context<CS, SupportsPBuffers::Yes, WST, ST>, size: dpi::PhysicalSize) -> Result<Self, CreationError> {
        let ctx = ctx.inner();
        platform_impl::PBuffer::new(el, ctx, size)
            .map(|surface| PBuffer {
                surface,
            })
    }
}

impl IsPBuffer for PBuffer {}
