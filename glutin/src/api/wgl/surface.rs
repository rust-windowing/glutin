//! A wrapper around `HWND` used for GL operations.

use std::io::Error as IoError;
use std::num::NonZeroU32;
use std::{fmt, mem};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::{Gdi as gdi, OpenGL as gl};
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

use crate::config::GetGlConfig;
use crate::display::{DisplayFeatures, GetGlDisplay};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
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
        Err(ErrorKind::NotSupported("pixmaps are not implemented with WGL").into())
    }

    pub(crate) unsafe fn create_pbuffer_surface(
        &self,
        _config: &Config<D>,
        _surface_attributes: SurfaceAttributes<PbufferSurface>,
    ) -> Result<Surface<D, PbufferSurface>> {
        Err(ErrorKind::NotSupported("pbuffers are not implemented with WGL").into())
    }

    pub(crate) fn create_window_surface<W: HasWindowHandle>(
        &self,
        config: &Config<D>,
        surface_attributes: SurfaceAttributes<WindowSurface<W>>,
    ) -> Result<Surface<D, WindowSurface<W>>> {
        let hwnd = match surface_attributes.ty.0.window_handle()?.as_raw() {
            RawWindowHandle::Win32(window_handle) => {
                let _ = config.apply_on_native_window(&surface_attributes.ty.0);
                window_handle.hwnd.get() as HWND
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided native window is not supported").into()
                )
            },
        };

        let hdc = unsafe { gdi::GetDC(hwnd) };

        let surface = Surface {
            display: self.clone(),
            config: config.clone(),
            hwnd,
            hdc,
            ty: surface_attributes.ty,
        };

        Ok(surface)
    }
}

/// A Wrapper around `HWND`.
pub struct Surface<D, T: SurfaceTypeTrait> {
    display: Display<D>,
    config: Config<D>,
    pub(crate) hwnd: HWND,
    pub(crate) hdc: HDC,
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

impl<D, T: SurfaceTypeTrait> Drop for Surface<D, T> {
    fn drop(&mut self) {
        unsafe {
            gdi::ReleaseDC(self.hwnd, self.hdc);
        }
    }
}

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> GlSurface<T> for Surface<D, T> {
    type Context = PossiblyCurrentContext<D>;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        0
    }

    fn width(&self) -> Option<u32> {
        let mut rect: RECT = unsafe { mem::zeroed() };
        if unsafe { GetClientRect(self.hwnd, &mut rect) } == false.into() {
            None
        } else {
            Some((rect.right - rect.left) as u32)
        }
    }

    fn height(&self) -> Option<u32> {
        let mut rect: RECT = unsafe { mem::zeroed() };
        if unsafe { GetClientRect(self.hwnd, &mut rect) } == false.into() {
            None
        } else {
            Some((rect.bottom - rect.top) as u32)
        }
    }

    fn is_single_buffered(&self) -> bool {
        self.config.is_single_buffered()
    }

    fn swap_buffers(&self, _context: &Self::Context) -> Result<()> {
        unsafe {
            if gl::SwapBuffers(self.hdc) == 0 {
                Err(IoError::last_os_error().into())
            } else {
                Ok(())
            }
        }
    }

    fn set_swap_interval(&self, _context: &Self::Context, interval: SwapInterval) -> Result<()> {
        let interval = match interval {
            SwapInterval::DontWait => 0,
            SwapInterval::Wait(n) => n.get(),
        };

        let res = match self.display.inner.wgl_extra {
            Some(extra) if self.display.inner.features.contains(DisplayFeatures::SWAP_CONTROL) => unsafe {
                extra.SwapIntervalEXT(interval as _)
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("swap contol extrensions are not supported").into()
                )
            },
        };

        if res == 0 {
            Err(IoError::last_os_error().into())
        } else {
            Ok(())
        }
    }

    fn is_current(&self, context: &Self::Context) -> bool {
        context.is_current()
    }

    fn is_current_draw(&self, context: &Self::Context) -> bool {
        context.is_current()
    }

    fn is_current_read(&self, context: &Self::Context) -> bool {
        context.is_current()
    }

    fn resize(&self, _context: &Self::Context, _width: NonZeroU32, _height: NonZeroU32) {
        // This isn't supported with WGL.
    }
}

impl<D, T: SurfaceTypeTrait> fmt::Debug for Surface<D, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("config", &self.config.inner.pixel_format_index)
            .field("hwnd", &self.hwnd)
            .field("hdc", &self.hdc)
            .finish()
    }
}

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> AsRawSurface for Surface<D, T> {
    fn raw_surface(&self) -> RawSurface {
        RawSurface::Wgl(self.hwnd as _)
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

impl<D: HasDisplayHandle, T: SurfaceTypeTrait> Sealed for Surface<D, T> {}
