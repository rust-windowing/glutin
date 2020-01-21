use super::ffi;
use crate::api::dlloader::{SymTrait, SymWrapper};

use std::ffi::CString;
use std::ops::{Deref, DerefMut};

use winit_types::error::Error;
use winit_types::platform::OsError;

#[derive(Clone)]
pub struct Glx(SymWrapper<ffi::glx::Glx>);

/// Because `*const raw::c_void` doesn't implement `Sync`.
unsafe impl Sync for Glx {}

impl SymTrait for ffi::glx::Glx {
    #[inline]
    fn load_with(lib: &libloading::Library) -> Self {
        Self::load_with(|sym| unsafe {
            lib.get(CString::new(sym.as_bytes()).unwrap().as_bytes_with_nul())
                .map(|sym| *sym)
                .unwrap_or(std::ptr::null_mut())
        })
    }
}

impl Glx {
    #[inline]
    pub fn new() -> Result<Self, Error> {
        let paths = vec!["libGL.so.1", "libGL.so"];

        SymWrapper::new(paths)
            .map(|i| Glx(i))
            .map_err(|_| make_oserror!(OsError::Misc("Could not load Glx symbols".to_string())))
    }
}

impl Deref for Glx {
    type Target = ffi::glx::Glx;

    #[inline]
    fn deref(&self) -> &ffi::glx::Glx {
        &self.0
    }
}

impl DerefMut for Glx {
    #[inline]
    fn deref_mut(&mut self) -> &mut ffi::glx::Glx {
        &mut self.0
    }
}

#[derive(Clone)]
pub struct GlxExtra(ffi::glx_extra::Glx);

/// Because `*const raw::c_void` doesn't implement `Sync`.
unsafe impl Sync for GlxExtra {}

impl GlxExtra {
    #[inline]
    pub fn new(glx: &Glx) -> Self {
        GlxExtra(ffi::glx_extra::Glx::load_with(|proc_name| {
            let c_str = CString::new(proc_name).unwrap();
            unsafe { glx.GetProcAddress(c_str.as_ptr() as *const u8) as *const _ }
        }))
    }
}

impl Deref for GlxExtra {
    type Target = ffi::glx_extra::Glx;

    #[inline]
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
