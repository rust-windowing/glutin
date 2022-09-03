//! GLX platform Api.

use std::ffi::{self, CStr, CString};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use libloading::Library;
use once_cell::sync::Lazy;
use x11_dl::xlib::{self, XErrorEvent};

use glutin_glx_sys::{glx, glx_extra};

use crate::error::{Error, ErrorKind, Result};
use crate::lib_loading::{SymLoading, SymWrapper};
use crate::platform::x11::XLIB;

pub mod config;
pub mod context;
pub mod display;
pub mod surface;

/// When using Xlib we need to get errors from it somehow, however creating
/// inner `XDisplay` to handle that or change the error hook is unsafe in
/// multithreaded applications, given that error hook is per process and not
/// connection.
///
/// The hook registrar must call to the function inside xlib error
/// [`handler`].
///
/// [`handler`]: https://tronche.com/gui/x/xlib/event-handling/protocol-errors/XSetErrorHandler.html
pub type XlibErrorHookRegistrar =
    Box<dyn Fn(Box<dyn Fn(*mut ffi::c_void, *mut ffi::c_void) -> bool + Send + Sync>)>;

/// The base used for GLX errors.
static GLX_BASE_ERROR: AtomicI32 = AtomicI32::new(0);

/// The last error arrived from GLX normalized by `GLX_BASE_ERROR`.
static LAST_GLX_ERROR: Lazy<Mutex<Option<Error>>> = Lazy::new(|| Mutex::new(None));

static GLX: Lazy<Option<Glx>> = Lazy::new(|| {
    let paths = ["libGL.so.1", "libGL.so"];

    unsafe { SymWrapper::new(&paths).map(Glx).ok() }
});

static GLX_EXTRA: Lazy<Option<GlxExtra>> = Lazy::new(|| {
    let glx = GLX.as_ref()?;
    Some(GlxExtra::new(glx))
});

pub(crate) struct Glx(pub SymWrapper<glx::Glx>);

unsafe impl Sync for Glx {}
unsafe impl Send for Glx {}

impl SymLoading for glx::Glx {
    unsafe fn load_with(lib: &Library) -> Self {
        Self::load_with(|sym| unsafe {
            lib.get(CString::new(sym.as_bytes()).unwrap().as_bytes_with_nul())
                .map(|sym| *sym)
                .unwrap_or(std::ptr::null_mut())
        })
    }
}

impl Deref for Glx {
    type Target = glx::Glx;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Glx {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) struct GlxExtra(glx_extra::Glx);

unsafe impl Sync for GlxExtra {}
unsafe impl Send for GlxExtra {}

impl GlxExtra {
    #[inline]
    pub fn new(glx: &Glx) -> Self {
        GlxExtra(glx_extra::Glx::load_with(|proc_name| {
            let c_str = CString::new(proc_name).unwrap();
            unsafe { glx.GetProcAddress(c_str.as_ptr() as *const u8) as *const _ }
        }))
    }
}

impl Deref for GlxExtra {
    type Target = glx_extra::Glx;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GlxExtra {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
/// Store the last error received from the GLX.
fn glx_error_hook(_display: *mut ffi::c_void, xerror_event: *mut ffi::c_void) -> bool {
    let xerror = xerror_event as *mut XErrorEvent;
    unsafe {
        let code = (*xerror).error_code;
        let glx_code = code as i32 - GLX_BASE_ERROR.load(Ordering::Relaxed);

        // Get the kind of the error.
        let kind = match code as u8 {
            xlib::BadValue => ErrorKind::BadAttribute,
            xlib::BadMatch => ErrorKind::BadMatch,
            xlib::BadWindow => ErrorKind::BadNativeWindow,
            xlib::BadAlloc => ErrorKind::OutOfMemory,
            xlib::BadPixmap => ErrorKind::BadPixmap,
            xlib::BadAccess => ErrorKind::BadAccess,
            _ if glx_code >= 0 => match glx_code as glx::types::GLenum {
                glx::PROTO_BAD_CONTEXT => ErrorKind::BadContext,
                glx::PROTO_BAD_CONTEXT_STATE => ErrorKind::BadContext,
                glx::PROTO_BAD_CURRENT_DRAWABLE => ErrorKind::BadCurrentSurface,
                glx::PROTO_BAD_CURRENT_WINDOW => ErrorKind::BadCurrentSurface,
                glx::PROTO_BAD_FBCONFIG => ErrorKind::BadConfig,
                glx::PROTO_BAD_PBUFFER => ErrorKind::BadPbuffer,
                glx::PROTO_BAD_PIXMAP => ErrorKind::BadPixmap,
                glx::PROTO_UNSUPPORTED_PRIVATE_REQUEST => ErrorKind::Misc,
                glx::PROTO_BAD_DRAWABLE => ErrorKind::BadSurface,
                glx::PROTO_BAD_WINDOW => ErrorKind::BadSurface,
                glx::PROTO_BAD_CONTEXT_TAG => ErrorKind::Misc,
                glx::PROTO_BAD_RENDER_REQUEST => ErrorKind::Misc,
                glx::PROTO_BAD_LARGE_REQUEST => ErrorKind::Misc,
                _ => return false,
            },
            _ => return false,
        };

        // Get the string from X11 error.
        let mut buf = vec![0u8; 1024];
        (XLIB.as_ref().unwrap().XGetErrorText)(
            _display as *mut _,
            (*xerror).error_code as _,
            buf.as_mut_ptr() as *mut _,
            buf.len() as _,
        );
        let description = CStr::from_ptr(buf.as_ptr() as *const _).to_string_lossy().to_string();

        *LAST_GLX_ERROR.lock().unwrap() =
            Some(Error::new(Some(code as _), Some(description), kind));

        true
    }
}

/// Get the error from the X11.
fn last_glx_error(display: display::GlxDisplay) -> Result<()> {
    unsafe {
        // Force synchronization.
        (XLIB.as_ref().unwrap().XSync)(*display as *mut _, 0);
    }

    // Reset and report last error.
    let last_error = LAST_GLX_ERROR.lock().unwrap().take();
    match last_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
