#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::manual_non_exhaustive)]
#![allow(clippy::unused_unit)]
#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::unnecessary_cast)]

pub use self::glx::types::GLXContext;
pub use x11_dl::xlib::*;

/// GLX bindings
pub mod glx {
    include!(concat!(env!("OUT_DIR"), "/glx_bindings.rs"));

    // The GLX protocol error codes extracted from <GL/glxproto.h>.
    pub const PROTO_BAD_CONTEXT: types::GLenum = 0;
    pub const PROTO_BAD_CONTEXT_STATE: types::GLenum = 1;
    pub const PROTO_BAD_DRAWABLE: types::GLenum = 2;
    pub const PROTO_BAD_PIXMAP: types::GLenum = 3;
    pub const PROTO_BAD_CONTEXT_TAG: types::GLenum = 4;
    pub const PROTO_BAD_CURRENT_WINDOW: types::GLenum = 5;
    pub const PROTO_BAD_RENDER_REQUEST: types::GLenum = 6;
    pub const PROTO_BAD_LARGE_REQUEST: types::GLenum = 7;
    pub const PROTO_UNSUPPORTED_PRIVATE_REQUEST: types::GLenum = 8;
    pub const PROTO_BAD_FBCONFIG: types::GLenum = 9;
    pub const PROTO_BAD_PBUFFER: types::GLenum = 10;
    pub const PROTO_BAD_CURRENT_DRAWABLE: types::GLenum = 11;
    pub const PROTO_BAD_WINDOW: types::GLenum = 12;
    pub const PROTO_BAD_PROFILE_ARB: types::GLenum = 13;
}

/// Functions that are not necessarily always available
pub mod glx_extra {
    include!(concat!(env!("OUT_DIR"), "/glx_extra_bindings.rs"));
}
