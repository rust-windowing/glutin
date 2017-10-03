#![cfg(any(target_os = "android"))]

pub use winit::os::android::{WindowBuilderExt, WindowExt};

pub use api::egl::ffi::egl::types::EGLContext;
