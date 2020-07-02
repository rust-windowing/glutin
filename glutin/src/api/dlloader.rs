#![cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

use libloading::Library;

#[cfg(target_os = "windows")]
use libloading::os::windows;

#[cfg(target_os = "windows")]
use winapi::um::libloaderapi::*;

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Clone)]
pub struct SymWrapper<T> {
    inner: T,
    _lib: Arc<Library>,
}

pub trait SymTrait {
    fn load_with(lib: &Library) -> Self;
}

impl<T: SymTrait> SymWrapper<T> {
    pub fn new(lib_paths: Vec<&str>) -> Result<Self, ()> {
        for path in lib_paths {
            // Avoid loading from PATH
            #[cfg(target_os = "windows")]
            let lib = windows::Library::load_with_flags(
                path,
                LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
            .map(From::from);

            #[cfg(not(target_os = "windows"))]
            let lib = Library::new(path);

            if lib.is_ok() {
                return Ok(SymWrapper {
                    inner: T::load_with(lib.as_ref().unwrap()),
                    _lib: Arc::new(lib.unwrap()),
                });
            }
        }

        Err(())
    }
}

impl<T> Deref for SymWrapper<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> DerefMut for SymWrapper<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
