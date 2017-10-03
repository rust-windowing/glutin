#![cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]

pub use winit::os::unix::XNotSupported;
pub use winit::os::unix::EventsLoopExt;
pub use winit::os::unix::MonitorIdExt;
pub use winit::os::unix::WindowBuilderExt;
pub use winit::os::unix::WindowExt;

pub use api::egl::ffi::egl::types::EGLContext;
pub use api::glx::ffi::GLXContext;
pub use api::osmesa::ffi::OSMesaContext;

/// Context types available on Unix-like platforms.
#[derive(Clone, Debug)]
pub enum Context {
    Glx(GLXContext),
    Egl(EGLContext),
}
