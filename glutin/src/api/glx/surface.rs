//! Everything related to the GLXWindow.

use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::os::raw::{c_int, c_uint};

use glutin_glx_sys::glx::types::GLXWindow;
use glutin_glx_sys::{glx, glx_extra};
use raw_window_handle::RawWindowHandle;

use crate::config::GetGlConfig;
use crate::display::{DisplayFeatures, GetGlDisplay};
use crate::error::{ErrorKind, Result};
use crate::private::Sealed;
use crate::surface::{
    AsRawSurface, GlSurface, NativePixmap, PbufferSurface, PixmapSurface, RawSurface,
    SurfaceAttributes, SurfaceType, SurfaceTypeTrait, SwapInterval, WindowSurface,
};

use super::config::Config;
use super::context::PossiblyCurrentContext;
use super::display::Display;

/// Hint for the attributes array.
const ATTR_SIZE_HINT: usize = 8;

impl Display {
    pub(crate) unsafe fn create_pixmap_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Surface<PixmapSurface>> {
        let native_pixmap = surface_attributes.native_pixmap.as_ref().unwrap();
        let xid = match native_pixmap {
            NativePixmap::XlibPixmap(xid) => {
                if *xid == 0 {
                    return Err(ErrorKind::BadNativePixmap.into());
                }

                *xid
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided native pixmap is not supported.").into()
                )
            },
        };

        let mut attrs = Vec::<c_int>::with_capacity(ATTR_SIZE_HINT);

        // Push X11 `None` to terminate the list.
        attrs.push(0);

        let config = config.clone();
        let surface = super::last_glx_error(|| unsafe {
            self.inner.glx.CreatePixmap(
                self.inner.raw.cast(),
                *config.inner.raw,
                xid,
                attrs.as_ptr(),
            )
        })?;

        Ok(Surface {
            display: self.clone(),
            config,
            raw: surface,
            _nosendsync: PhantomData,
            _ty: PhantomData,
        })
    }

    pub(crate) unsafe fn create_pbuffer_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Surface<PbufferSurface>> {
        let width = surface_attributes.width.unwrap();
        let height = surface_attributes.height.unwrap();

        let mut attrs = Vec::<c_int>::with_capacity(ATTR_SIZE_HINT);

        attrs.push(glx::PBUFFER_WIDTH as c_int);
        attrs.push(width.get() as c_int);
        attrs.push(glx::PBUFFER_HEIGHT as c_int);
        attrs.push(height.get() as c_int);
        attrs.push(glx::LARGEST_PBUFFER as c_int);
        attrs.push(surface_attributes.largest_pbuffer as c_int);

        // Push X11 `None` to terminate the list.
        attrs.push(0);

        let config = config.clone();
        let surface = super::last_glx_error(|| unsafe {
            self.inner.glx.CreatePbuffer(self.inner.raw.cast(), *config.inner.raw, attrs.as_ptr())
        })?;

        Ok(Surface {
            display: self.clone(),
            config,
            raw: surface,
            _nosendsync: PhantomData,
            _ty: PhantomData,
        })
    }

    pub(crate) unsafe fn create_window_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Surface<WindowSurface>> {
        let window = match surface_attributes.raw_window_handle.unwrap() {
            RawWindowHandle::Xlib(window_handle) => {
                if window_handle.window == 0 {
                    return Err(ErrorKind::BadNativeWindow.into());
                }

                window_handle.window
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided native window is not supported").into()
                )
            },
        };

        let mut attrs = Vec::<c_int>::with_capacity(ATTR_SIZE_HINT);

        // Push X11 `None` to terminate the list.
        attrs.push(0);

        let config = config.clone();
        let surface = super::last_glx_error(|| unsafe {
            self.inner.glx.CreateWindow(
                self.inner.raw.cast(),
                *config.inner.raw,
                window,
                attrs.as_ptr() as *const _,
            )
        })?;

        Ok(Surface {
            display: self.clone(),
            config,
            raw: surface,
            _nosendsync: PhantomData,
            _ty: PhantomData,
        })
    }
}

/// A wrapper around the `GLXWindow`.
pub struct Surface<T: SurfaceTypeTrait> {
    display: Display,
    config: Config,
    pub(crate) raw: GLXWindow,
    _nosendsync: PhantomData<*const std::ffi::c_void>,
    _ty: PhantomData<T>,
}

impl<T: SurfaceTypeTrait> Surface<T> {
    /// # Safety
    ///
    /// The caller must ensure that the attribute could be present.
    unsafe fn raw_attribute(&self, attr: c_int) -> c_uint {
        unsafe {
            let mut value = 0;
            // This shouldn't generate any errors given that we know that the surface is
            // valid.
            self.display.inner.glx.QueryDrawable(
                self.display.inner.raw.cast(),
                self.raw,
                attr,
                &mut value,
            );
            value
        }
    }
}

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    fn drop(&mut self) {
        let _ = super::last_glx_error(|| unsafe {
            match T::surface_type() {
                SurfaceType::Pbuffer => {
                    self.display.inner.glx.DestroyPbuffer(self.display.inner.raw.cast(), self.raw);
                },
                SurfaceType::Window => {
                    self.display.inner.glx.DestroyWindow(self.display.inner.raw.cast(), self.raw);
                },
                SurfaceType::Pixmap => {
                    self.display.inner.glx.DestroyPixmap(self.display.inner.raw.cast(), self.raw);
                },
            }
        });
    }
}

impl<T: SurfaceTypeTrait> GlSurface<T> for Surface<T> {
    type Context = PossiblyCurrentContext;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        self.display
            .inner
            .client_extensions
            .contains("GLX_EXT_buffer_age")
            .then(|| unsafe { self.raw_attribute(glx_extra::BACK_BUFFER_AGE_EXT as c_int) })
            .unwrap_or(0) as u32
    }

    fn width(&self) -> Option<u32> {
        unsafe { Some(self.raw_attribute(glx::WIDTH as c_int) as u32) }
    }

    fn height(&self) -> Option<u32> {
        unsafe { Some(self.raw_attribute(glx::HEIGHT as c_int) as u32) }
    }

    fn is_single_buffered(&self) -> bool {
        self.config.is_single_buffered()
    }

    fn swap_buffers(&self, _context: &Self::Context) -> Result<()> {
        super::last_glx_error(|| unsafe {
            self.display.inner.glx.SwapBuffers(self.display.inner.raw.cast(), self.raw);
        })
    }

    fn set_swap_interval(&self, _context: &Self::Context, interval: SwapInterval) -> Result<()> {
        let extra = match self.display.inner.glx_extra {
            Some(extra) if self.display.inner.features.contains(DisplayFeatures::SWAP_CONTROL) => {
                extra
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("swap contol extrensions are not supported").into()
                );
            },
        };

        let interval = match interval {
            SwapInterval::DontWait => 0,
            SwapInterval::Wait(n) => n.get(),
        };

        let mut applied = false;

        // Apply the `EXT` first since it's per window.
        if !applied && self.display.inner.client_extensions.contains("GLX_EXT_swap_control") {
            super::last_glx_error(|| unsafe {
                // Check for error explicitly here, other apis do have indication for failure.
                extra.SwapIntervalEXT(self.display.inner.raw.cast(), self.raw, interval as _);
                applied = true;
            })?;
        }

        if !applied && self.display.inner.client_extensions.contains("GLX_MESA_swap_control") {
            unsafe {
                applied = extra.SwapIntervalMESA(interval as _) != glx::BAD_CONTEXT as _;
            }
        }

        if !applied && self.display.inner.client_extensions.contains("GLX_SGI_swap_control") {
            unsafe {
                applied = extra.SwapIntervalSGI(interval as _) != glx::BAD_CONTEXT as _;
            }
        }

        if applied {
            Ok(())
        } else {
            Err(ErrorKind::BadContext.into())
        }
    }

    fn is_current(&self, context: &Self::Context) -> bool {
        self.is_current_draw(context) && self.is_current_read(context)
    }

    fn is_current_draw(&self, _context: &Self::Context) -> bool {
        unsafe { self.display.inner.glx.GetCurrentDrawable() == self.raw }
    }

    fn is_current_read(&self, _context: &Self::Context) -> bool {
        unsafe { self.display.inner.glx.GetCurrentReadDrawable() == self.raw }
    }

    fn resize(&self, _context: &Self::Context, _width: NonZeroU32, _height: NonZeroU32) {
        // This isn't supported with GLXDrawable.
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

impl<T: SurfaceTypeTrait> fmt::Debug for Surface<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("display", &self.display.inner.raw)
            .field("config", &self.config.inner.raw)
            .field("raw", &self.raw)
            .field("type", &T::surface_type())
            .finish()
    }
}

impl<T: SurfaceTypeTrait> AsRawSurface for Surface<T> {
    fn raw_surface(&self) -> RawSurface {
        RawSurface::Glx(self.raw as u64)
    }
}

impl<T: SurfaceTypeTrait> Sealed for Surface<T> {}
