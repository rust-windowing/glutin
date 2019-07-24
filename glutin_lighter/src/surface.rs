use super::*;

use glutin::event_loop::EventLoopWindowTarget;
use glutin::window::{Window, WindowBuilder};
use glutin::{PBuffer, WindowSurface};
use std::marker::PhantomData;
use takeable_option::Takeable;

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

pub trait LighterSurface {
    type Inner;
    type NotInUseType: LighterSurface;
    type PossiblyInUseType: LighterSurface;

    fn inner(&self) -> &Self::Inner;
    /// Returns the pixel format of the main framebuffer of the context.
    fn get_pixel_format(&self) -> PixelFormat;

    fn is_current(&self) -> bool;

    unsafe fn treat_as_not_current(self) -> Self::NotInUseType;

    unsafe fn treat_as_current(self) -> Self::PossiblyInUseType;

    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>;
}

pub type LighterWindowSurface<IU> = LighterWindowSurfaceWrapper<Window, IU>;
pub type RawLighterWindowSurface<IU> = LighterWindowSurfaceWrapper<(), IU>;

#[derive(Debug)]
pub struct LighterWindowSurfaceWrapper<W, IU: SurfaceInUseTrait> {
    pub(crate) surface: Takeable<WindowSurface>,
    pub(crate) window: Takeable<W>,
    phantom: PhantomData<IU>,
}

impl<W, IU: SurfaceInUseTrait> LighterSurface
    for LighterWindowSurfaceWrapper<W, IU>
{
    type Inner = WindowSurface;
    type NotInUseType = LighterWindowSurfaceWrapper<W, SurfaceInUse::No>;
    type PossiblyInUseType =
        LighterWindowSurfaceWrapper<W, SurfaceInUse::Possibly>;

    #[inline]
    fn inner(&self) -> &Self::Inner {
        &self.surface
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }

    #[inline]
    fn is_current(&self) -> bool {
        self.surface.is_current()
    }

    #[inline]
    unsafe fn treat_as_not_current(mut self) -> Self::NotInUseType {
        LighterWindowSurfaceWrapper {
            surface: Takeable::new_take(&mut self.surface),
            window: Takeable::new_take(&mut self.window),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn treat_as_current(mut self) -> Self::PossiblyInUseType {
        LighterWindowSurfaceWrapper {
            surface: Takeable::new_take(&mut self.surface),
            window: Takeable::new_take(&mut self.window),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn make_not_current(
        mut self,
    ) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>
    {
        match self.surface.make_not_current() {
            Ok(()) => Ok(LighterWindowSurfaceWrapper {
                surface: Takeable::new_take(&mut self.surface),
                window: Takeable::new_take(&mut self.window),
                phantom: PhantomData,
            }),
            Err(err) => Err((LighterSurface::treat_as_current(self), err)),
        }
    }
}

impl<IU: SurfaceInUseTrait> LighterWindowSurface<IU> {
    #[inline]
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
    ) -> Result<LighterWindowSurface<SurfaceInUse::No>, CreationError> {
        WindowSurface::new(el, ctx.into().inner(), wb).map(
            |(window, surface)| LighterWindowSurface {
                surface: Takeable::new(surface),
                window: Takeable::new(window),
                phantom: PhantomData,
            },
        )
    }

    #[inline]
    pub fn window(&self) -> &Window {
        &self.window
    }
    #[inline]
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    #[inline]
    pub unsafe fn split(mut self) -> (RawLighterWindowSurface<IU>, Window) {
        (
            RawLighterWindowSurface {
                surface: Takeable::new_take(&mut self.surface),
                window: Takeable::new(()),
                phantom: PhantomData,
            },
            Takeable::take(&mut self.window),
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
    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
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
    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.surface.swap_buffers()
    }
}

#[derive(Debug)]
pub struct LighterPBuffer<IU: SurfaceInUseTrait> {
    pub(crate) pbuffer: Takeable<PBuffer>,
    phantom: PhantomData<IU>,
}

impl<IU: SurfaceInUseTrait> LighterSurface for LighterPBuffer<IU> {
    type Inner = PBuffer;
    type NotInUseType = LighterPBuffer<SurfaceInUse::No>;
    type PossiblyInUseType = LighterPBuffer<SurfaceInUse::Possibly>;

    #[inline]
    fn inner(&self) -> &Self::Inner {
        &self.pbuffer
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.pbuffer.get_pixel_format()
    }

    #[inline]
    fn is_current(&self) -> bool {
        self.pbuffer.is_current()
    }

    #[inline]
    unsafe fn treat_as_not_current(mut self) -> Self::NotInUseType {
        LighterPBuffer {
            pbuffer: Takeable::new_take(&mut self.pbuffer),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn treat_as_current(mut self) -> Self::PossiblyInUseType {
        LighterPBuffer {
            pbuffer: Takeable::new_take(&mut self.pbuffer),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn make_not_current(
        mut self,
    ) -> Result<Self::NotInUseType, (Self::PossiblyInUseType, ContextError)>
    {
        match self.pbuffer.make_not_current() {
            Ok(()) => Ok(LighterPBuffer {
                pbuffer: Takeable::new_take(&mut self.pbuffer),
                phantom: PhantomData,
            }),
            Err(err) => Err((LighterSurface::treat_as_current(self), err)),
        }
    }
}

impl<IU: SurfaceInUseTrait> LighterPBuffer<IU> {
    #[inline]
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
    ) -> Result<LighterPBuffer<SurfaceInUse::No>, CreationError> {
        PBuffer::new(el, ctx.into().inner(), size).map(|pbuffer| {
            LighterPBuffer {
                pbuffer: Takeable::new(pbuffer),
                phantom: PhantomData,
            }
        })
    }
}

impl<IU: SurfaceInUseTrait> Drop for LighterPBuffer<IU> {
    fn drop(&mut self) {
        Takeable::try_take(&mut self.pbuffer)
            .map(|pbuffer|
        if pbuffer.is_current() {
            warn!("User dropped PBuffer that is still current. Future operations that modify and/or depend on the pbuffer will cause UB.");
        });
    }
}

impl<W, IU: SurfaceInUseTrait> Drop for LighterWindowSurfaceWrapper<W, IU> {
    fn drop(&mut self) {
        Takeable::try_take(&mut self.surface)
            .map(|surface|
        if surface.is_current() {
            warn!("User dropped LighterWindowSurfaceWrapper that is still current. Future operations that modify and/or depend on the surface will cause UB.");
        });
    }
}
