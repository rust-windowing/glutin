use crate::config::Config;
use crate::display::Display;
use crate::platform_impl;

use glutin_winit_interface::{NativePixmapSource, NativeWindowSource};
use winit_types::dpi;
use winit_types::error::Error;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum SurfaceType {
    Window,
    PBuffer,
    Pixmap,
}

pub trait SurfaceTypeTrait {
    fn surface_type() -> SurfaceType;
}

#[derive(Copy, Clone, Debug)]
pub enum Window {}
#[derive(Copy, Clone, Debug)]
pub enum PBuffer {}
#[derive(Copy, Clone, Debug)]
pub enum Pixmap {}

impl SurfaceTypeTrait for Window {
    fn surface_type() -> SurfaceType { SurfaceType::Window }
}
impl SurfaceTypeTrait for PBuffer {
    fn surface_type() -> SurfaceType { SurfaceType::PBuffer }
}
impl SurfaceTypeTrait for Pixmap {
    fn surface_type() -> SurfaceType { SurfaceType::Pixmap }
}

#[derive(Debug)]
pub struct Surface<T: SurfaceTypeTrait>(pub(crate) platform_impl::Surface<T>);

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_config(&self) -> Config {
        self.0.get_config()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.0.make_not_current()
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new<NPS: NativePixmapSource>(
        disp: &Display,
        conf: &Config,
        nps: NPS,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        platform_impl::Surface::<Pixmap>::new(&disp.0, conf.as_ref(), nps)
            .map(|(pix, surf)| (pix, Surface(surf)))
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        disp: &Display,
        conf: &Config,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        platform_impl::Surface::<PBuffer>::new(&disp.0, conf.as_ref(), size)
            .map(Surface)
    }
}

impl Surface<Window> {
    #[inline]
    pub unsafe fn new<NWS: NativeWindowSource>(
        disp: &Display,
        conf: &Config,
        nws: NWS,
    ) -> Result<(NWS::Window, Self), Error> {
        platform_impl::Surface::<Window>::new(&disp.0, conf.as_ref(), nws)
            .map(|(win, surf)| (win, Surface(surf)))
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.0.swap_buffers()
    }

    /// Swaps the buffers in case of double or triple buffering using specified
    /// damage rects.
    ///
    /// You should call this function every time you have finished rendering, or
    /// the image may not be displayed on the screen.
    ///
    /// **Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    pub fn swap_buffers_with_damage(&self, rects: &[Rect]) -> Result<(), Error> {
        self.0.swap_buffers_with_damage(rects)
    }

    // FIXME: As discussed with Osspial, kill me please. Should talk with vberger about best
    // method.
    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        #![cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        self.0.update_after_resize(size);
    }
}

// Rectangles to submit as buffer damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
