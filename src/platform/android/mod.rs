#![cfg(target_os = "android")]

pub use winit::EventsLoop;

pub use api::android::*;
pub use api::egl::ffi::egl::types::EGLContext;
