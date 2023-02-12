//! Everything related to `EGLSurface`.

use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::{ffi, fmt};

use glutin_egl_sys::egl;
use glutin_egl_sys::egl::types::{EGLAttrib, EGLSurface, EGLint};
use raw_window_handle::RawWindowHandle;
#[cfg(wayland_platform)]
use wayland_sys::{egl::*, ffi_dispatch};

use crate::api::egl::display::EglDisplay;
use crate::config::GetGlConfig;
use crate::display::GetGlDisplay;
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{
    AsRawSurface, NativePixmap, PbufferSurface, PixmapSurface, RawSurface, Rect, SurfaceAttributes,
    SurfaceTypeTrait, SwapInterval, WindowSurface,
};

use super::config::Config;
use super::context::PossiblyCurrentContext;
use super::display::Display;

/// Hint for the attribute list size.
const ATTR_SIZE_HINT: usize = 8;

impl Display {
    pub(crate) unsafe fn create_pbuffer_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Surface<PbufferSurface>> {
        let width = surface_attributes.width.unwrap();
        let height = surface_attributes.height.unwrap();

        // XXX Window surface is using `EGLAttrib` and not `EGLint`.
        let mut attrs = Vec::<EGLint>::with_capacity(ATTR_SIZE_HINT);

        // Add dimensions.
        attrs.push(egl::WIDTH as EGLint);
        attrs.push(width.get() as EGLint);

        attrs.push(egl::HEIGHT as EGLint);
        attrs.push(height.get() as EGLint);

        // Add information about render buffer.
        attrs.push(egl::RENDER_BUFFER as EGLint);
        let buffer =
            if surface_attributes.single_buffer { egl::SINGLE_BUFFER } else { egl::BACK_BUFFER }
                as EGLint;
        attrs.push(buffer);

        // Push `egl::NONE` to terminate the list.
        attrs.push(egl::NONE as EGLint);

        let config = config.clone();
        let surface = unsafe {
            Self::check_surface_error(self.inner.egl.CreatePbufferSurface(
                *self.inner.raw,
                *config.inner.raw,
                attrs.as_ptr(),
            ))?
        };

        Ok(Surface {
            display: self.clone(),
            native_window: None,
            config,
            raw: surface,
            _ty: PhantomData,
        })
    }

    pub(crate) unsafe fn create_pixmap_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Surface<PixmapSurface>> {
        let native_pixmap = surface_attributes.native_pixmap.as_ref().unwrap();

        let mut attrs = Vec::<EGLAttrib>::with_capacity(ATTR_SIZE_HINT);

        if surface_attributes.srgb.is_some() && config.srgb_capable() {
            attrs.push(egl::GL_COLORSPACE as EGLAttrib);
            let colorspace = match surface_attributes.srgb {
                Some(true) => egl::GL_COLORSPACE_SRGB as EGLAttrib,
                _ => egl::GL_COLORSPACE_LINEAR as EGLAttrib,
            };
            attrs.push(colorspace);
        }

        // Push `egl::NONE` to terminate the list.
        attrs.push(egl::NONE as EGLAttrib);

        let config = config.clone();
        let surface = match self.inner.raw {
            EglDisplay::Khr(display) => unsafe {
                self.inner.egl.CreatePlatformPixmapSurface(
                    display,
                    *config.inner.raw,
                    native_pixmap.as_ptr(),
                    attrs.as_ptr(),
                )
            },
            EglDisplay::Ext(display) => unsafe {
                let attrs: Vec<EGLint> = attrs.into_iter().map(|attr| attr as EGLint).collect();
                self.inner.egl.CreatePlatformPixmapSurfaceEXT(
                    display,
                    *config.inner.raw,
                    native_pixmap.as_ptr(),
                    attrs.as_ptr(),
                )
            },
            EglDisplay::Legacy(display) => unsafe {
                // This call accepts raw value, instead of pointer.
                let attrs: Vec<EGLint> = attrs.into_iter().map(|attr| attr as EGLint).collect();
                self.inner.egl.CreatePixmapSurface(
                    display,
                    *config.inner.raw,
                    *(native_pixmap.as_ptr() as *const egl::NativePixmapType),
                    attrs.as_ptr(),
                )
            },
        };

        let surface = Self::check_surface_error(surface)?;

        Ok(Surface {
            display: self.clone(),
            config,
            native_window: None,
            raw: surface,
            _ty: PhantomData,
        })
    }

    pub(crate) unsafe fn create_window_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Surface<WindowSurface>> {
        // Create native window.
        let native_window = NativeWindow::new(
            surface_attributes.width.unwrap(),
            surface_attributes.height.unwrap(),
            surface_attributes.raw_window_handle.as_ref().unwrap(),
        )?;

        // XXX Window surface is using `EGLAttrib` and not `EGLint`.
        let mut attrs = Vec::<EGLAttrib>::with_capacity(ATTR_SIZE_HINT);

        // Add information about render buffer.
        attrs.push(egl::RENDER_BUFFER as EGLAttrib);
        let buffer =
            if surface_attributes.single_buffer { egl::SINGLE_BUFFER } else { egl::BACK_BUFFER }
                as EGLAttrib;
        attrs.push(buffer);

        // // Add colorspace if the extension is present.
        if surface_attributes.srgb.is_some() && config.srgb_capable() {
            attrs.push(egl::GL_COLORSPACE as EGLAttrib);
            let colorspace = match surface_attributes.srgb {
                Some(true) => egl::GL_COLORSPACE_SRGB as EGLAttrib,
                _ => egl::GL_COLORSPACE_LINEAR as EGLAttrib,
            };
            attrs.push(colorspace);
        }

        // Push `egl::NONE` to terminate the list.
        attrs.push(egl::NONE as EGLAttrib);

        let config = config.clone();

        let surface = match self.inner.raw {
            EglDisplay::Khr(display) => unsafe {
                self.inner.egl.CreatePlatformWindowSurface(
                    display,
                    *config.inner.raw,
                    native_window.as_ptr().cast(),
                    attrs.as_ptr(),
                )
            },
            EglDisplay::Ext(display) => unsafe {
                let attrs: Vec<EGLint> = attrs.into_iter().map(|attr| attr as EGLint).collect();
                self.inner.egl.CreatePlatformWindowSurfaceEXT(
                    display,
                    *config.inner.raw,
                    native_window.as_ptr().cast(),
                    attrs.as_ptr(),
                )
            },
            EglDisplay::Legacy(display) => unsafe {
                let attrs: Vec<EGLint> = attrs.into_iter().map(|attr| attr as EGLint).collect();
                // This call accepts raw value, instead of pointer.
                self.inner.egl.CreateWindowSurface(
                    display,
                    *config.inner.raw,
                    *(native_window.as_ptr() as *const egl::NativeWindowType),
                    attrs.as_ptr(),
                )
            },
        };

        let surface = Self::check_surface_error(surface)?;

        Ok(Surface {
            display: self.clone(),
            config,
            native_window: Some(native_window),
            raw: surface,
            _ty: PhantomData,
        })
    }

    fn check_surface_error(surface: EGLSurface) -> Result<EGLSurface> {
        if surface == egl::NO_SURFACE {
            Err(super::check_error().err().unwrap())
        } else {
            Ok(surface)
        }
    }
}

/// A wrapper around `EGLSurface`.
pub struct Surface<T: SurfaceTypeTrait> {
    display: Display,
    config: Config,
    pub(crate) raw: EGLSurface,
    native_window: Option<NativeWindow>,
    _ty: PhantomData<T>,
}

impl<T: SurfaceTypeTrait> Surface<T> {
    /// Swaps the underlying back buffers when the surface is not single
    /// buffered and pass the [`Rect`] information to the system
    /// compositor. Providing empty slice will damage the entire surface.
    ///
    /// When the underlying extensions are not supported the function acts like
    /// [`Self::swap_buffers`].
    ///
    /// This Api doesn't do any partial rendering, it just provides hints for
    /// the system compositor.
    pub fn swap_buffers_with_damage(
        &self,
        context: &PossiblyCurrentContext,
        rects: &[Rect],
    ) -> Result<()> {
        context.inner.bind_api();

        let res = unsafe {
            if self.display.inner.client_extensions.contains("EGL_KHR_swap_buffers_with_damage") {
                self.display.inner.egl.SwapBuffersWithDamageKHR(
                    *self.display.inner.raw,
                    self.raw,
                    rects.as_ptr() as *mut _,
                    rects.len() as _,
                )
            } else if self
                .display
                .inner
                .client_extensions
                .contains("EGL_EXT_swap_buffers_with_damage")
            {
                self.display.inner.egl.SwapBuffersWithDamageEXT(
                    *self.display.inner.raw,
                    self.raw,
                    rects.as_ptr() as *mut _,
                    rects.len() as _,
                )
            } else {
                self.display.inner.egl.SwapBuffers(*self.display.inner.raw, self.raw)
            }
        };

        if res == egl::FALSE {
            super::check_error()
        } else {
            Ok(())
        }
    }

    /// # Safety
    ///
    /// The caller must ensure that the attribute could be present.
    unsafe fn raw_attribute(&self, attr: EGLint) -> EGLint {
        unsafe {
            let mut value = 0;
            self.display.inner.egl.QuerySurface(
                *self.display.inner.raw,
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
        unsafe {
            self.display.inner.egl.DestroySurface(*self.display.inner.raw, self.raw);
        }
    }
}

impl<T: SurfaceTypeTrait> GlSurface<T> for Surface<T> {
    type Context = PossiblyCurrentContext;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        self.display
            .inner
            .client_extensions
            .contains("EGL_EXT_buffer_age")
            .then(|| unsafe { self.raw_attribute(egl::BUFFER_AGE_EXT as EGLint) })
            .unwrap_or(0) as u32
    }

    fn width(&self) -> Option<u32> {
        unsafe { Some(self.raw_attribute(egl::WIDTH as EGLint) as u32) }
    }

    fn height(&self) -> Option<u32> {
        unsafe { Some(self.raw_attribute(egl::HEIGHT as EGLint) as u32) }
    }

    fn is_single_buffered(&self) -> bool {
        unsafe { self.raw_attribute(egl::RENDER_BUFFER as EGLint) != egl::SINGLE_BUFFER as i32 }
    }

    fn swap_buffers(&self, context: &Self::Context) -> Result<()> {
        unsafe {
            context.inner.bind_api();

            if self.display.inner.egl.SwapBuffers(*self.display.inner.raw, self.raw) == egl::FALSE {
                super::check_error()
            } else {
                Ok(())
            }
        }
    }

    fn set_swap_interval(&self, context: &Self::Context, interval: SwapInterval) -> Result<()> {
        unsafe {
            context.inner.bind_api();

            let interval = match interval {
                SwapInterval::DontWait => 0,
                SwapInterval::Wait(interval) => interval.get() as EGLint,
            };
            if self.display.inner.egl.SwapInterval(*self.display.inner.raw, interval) == egl::FALSE
            {
                super::check_error()
            } else {
                Ok(())
            }
        }
    }

    fn is_current(&self, context: &Self::Context) -> bool {
        self.is_current_draw(context) && self.is_current_read(context)
    }

    fn is_current_draw(&self, context: &Self::Context) -> bool {
        unsafe {
            context.inner.bind_api();
            self.display.inner.egl.GetCurrentSurface(egl::DRAW as EGLint) == self.raw
        }
    }

    fn is_current_read(&self, context: &Self::Context) -> bool {
        unsafe {
            context.inner.bind_api();
            self.display.inner.egl.GetCurrentSurface(egl::READ as EGLint) == self.raw
        }
    }

    fn resize(&self, _context: &Self::Context, width: NonZeroU32, height: NonZeroU32) {
        self.native_window.as_ref().unwrap().resize(width, height)
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
        RawSurface::Egl(self.raw)
    }
}

impl<T: SurfaceTypeTrait> fmt::Debug for Surface<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("display", &self.display.inner.raw)
            .field("config", &self.config.inner.raw)
            .field("raw", &self.raw)
            .field("native_window", &self.native_window)
            .field("type", &T::surface_type())
            .finish()
    }
}

impl<T: SurfaceTypeTrait> Sealed for Surface<T> {}

#[derive(Debug)]
enum NativeWindow {
    #[cfg(wayland_platform)]
    Wayland(*mut ffi::c_void),

    #[cfg(x11_platform)]
    Xlib(std::os::raw::c_ulong),

    #[cfg(x11_platform)]
    Xcb(u32),

    Android(*mut ffi::c_void),

    Win32(*mut ffi::c_void),

    Gbm(*mut ffi::c_void),
}

impl NativeWindow {
    fn new(
        _width: NonZeroU32,
        _height: NonZeroU32,
        raw_window_handle: &RawWindowHandle,
    ) -> Result<Self> {
        let native_window = match raw_window_handle {
            #[cfg(wayland_platform)]
            RawWindowHandle::Wayland(window_handle) => unsafe {
                let ptr = ffi_dispatch!(
                    WAYLAND_EGL_HANDLE,
                    wl_egl_window_create,
                    window_handle.surface.cast(),
                    _width.get() as _,
                    _height.get() as _
                );
                if ptr.is_null() {
                    return Err(ErrorKind::OutOfMemory.into());
                }
                Self::Wayland(ptr.cast())
            },
            #[cfg(x11_platform)]
            RawWindowHandle::Xlib(window_handle) => Self::Xlib(window_handle.window as _),
            #[cfg(x11_platform)]
            RawWindowHandle::Xcb(window_handle) => Self::Xcb(window_handle.window as _),
            RawWindowHandle::AndroidNdk(window_handle) => {
                Self::Android(window_handle.a_native_window)
            },
            RawWindowHandle::Win32(window_hanlde) => Self::Win32(window_hanlde.hwnd),
            RawWindowHandle::Gbm(window_handle) => Self::Gbm(window_handle.gbm_surface),
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided native window is not supported").into()
                )
            },
        };

        Ok(native_window)
    }

    fn resize(&self, _width: NonZeroU32, _height: NonZeroU32) {
        #[cfg(wayland_platform)]
        if let Self::Wayland(wl_egl_surface) = self {
            unsafe {
                ffi_dispatch!(
                    WAYLAND_EGL_HANDLE,
                    wl_egl_window_resize,
                    *wl_egl_surface as _,
                    _width.get() as _,
                    _height.get() as _,
                    0,
                    0
                )
            }
        }
    }

    fn as_ptr(&self) -> *mut ffi::c_void {
        match self {
            #[cfg(wayland_platform)]
            Self::Wayland(wl_egl_surface) => *wl_egl_surface,
            #[cfg(x11_platform)]
            Self::Xlib(window_id) => window_id as *const _ as *mut ffi::c_void,
            #[cfg(x11_platform)]
            Self::Xcb(window_id) => window_id as *const _ as *mut ffi::c_void,
            Self::Win32(hwnd) => *hwnd,
            Self::Android(a_native_window) => *a_native_window,
            Self::Gbm(gbm_surface) => *gbm_surface,
        }
    }
}

#[cfg(wayland_platform)]
impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe {
            if let Self::Wayland(wl_egl_window) = self {
                ffi_dispatch!(WAYLAND_EGL_HANDLE, wl_egl_window_destroy, wl_egl_window.cast());
            }
        }
    }
}

impl NativePixmap {
    fn as_ptr(&self) -> *mut ffi::c_void {
        match self {
            Self::XlibPixmap(xid) => xid as *const _ as *mut _,
            Self::XcbPixmap(xid) => xid as *const _ as *mut _,
            Self::WindowsPixmap(hbitmap) => hbitmap as *const _ as *mut _,
        }
    }
}
