#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#![allow(
    clippy::manual_non_exhaustive,
    clippy::missing_safety_doc,
    clippy::redundant_static_lifetimes,
    clippy::unused_unit
)]
#![cfg_attr(feature = "cargo-clippy", deny(warnings))]

pub use self::glx::types::GLXContext;
pub use x11_dl::xlib::*;

/// GLX bindings
pub mod glx {
    include!(concat!(env!("OUT_DIR"), "/glx_bindings.rs"));
}

/// Functions that are not necessarily always available
pub mod glx_extra {
    include!(concat!(env!("OUT_DIR"), "/glx_extra_bindings.rs"));
}
