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
        let hdc = config.inner.hdc;
        let width = surface_attributes.width.unwrap().get() as c_int;
        let height = surface_attributes.height.unwrap().get() as c_int;

        let mut attrs = Vec::<c_int>::with_capacity(2);
        if surface_attributes.largest_pbuffer {
            attrs.push(wgl_extra::PBUFFER_LARGEST_ARB as c_int);
            attrs.push(1 as c_int);
        }
        attrs.push(0);

        let (hbuf, hdc) = match self.inner.wgl_extra {
            Some(extra) if extra.CreatePbufferARB.is_loaded() => unsafe {
                let hbuf = extra.CreatePbufferARB(
                    hdc as _,
                    config.inner.pixel_format_index,
                    width,
                    height,
                    attrs.as_ptr(),
                );
                let hdc = extra.GetPbufferDCARB(hbuf);
                (hbuf, hdc)
            },
            _ => return Err(ErrorKind::NotSupported("pbuffer extensions are not supported").into()),
        };

        let surface = Surface {
            display: self.clone(),
            config: config.clone(),
            raw: WGLSurface::PBuffer(hbuf, hdc),
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
            raw: WGLSurface::Window(hwnd, hdc),
            _ty: PhantomData,
        };

        Ok(surface)
    }
}

/// A wrapper around WGL surfaces.
#[derive(Debug)]
pub enum WGLSurface {
    /// Surface backed by a window surface.
    Window(HWND, gdi::HDC),
    /// Surface backed by a pixel buffer.
    PBuffer(HPBUFFEREXT, wgl_extra::types::HDC),
}

/// A Wrapper around `WGLSurface`.
pub struct Surface<T: SurfaceTypeTrait> {
    display: Display,
    config: Config,
    pub(crate) raw: WGLSurface,
    _ty: PhantomData<T>,
}

// Impl only `Send` for Surface.
unsafe impl<T: SurfaceTypeTrait> Send for Surface<T> {}

impl<T: SurfaceTypeTrait> Surface<T> {
    /// # Safety
    ///
    /// The caller must ensure that the attribute could be present.
    unsafe fn raw_attribute(&self, attr: GLenum) -> c_int {
        let mut value = 0;
        unsafe {
            match self.raw {
                WGLSurface::Window(..) => unreachable!(),
                WGLSurface::PBuffer(hbuf, _) => {
                    if let Some(extra) = self.display.inner.wgl_extra {
                        extra.QueryPbufferARB(hbuf, attr as _, &mut value);
                    }
                },
            }
        }
        value
    }
}

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    fn drop(&mut self) {
        unsafe {
            match self.raw {
                WGLSurface::Window(hwnd, hdc) => {
                    gdi::ReleaseDC(hwnd, hdc);
                },
                WGLSurface::PBuffer(hbuf, hdc) => {
                    if let Some(extra) = self.display.inner.wgl_extra {
                        extra.ReleasePbufferDCARB(hbuf, hdc);
                        extra.DestroyPbufferARB(hbuf);
                    }
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
            WGLSurface::Window(hwnd, _) => {
                let mut rect: RECT = unsafe { mem::zeroed() };
                if unsafe { GetClientRect(hwnd, &mut rect) } == false.into() {
                    None
                } else {
                    Some((rect.right - rect.left) as u32)
                }
            },
            WGLSurface::PBuffer(..) => unsafe {
                Some(self.raw_attribute(wgl_extra::PBUFFER_WIDTH_ARB) as _)
            },
        }
    }

    fn height(&self) -> Option<u32> {
        match self.raw {
            WGLSurface::Window(hwnd, _) => {
                let mut rect: RECT = unsafe { mem::zeroed() };
                if unsafe { GetClientRect(hwnd, &mut rect) } == false.into() {
                    None
                } else {
                    Some((rect.bottom - rect.top) as u32)
                }
            },
            WGLSurface::PBuffer(..) => unsafe {
                Some(self.raw_attribute(wgl_extra::PBUFFER_HEIGHT_ARB) as _)
            },
        }
    }

    fn is_single_buffered(&self) -> bool {
        self.config.is_single_buffered()
    }

    fn swap_buffers(&self, _context: &Self::Context) -> Result<()> {
        unsafe {
            let hdc = match self.raw {
                WGLSurface::Window(_, hdc) => hdc as _,
                WGLSurface::PBuffer(_, hdc) => hdc as _,
            };

            if gl::SwapBuffers(hdc) == 0 {
                Err(IoError::last_os_error().into())
            } else {
                Ok(())
            }
        }
    }

    fn set_swap_interval(&self, _context: &Self::Context, interval: SwapInterval) -> Result<()> {
        let WGLSurface::Window(..) = self.raw else {
            return Ok(());
        };

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
                    ErrorKind::NotSupported("swap control extensions are not supported").into()
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
            WGLSurface::Window(hwnd, _) => RawSurface::Wgl(hwnd as _),
            WGLSurface::PBuffer(..) => RawSurface::Wgl(0 as _),
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
