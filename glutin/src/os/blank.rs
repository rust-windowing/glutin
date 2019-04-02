#![cfg(not(any(
    target_os = "ios",
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "emscripten",
)))]

use crate::os::ContextTraitExt;
use crate::{Context, ContextCurrentState};

use std::os::raw;

impl<T: ContextCurrentState> ContextTraitExt for Context<T> {
    type Handle = ();

    #[inline]
    unsafe fn raw_handle(&self) -> Self::Handle {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }

    #[inline]
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        unimplemented!("Glutin-Blank: Platform unsupported")
    }
}
