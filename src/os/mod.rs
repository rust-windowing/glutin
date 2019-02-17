//! Contains traits with platform-specific methods in them.
//!
//! Contains the following modules:
//!
//!  - `android`
//!  - `ios`
//!  - `macos`
//!  - `unix`
//!  - `windows`
//!

pub mod android;
pub mod ios;
pub mod macos;
pub mod unix;
pub mod windows;

use std::os::raw;

/// Platform-specific extensions for OpenGL contexts.
pub trait ContextTraitExt {
    /// Raw context handle.
    type Handle;

    /// Returns the raw context handle.
    unsafe fn raw_handle(&self) -> Self::Handle;

    /// Returns a pointer to the `EGLDisplay` object of EGL that is used by this
    /// context.
    ///
    /// Return `None` if the context doesn't use EGL.
    // The pointer will become invalid when the context is destroyed.
    unsafe fn get_egl_display(&self) -> Option<*const raw::c_void>;
}
