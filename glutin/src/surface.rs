use super::*;

use std::convert::AsRef;
use std::marker::PhantomData;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

#[allow(non_snake_case)]
pub mod SurfaceInUse {
    use std::fmt::Debug;
    use std::marker::PhantomData;
    pub trait SurfaceInUseTrait: Debug + Clone + Copy {}

    // This is nightly only:
    // impl !Send for Context<PossiblyCurrent> {}
    // impl !Sync for Context<PossiblyCurrent> {}
    //
    // Instead we add a phantom type
    #[derive(Debug, Clone, Copy)]
    pub struct Possibly {
        phantom: PhantomData<*mut ()>,
    }
    #[derive(Debug, Clone, Copy)]
    pub enum No {}

    impl SurfaceInUseTrait for Possibly {}
    impl SurfaceInUseTrait for No {}
}
pub use SurfaceInUse::SurfaceInUseTrait;

pub trait Surface {
    type Inner;
    type NotInUseType: Surface;
    type PossiblyInUseType: Surface;

    fn inner(&self) -> &Self::Inner;
    fn inner_mut(&mut self) -> &mut Self::Inner;
    /// Returns the pixel format of the main framebuffer of the context.
    fn get_pixel_format(&self) -> PixelFormat;

    fn is_current(&self) -> bool;

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType;

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType;

    unsafe fn make_not_current(self) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>;
}

pub type WindowSurface<IU> = WindowSurfaceWrapper<Window, IU>;
pub type RawWindowSurface<IU> = WindowSurfaceWrapper<(), IU>;

#[derive(Debug)]
pub struct WindowSurfaceWrapper<W, IU: SurfaceInUseTrait> {
    pub(crate) surface: platform_impl::WindowSurface,
    pub(crate) window: W,
    phantom: PhantomData<IU>,
}

impl<W, IU: SurfaceInUseTrait> Surface for WindowSurfaceWrapper<W, IU> {
    type Inner = platform_impl::WindowSurface;
    type NotInUseType = WindowSurfaceWrapper<W, SurfaceInUse::No>;
    type PossiblyInUseType = WindowSurfaceWrapper<W, SurfaceInUse::Possibly>;

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

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        WindowSurfaceWrapper {
            surface: self.surface,
            window: self.window,
            phantom: PhantomData,
        }
    }

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        WindowSurfaceWrapper {
            surface: self.surface,
            window: self.window,
            phantom: PhantomData,
        }
    }

    unsafe fn make_not_current(self) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)> {
        match self.surface.make_not_current() {
            Ok(()) => Ok(WindowSurfaceWrapper {
                surface: self.surface,
                window: self.window,
                phantom: PhantomData,
            }),
            Err(err) => Err((Surface::treat_as_current(self), err)),
        }
    }
}

impl<IU: SurfaceInUseTrait> WindowSurface<IU> {
    pub fn new<
        'a,
        TE,
        IC: ContextIsCurrentTrait + 'a,
        PBT: SupportsPBuffersTrait + 'a,
        ST: SupportsSurfacelessTrait + 'a,
        CTX: Into<&'a SplitContext<IC, PBT, SupportsWindowSurfaces::Yes, ST>>,
    >(
        el: &EventLoopWindowTarget<TE>,
        ctx: CTX,
        wb: WindowBuilder,
    ) -> Result<WindowSurface<SurfaceInUse::No>, CreationError> {
        platform_impl::WindowSurface::new(el, ctx.into().inner(), wb).map(
            |(surface, window)| WindowSurface {
                surface,
                window,
                phantom: PhantomData,
            },
        )
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    pub unsafe fn split(self) -> (RawWindowSurface<IU>, Window) {
        (
            RawWindowSurface {
                surface: self.surface,
                window: (),
                phantom: PhantomData,
            },
            self.window,
        )
    }

    /// Update the surface after the underlying surface resizes.
    ///
    /// Wayland requires updating the surface when the underlying surface
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

    /// Swaps the buffers in case of double or triple buffering.
    ///
    /// You should call this function every time you have finished rendering, or
    /// the image may not be displayed on the screen.
    ///
    /// **Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.surface.swap_buffers()
    }
}

#[derive(Debug)]
pub struct PBuffer<IU: SurfaceInUseTrait> {
    pub(crate) pbuffer: platform_impl::PBuffer,
    phantom: PhantomData<IU>,
}

impl<IU: SurfaceInUseTrait> Surface for PBuffer<IU> {
    type Inner = platform_impl::PBuffer;
    type NotInUseType = PBuffer<SurfaceInUse::No>;
    type PossiblyInUseType = PBuffer<SurfaceInUse::Possibly>;

    fn inner(&self) -> &Self::Inner {
        &self.pbuffer
    }
    fn inner_mut(&mut self) -> &mut Self::Inner {
        &mut self.pbuffer
    }

    fn get_pixel_format(&self) -> PixelFormat {
        self.pbuffer.get_pixel_format()
    }

    fn is_current(&self) -> bool {
        self.pbuffer.is_current()
    }

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType {
        PBuffer {
            pbuffer: self.pbuffer,
            phantom: PhantomData,
        }
    }

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType {
        PBuffer {
            pbuffer: self.pbuffer,
            phantom: PhantomData,
        }
    }

    unsafe fn make_not_current(self) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)> {
        match self.pbuffer.make_not_current() {
            Ok(()) => Ok(PBuffer {
                pbuffer: self.pbuffer,
                phantom: PhantomData,
            }),
            Err(err) => Err((Surface::treat_as_current(self), err)),
        }
    }
}

impl<IU: SurfaceInUseTrait> PBuffer<IU> {
    pub fn new<
        'a,
        TE,
        IC: ContextIsCurrentTrait + 'a,
        WST: SupportsWindowSurfacesTrait + 'a,
        ST: SupportsSurfacelessTrait + 'a,
        CTX: Into<&'a SplitContext<IC, SupportsPBuffers::Yes, WST, ST>>,
    >(
        el: &EventLoopWindowTarget<TE>,
        ctx: CTX,
        size: dpi::PhysicalSize,
    ) -> Result<PBuffer<SurfaceInUse::No>, CreationError> {
        platform_impl::PBuffer::new(el, ctx.into().inner(), size).map(
            |pbuffer| PBuffer {
                pbuffer,
                phantom: PhantomData,
            },
        )
    }
}

impl Drop for platform_impl::PBuffer {
    fn drop(&mut self) {
        if self.is_current() {
            warn!("User dropped PBuffer that is still current. Future operations that modify and/or depend on the pbuffer will cause UB.");
        }
    }
}

impl Drop for platform_impl::WindowSurface {
    fn drop(&mut self) {
        if self.is_current() {
            warn!("User dropped WindowSurfaceWrapper that is still current. Future operations that modify and/or depend on the surface will cause UB.");
        }
    }
}
