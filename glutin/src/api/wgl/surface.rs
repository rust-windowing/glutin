//! A wrapper around `HWND` used for GL operations.

use std::io::Error as IoError;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::os::raw::c_int;
use std::{fmt, mem};

use glutin_wgl_sys::wgl::types::GLenum;
use glutin_wgl_sys::wgl_extra::types::HPBUFFEREXT;
use glutin_wgl_sys::wgl_extra::{self};
use raw_window_handle::RawWindowHandle;
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

impl Display {
    pub(crate) unsafe fn create_pixmap_surface(
        &self,
        _config: &Config,
        _surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Surface<PixmapSurface>> {
        Err(ErrorKind::NotSupported("pixmaps are not implemented with WGL").into())
    }

    pub(crate) unsafe fn create_pbuffer_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Surface<PbufferSurface>> {
        let extra = self
            .inner
            .wgl_extra
            .filter(|_| self.inner.client_extensions.contains("WGL_ARB_pbuffer"))
            .ok_or(ErrorKind::NotSupported("pbuffer extensions are not supported"))?;

        let hdc = config.inner.hdc;
        let width = surface_attributes.width.unwrap().get() as c_int;
        let height = surface_attributes.height.unwrap().get() as c_int;
        let mut attrs = [0; 3];
        if surface_attributes.largest_pbuffer {
            attrs[0] = wgl_extra::PBUFFER_LARGEST_ARB as c_int;
            attrs[1] = 1;
        }

        let hbuf = unsafe {
            extra.CreatePbufferARB(
                hdc as _,
                config.inner.pixel_format_index,
                width,
                height,
                attrs.as_ptr(),
            )
        };
        if hbuf.is_null() {
            return Err(IoError::last_os_error().into());
        }

        let hdc = unsafe { extra.GetPbufferDCARB(hbuf) };
        if hdc.is_null() {
            return Err(IoError::last_os_error().into());
        }

        let surface = Surface {
            display: self.clone(),
            config: config.clone(),
            raw: WglSurface::PBuffer(hbuf, hdc as _),
            _ty: PhantomData,
        };

        Ok(surface)
    }

    pub(crate) unsafe fn create_window_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Surface<WindowSurface>> {
        let hwnd = match surface_attributes.raw_window_handle.as_ref().unwrap() {
            handle @ RawWindowHandle::Win32(window_handle) => {
                let _ = unsafe { config.apply_on_native_window(handle) };
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
            raw: WglSurface::Window(hwnd, hdc),
            _ty: PhantomData,
        };

        Ok(surface)
    }
}

/// A Wrapper around `WglSurface`.
pub struct Surface<T: SurfaceTypeTrait> {
    display: Display,
    config: Config,
    pub(crate) raw: WglSurface,
    _ty: PhantomData<T>,
}

// Impl only `Send` for Surface.
unsafe impl<T: SurfaceTypeTrait> Send for Surface<T> {}

impl<T: SurfaceTypeTrait> Surface<T> {
    fn raw_attribute(&self, attr: GLenum) -> Option<c_int> {
        match self.raw {
            WglSurface::Window(..) => None,
            WglSurface::PBuffer(hbuf, _) => {
                let extra = self.display.inner.wgl_extra.unwrap();
                let mut value = 0;
                if unsafe { extra.QueryPbufferARB(hbuf, attr as _, &mut value) } == false.into() {
                    None
                } else {
                    Some(value)
                }
            },
        }
    }
}

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    fn drop(&mut self) {
        unsafe {
            match self.raw {
                WglSurface::Window(hwnd, hdc) => {
                    gdi::ReleaseDC(hwnd, hdc);
                },
                WglSurface::PBuffer(hbuf, hdc) => {
                    let extra = self.display.inner.wgl_extra.unwrap();
                    extra.ReleasePbufferDCARB(hbuf, hdc as _);
                    extra.DestroyPbufferARB(hbuf);
                },
            }
        }
    }
}

impl<T: SurfaceTypeTrait> GlSurface<T> for Surface<T> {
    type Context = PossiblyCurrentContext;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        0
    }

    fn width(&self) -> Option<u32> {
        match self.raw {
            WglSurface::Window(hwnd, _) => {
                let mut rect: RECT = unsafe { mem::zeroed() };
                if unsafe { GetClientRect(hwnd, &mut rect) } == false.into() {
                    None
                } else {
                    Some((rect.right - rect.left) as u32)
                }
            },
            WglSurface::PBuffer(..) => {
                self.raw_attribute(wgl_extra::PBUFFER_WIDTH_ARB).map(|x| x as _)
            },
        }
    }

    fn height(&self) -> Option<u32> {
        match self.raw {
            WglSurface::Window(hwnd, _) => {
                let mut rect: RECT = unsafe { mem::zeroed() };
                if unsafe { GetClientRect(hwnd, &mut rect) } == false.into() {
                    None
                } else {
                    Some((rect.bottom - rect.top) as u32)
                }
            },
            WglSurface::PBuffer(..) => {
                self.raw_attribute(wgl_extra::PBUFFER_HEIGHT_ARB).map(|x| x as _)
            },
        }
    }

    fn is_single_buffered(&self) -> bool {
        self.config.is_single_buffered()
    }

    fn swap_buffers(&self, _context: &Self::Context) -> Result<()> {
        unsafe {
            if gl::SwapBuffers(self.raw.hdc()) == 0 {
                Err(IoError::last_os_error().into())
            } else {
                Ok(())
            }
        }
    }

    fn set_swap_interval(&self, _context: &Self::Context, interval: SwapInterval) -> Result<()> {
        match self.raw {
            WglSurface::Window(..) => {
                let extra = self
                    .display
                    .inner
                    .wgl_extra
                    .filter(|_| self.display.inner.features.contains(DisplayFeatures::SWAP_CONTROL))
                    .ok_or(ErrorKind::NotSupported("swap control extensions are not supported"))?;

                let interval = match interval {
                    SwapInterval::DontWait => 0,
                    SwapInterval::Wait(n) => n.get(),
                };

                if unsafe { extra.SwapIntervalEXT(interval as _) } == 0 {
                    Err(IoError::last_os_error().into())
                } else {
                    Ok(())
                }
            },
            _ => Err(ErrorKind::NotSupported("swap control not supported for surface").into()),
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

impl<T: SurfaceTypeTrait> fmt::Debug for Surface<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("config", &self.config.inner.pixel_format_index)
            .field("raw", &self.raw)
            .finish()
    }
}

impl<T: SurfaceTypeTrait> AsRawSurface for Surface<T> {
    fn raw_surface(&self) -> RawSurface {
        match self.raw {
            WglSurface::Window(hwnd, _) => RawSurface::Wgl(hwnd as _),
            WglSurface::PBuffer(hbuf, _) => RawSurface::Wgl(hbuf as _),
        }
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

impl<T: SurfaceTypeTrait> Sealed for Surface<T> {}

/// A wrapper around WGL surfaces.
#[derive(Debug)]
pub(crate) enum WglSurface {
    /// Surface backed by a window surface.
    Window(HWND, HDC),
    /// Surface backed by a pixel buffer.
    PBuffer(HPBUFFEREXT, HDC),
}

impl WglSurface {
    pub(crate) fn hdc(&self) -> HDC {
        *match self {
            WglSurface::Window(_, hdc) => hdc,
            WglSurface::PBuffer(_, hdc) => hdc,
        }
    }
}
