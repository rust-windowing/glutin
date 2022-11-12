//! EGL platform Api.
//!
//! This platform is typically available on Linux, Android and other Unix-like
//! platforms.
//!
//! The EGL platform allows creating a [`Display`](self::display::Display) from
//! a [`Device`](self::device::Device).

use std::ffi::{self, CString};
use std::ops::{Deref, DerefMut};

use glutin_egl_sys::egl;

use libloading::Library;
use once_cell::sync::{Lazy, OnceCell};

#[cfg(unix)]
use libloading::os::unix as libloading_os;
#[cfg(windows)]
use libloading::os::windows as libloading_os;

use crate::error::{Error, ErrorKind, Result};
use crate::lib_loading::{SymLoading, SymWrapper};

pub mod config;
pub mod context;
pub mod device;
pub mod display;
pub mod surface;

pub(crate) static EGL: Lazy<Option<Egl>> = Lazy::new(|| {
    #[cfg(windows)]
    let paths = ["libEGL.dll", "atioglxx.dll"];

    #[cfg(not(windows))]
    let paths = ["libEGL.so.1", "libEGL.so"];

    unsafe { SymWrapper::new(&paths).map(Egl).ok() }
});

type EglGetProcAddress = unsafe extern "C" fn(*const ffi::c_void) -> *const ffi::c_void;
static EGL_GET_PROC_ADDRESS: OnceCell<libloading_os::Symbol<EglGetProcAddress>> = OnceCell::new();

pub(crate) struct Egl(pub SymWrapper<egl::Egl>);

unsafe impl Sync for Egl {}
unsafe impl Send for Egl {}

impl SymLoading for egl::Egl {
    unsafe fn load_with(lib: &Library) -> Self {
        let loader = move |sym_name: &'static str| -> *const ffi::c_void {
            unsafe {
                let sym_name = CString::new(sym_name.as_bytes()).unwrap();
                if let Ok(sym) = lib.get(sym_name.as_bytes_with_nul()) {
                    return *sym;
                }

                let egl_proc_address = EGL_GET_PROC_ADDRESS.get_or_init(|| {
                    let sym: libloading::Symbol<'_, EglGetProcAddress> =
                        lib.get(b"eglGetProcAddress\0").unwrap();
                    sym.into_raw()
                });

                // The symbol was not available in the library, so ask eglGetProcAddress for it.
                // Note that eglGetProcAddress was only able to look up extension
                // functions prior to EGL 1.5, hence this two-part dance.
                (egl_proc_address)(sym_name.as_bytes_with_nul().as_ptr() as *const ffi::c_void)
            }
        };

        Self::load_with(loader)
    }
}

impl Deref for Egl {
    type Target = egl::Egl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Egl {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Obtain the error from the EGL.
fn check_error() -> Result<()> {
    let egl = EGL.as_ref().unwrap();
    unsafe {
        let raw_code = egl.GetError() as egl::types::EGLenum;
        let kind = match raw_code {
            egl::SUCCESS => return Ok(()),
            egl::NOT_INITIALIZED => ErrorKind::InitializationFailed,
            egl::BAD_ACCESS => ErrorKind::BadAccess,
            egl::BAD_ALLOC => ErrorKind::OutOfMemory,
            egl::BAD_ATTRIBUTE => ErrorKind::BadAttribute,
            egl::BAD_CONTEXT => ErrorKind::BadContext,
            egl::BAD_CONFIG => ErrorKind::BadConfig,
            egl::BAD_CURRENT_SURFACE => ErrorKind::BadCurrentSurface,
            egl::BAD_DISPLAY => ErrorKind::BadDisplay,
            egl::BAD_SURFACE => ErrorKind::BadSurface,
            egl::BAD_MATCH => ErrorKind::BadMatch,
            egl::BAD_PARAMETER => ErrorKind::BadParameter,
            egl::BAD_NATIVE_PIXMAP => ErrorKind::BadNativePixmap,
            egl::BAD_NATIVE_WINDOW => ErrorKind::BadNativeWindow,
            egl::CONTEXT_LOST => ErrorKind::ContextLost,
            _ => ErrorKind::Misc,
        };

        Err(Error::new(Some(raw_code as i64), None, kind))
    }
}
