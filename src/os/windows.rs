#![cfg(target_os = "windows")]

pub use winapi::HDC;
pub use winit::os::windows::{WindowBuilderExt, WindowExt, MonitorIdExt};

pub use api::egl::ffi::egl::types::EGLContext;

/// Context types available on Windows.
#[derive(Clone, Debug)]
pub enum Context {
    Egl(EGLContext),
    Wgl(HDC),
}
