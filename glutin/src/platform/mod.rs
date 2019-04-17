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

/// Platform-specific methods for android.
pub mod android;
/// Platform-specific methods for blank backend.
pub mod blank;
/// Platform-specific methods for iOS.
pub mod ios;
/// Platform-specific methods for macOS.
pub mod macos;
/// Platform-specific methods for unix.
pub mod unix;
/// Platform-specific methods for Windows.
pub mod windows;

use std::os::raw;

/// Platform-specific extensions for OpenGL [`Context`]s.
///
/// [`Context`]: ../struct.Context.html
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
