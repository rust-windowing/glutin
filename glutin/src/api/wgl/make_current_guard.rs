use super::gl;
use crate::CreationError;

use winapi::shared::windef::{HDC, HGLRC};

use std::io;
use std::marker::PhantomData;
use std::os::raw;

/// A guard for when you want to make the context current. Destroying the guard
/// restores the previously-current context.
#[derive(Debug)]
pub struct CurrentContextGuard<'a, 'b> {
    previous_hdc: HDC,
    previous_hglrc: HGLRC,
    marker1: PhantomData<&'a ()>,
    marker2: PhantomData<&'b ()>,
}

impl<'a, 'b> CurrentContextGuard<'a, 'b> {
    pub unsafe fn make_current(
        hdc: HDC,
        context: HGLRC,
    ) -> Result<CurrentContextGuard<'a, 'b>, CreationError> {
        let previous_hdc = gl::wgl::GetCurrentDC() as HDC;
        let previous_hglrc = gl::wgl::GetCurrentContext() as HGLRC;

        let result = gl::wgl::MakeCurrent(hdc as *const _, context as *const _);
        if result == 0 {
            return Err(CreationError::OsError(format!(
                "wglMakeCurrent function failed: {}",
                io::Error::last_os_error()
            )));
        }

        Ok(CurrentContextGuard {
            previous_hdc: previous_hdc,
            previous_hglrc: previous_hglrc,
            marker1: PhantomData,
            marker2: PhantomData,
        })
    }
}

impl<'a, 'b> Drop for CurrentContextGuard<'a, 'b> {
    fn drop(&mut self) {
        unsafe {
            gl::wgl::MakeCurrent(
                self.previous_hdc as *const raw::c_void,
                self.previous_hglrc as *const raw::c_void,
            );
        }
    }
}
