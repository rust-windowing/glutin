//! Utilities to access X11 specific config properties.

use std::mem;

use once_cell::sync::Lazy;
use x11_dl::xlib::{Display, XVisualInfo, Xlib};
#[cfg(egl_backend)]
use x11_dl::xlib::{VisualIDMask, XID};
use x11_dl::xrender::Xrender;

/// The XLIB handle.
pub(crate) static XLIB: Lazy<Option<Xlib>> = Lazy::new(|| Xlib::open().ok());

/// The XRENDER handle.
static XRENDER: Lazy<Option<Xrender>> = Lazy::new(|| Xrender::open().ok());

/// The GlConfig extension trait to get X11 specific properties from a config.
pub trait X11GlConfigExt {
    /// The `X11VisualInfo` that must be used to inititalize the Xlib window.
    fn x11_visual(&self) -> Option<X11VisualInfo>;
}

/// The X11 visual info.
///
/// This must be used when building X11 window, so it'll be compatible with the
/// underlying Api.
#[derive(Debug)]
pub struct X11VisualInfo {
    raw: *const XVisualInfo,
    transparency: bool,
}

impl X11VisualInfo {
    #[cfg(egl_backend)]
    pub(crate) unsafe fn from_xid(display: *mut Display, xid: XID) -> Option<Self> {
        let xlib = XLIB.as_ref().unwrap();

        if xid == 0 {
            return None;
        }

        let raw = unsafe {
            let mut raw: XVisualInfo = std::mem::zeroed();
            raw.visualid = xid;

            let mut num_visuals = 0;
            (xlib.XGetVisualInfo)(display, VisualIDMask, &mut raw, &mut num_visuals)
        };

        if raw.is_null() {
            return None;
        }

        let transparency = Self::has_non_zero_alpha(display, raw);

        Some(Self { raw, transparency })
    }

    #[cfg(glx_backend)]
    pub(crate) unsafe fn from_raw(display: *mut Display, raw: *const XVisualInfo) -> Self {
        let transparency = Self::has_non_zero_alpha(display, raw);
        Self { raw, transparency }
    }

    /// Returns `true` if the visual has non-zero alpha mask.
    pub fn supports_transparency(&self) -> bool {
        self.transparency
    }

    /// Get XID of for this visual.
    pub fn visual_id(&self) -> std::ffi::c_ulong {
        unsafe { (*self.raw).visualid }
    }

    /// Convert the visual to the raw pointer.
    ///
    /// You must clear it with `XFree` after the use.
    pub fn into_raw(self) -> *const std::ffi::c_void {
        let raw = self.raw as *const _;
        mem::forget(self);
        raw
    }

    pub(crate) fn has_non_zero_alpha(display: *mut Display, raw: *const XVisualInfo) -> bool {
        let xrender = XRENDER.as_ref().unwrap();
        unsafe {
            let visual_format = (xrender.XRenderFindVisualFormat)(display, (*raw).visual);

            (!visual_format.is_null())
                .then(|| (*visual_format).direct.alphaMask != 0)
                .unwrap_or(false)
        }
    }
}

impl Drop for X11VisualInfo {
    fn drop(&mut self) {
        unsafe {
            (XLIB.as_ref().unwrap().XFree)(self.raw as *mut _);
        }
    }
}
