//! Wrapper around `NSView`.

use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use objc2::rc::Id;
use objc2_app_kit::NSView;
use objc2_foundation::{run_on_main, MainThreadBound, MainThreadMarker};
use raw_window_handle::RawWindowHandle;

use crate::config::GetGlConfig;
use crate::display::GetGlDisplay;
use crate::error::{ErrorKind, Result};
use crate::private::Sealed;
use crate::surface::{
    AsRawSurface, GlSurface, PbufferSurface, PixmapSurface, RawSurface, SurfaceAttributes,
    SurfaceTypeTrait, SwapInterval, WindowSurface,
};

use super::config::Config;
use super::context::PossiblyCurrentContext;
use super::display::Display;

impl Display {
    pub(crate) unsafe fn create_pixmap_surface(
        &self,
        _config: &Config,
        _surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Surface<PixmapSurface>> {
        Err(ErrorKind::NotSupported("pixmaps are not supported with CGL").into())
    }

    pub(crate) unsafe fn create_pbuffer_surface(
        &self,
        _config: &Config,
        _surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Surface<PbufferSurface>> {
        Err(ErrorKind::NotSupported("pbuffers are not supported with CGL").into())
    }

    pub(crate) unsafe fn create_window_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Surface<WindowSurface>> {
        let native_window = match surface_attributes.raw_window_handle.unwrap() {
            RawWindowHandle::AppKit(window) => window,
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided native window is not supported").into()
                )
            },
        };

        // SAFETY: The objects below must have been created on the main thread
        // in the first place, so we can safely "move" them back to that thread.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        // SAFETY: Validity of the view and window is ensured by caller
        // This function makes sure the window is non null.
        let ns_view = if let Some(ns_view) =
            unsafe { Id::retain(native_window.ns_view.as_ptr().cast()) }
        {
            ns_view
        } else {
            return Err(ErrorKind::NotSupported("ns_view of provided native window is nil").into());
        };
        let ns_view = MainThreadBound::new(ns_view, mtm);

        let surface = Surface {
            display: self.clone(),
            config: config.clone(),
            ns_view,
            _nosync: PhantomData,
            _ty: PhantomData,
        };
        Ok(surface)
    }
}

/// A wrapper around `NSView`.
pub struct Surface<T: SurfaceTypeTrait> {
    display: Display,
    config: Config,
    pub(crate) ns_view: MainThreadBound<Id<NSView>>,
    _nosync: PhantomData<*const std::ffi::c_void>,
    _ty: PhantomData<T>,
}

// Impl only `Send` for Surface.
unsafe impl<T: SurfaceTypeTrait> Send for Surface<T> {}

impl<T: SurfaceTypeTrait> GlSurface<T> for Surface<T> {
    type Context = PossiblyCurrentContext;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        0
    }

    fn width(&self) -> Option<u32> {
        let view = &self.ns_view;
        run_on_main(|mtm| {
            let view = view.get(mtm);
            let scale_factor = match view.window() {
                Some(window) => window.backingScaleFactor(),
                None => 1.0,
            };
            let frame = view.frame();
            Some((frame.size.width * scale_factor) as u32)
        })
    }

    fn height(&self) -> Option<u32> {
        let view = &self.ns_view;
        run_on_main(|mtm| {
            let view = view.get(mtm);
            let scale_factor = match view.window() {
                Some(window) => window.backingScaleFactor(),
                None => 1.0,
            };
            let frame = view.frame();
            Some((frame.size.height * scale_factor) as u32)
        })
    }

    fn is_single_buffered(&self) -> bool {
        self.config.is_single_buffered()
    }

    fn swap_buffers(&self, context: &Self::Context) -> Result<()> {
        context.inner.flush_buffer()
    }

    fn set_swap_interval(&self, context: &Self::Context, interval: SwapInterval) -> Result<()> {
        context.inner.set_swap_interval(interval);
        Ok(())
    }

    fn is_current(&self, context: &Self::Context) -> bool {
        context.inner.is_view_current(&self.ns_view)
    }

    fn is_current_draw(&self, context: &Self::Context) -> bool {
        self.is_current(context)
    }

    fn is_current_read(&self, context: &Self::Context) -> bool {
        self.is_current(context)
    }

    fn resize(&self, context: &Self::Context, _width: NonZeroU32, _height: NonZeroU32) {
        context.inner.update();
    }
}

impl<T: SurfaceTypeTrait> GetGlConfig for Surface<T> {
    type Target = Config;

    fn config(&self) -> Self::Target {
        self.config.clone()
    }
}

impl<T: SurfaceTypeTrait> GetGlDisplay for Surface<T> {
    type Target = Display;

    fn display(&self) -> Self::Target {
        self.display.clone()
    }
}

impl<T: SurfaceTypeTrait> AsRawSurface for Surface<T> {
    fn raw_surface(&self) -> RawSurface {
        // SAFETY: We only use the thread marker to get the pointer value of the view
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        RawSurface::Cgl(Id::as_ptr(self.ns_view.get(mtm)).cast())
    }
}

impl<T: SurfaceTypeTrait> fmt::Debug for Surface<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("config", &self.config.inner.raw)
            .field("ns_view", &self.ns_view)
            .field("type", &T::surface_type())
            .finish()
    }
}

impl<T: SurfaceTypeTrait> Sealed for Surface<T> {}
