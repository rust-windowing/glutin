//! Wrapper around `NSView`.

use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use icrate::AppKit::{NSView, NSWindow};
use icrate::Foundation::{MainThreadBound, MainThreadMarker};
use objc2::rc::Id;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};

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

impl<D: HasDisplayHandle> Display<D> {
    pub(crate) unsafe fn create_pixmap_surface(
        &self,
        _config: &Config<D>,
        _surface_attributes: SurfaceAttributes<PixmapSurface>,
    ) -> Result<Surface<D, PixmapSurface>> {
        Err(ErrorKind::NotSupported("pixmaps are not supported with CGL").into())
    }

    pub(crate) unsafe fn create_pbuffer_surface(
        &self,
        _config: &Config<D>,
        _surface_attributes: SurfaceAttributes<PbufferSurface>,
    ) -> Result<Surface<D, PbufferSurface>> {
        Err(ErrorKind::NotSupported("pbuffers are not supported with CGL").into())
    }

    pub(crate) fn create_window_surface<W: HasWindowHandle>(
        &self,
        config: &Config<D>,
        surface_attributes: SurfaceAttributes<WindowSurface<W>>,
    ) -> Result<Surface<D, WindowSurface<W>>> {
        let native_window = match surface_attributes.ty.0.window_handle()?.as_raw() {
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
        let ns_view: Id<NSView> = if let Some(ns_view) =
            unsafe { Id::retain(native_window.ns_view.as_ptr().cast()) }
        {
            ns_view
        } else {
            return Err(ErrorKind::NotSupported("ns_view of provided native window is nil").into());
        };
        let ns_window = match unsafe { ns_view.window() } {
            Some(window) => window,
            None => {
                return Err(
                    ErrorKind::NotSupported("ns_window of provided native window is nil").into()
                )
            },
        };

        let ns_view = MainThreadBound::new(ns_view, mtm);
        let ns_window = MainThreadBound::new(ns_window, mtm);

        let surface = Surface {
            display: self.clone(),
            config: config.clone(),
            ns_view,
            ns_window,
            _nosendsync: PhantomData,
            ty: surface_attributes.ty,
        };
        Ok(surface)
    }
}

/// A wrapper aroud `NSView`.
pub struct Surface<D, T: SurfaceTypeTrait> {
    display: Display<D>,
    config: Config<D>,
    pub(crate) ns_view: MainThreadBound<Id<NSView>>,
    ns_window: MainThreadBound<Id<NSWindow>>,
    _nosendsync: PhantomData<*const std::ffi::c_void>,
    ty: T,
}

impl<D, W: HasWindowHandle> Surface<D, WindowSurface<W>> {
    /// Get a reference to the underlying window.
    pub fn window(&self) -> &W {
        &self.ty.0
    }
}

impl<D, W: HasWindowHandle> AsRef<W> for Surface<D, WindowSurface<W>> {
    fn as_ref(&self) -> &W {
        self.window()
    }
}

impl<D, W: HasWindowHandle> HasWindowHandle for Surface<D, WindowSurface<W>> {
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        self.window().window_handle()
    }
}

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> GlSurface<T> for Surface<D, T> {
    type Context = PossiblyCurrentContext<D>;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        0
    }

    fn width(&self) -> Option<u32> {
        let window = &self.ns_window;
        let view = &self.ns_view;
        MainThreadMarker::run_on_main(|mtm| unsafe {
            let scale_factor = window.get(mtm).backingScaleFactor();
            let frame = view.get(mtm).frame();
            Some((frame.size.width * scale_factor) as u32)
        })
    }

    fn height(&self) -> Option<u32> {
        let window = &self.ns_window;
        let view = &self.ns_view;
        MainThreadMarker::run_on_main(|mtm| unsafe {
            let scale_factor = window.get(mtm).backingScaleFactor();
            let frame = view.get(mtm).frame();
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

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> GetGlConfig for Surface<D, T> {
    type Target = Config<D>;

    fn config(&self) -> Self::Target {
        self.config.clone()
    }
}

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> GetGlDisplay for Surface<D, T> {
    type Target = Display<D>;

    fn display(&self) -> Self::Target {
        self.display.clone()
    }
}

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> AsRawSurface for Surface<D, T> {
    fn raw_surface(&self) -> RawSurface {
        // SAFETY: We only use the thread marker to get the pointer value of the view
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        RawSurface::Cgl(Id::as_ptr(self.ns_view.get(mtm)).cast())
    }
}

impl<D, T: SurfaceTypeTrait> fmt::Debug for Surface<D, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("config", &self.config.inner.raw)
            .field("ns_view", &self.ns_view)
            .field("type", &T::surface_type())
            .finish()
    }
}

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> Sealed for Surface<D, T> {}
