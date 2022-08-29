//! Library loading routines.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use libloading::Library;

#[cfg(windows)]
use libloading::os::windows::{Library as WinLibrary, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS};

pub trait SymLoading {
    /// # Safety
    /// The library must be unsured to live long enough.
    unsafe fn load_with(lib: &Library) -> Self;
}

#[derive(Clone)]
pub struct SymWrapper<T> {
    sym: T,
    _lib: Arc<Library>,
}

impl<T: SymLoading> SymWrapper<T> {
    pub unsafe fn new(lib_paths: &[&str]) -> Result<Self, ()> {
        unsafe {
            for path in lib_paths {
                #[cfg(windows)]
                let lib = WinLibrary::load_with_flags(path, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)
                    .map(From::from);

                #[cfg(not(windows))]
                let lib = Library::new(path);

                if let Ok(lib) = lib {
                    return Ok(SymWrapper { sym: T::load_with(&lib), _lib: Arc::new(lib) });
                }
            }
        }

        Err(())
    }
}

impl<T> Deref for SymWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.sym
    }
}

impl<T> DerefMut for SymWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sym
    }
}
