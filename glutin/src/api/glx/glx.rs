use super::ffi;
use crate::api::dlloader::{SymTrait, SymWrapper};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Glx(SymWrapper<ffi::glx::Glx>);

/// Because `*const raw::c_void` doesn't implement `Sync`.
unsafe impl Sync for Glx {}

impl SymTrait for ffi::glx::Glx {
    fn load_with(lib: &libloading::Library) -> Self {
        Self::load_with(|sym| unsafe {
            lib.get(
                std::ffi::CString::new(sym.as_bytes())
                    .unwrap()
                    .as_bytes_with_nul(),
            )
            .map(|sym| *sym)
            .unwrap_or(std::ptr::null_mut())
        })
    }
}

impl Glx {
    pub fn new() -> Result<Self, ()> {
        let paths = vec!["libGL.so.1", "libGL.so"];

        SymWrapper::new(paths).map(|i| Glx(i))
    }
}

impl Deref for Glx {
    type Target = ffi::glx::Glx;

    fn deref(&self) -> &ffi::glx::Glx {
        &self.0
    }
}

impl DerefMut for Glx {
    fn deref_mut(&mut self) -> &mut ffi::glx::Glx {
        &mut self.0
    }
}
