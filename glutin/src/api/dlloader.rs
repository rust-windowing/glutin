#![cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

use libloading::Library;

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
