#![cfg(any(target_os = "windows"))]
#![allow(clippy::manual_non_exhaustive, clippy::missing_safety_doc, clippy::too_many_arguments)]
#![cfg_attr(feature = "cargo-clippy", deny(warnings))]

/// WGL bindings
pub mod wgl {
    include!(concat!(env!("OUT_DIR"), "/wgl_bindings.rs"));
}

/// Functions that are not necessarily always available
pub mod wgl_extra {
    include!(concat!(env!("OUT_DIR"), "/wgl_extra_bindings.rs"));
}

#[link(name = "opengl32")]
extern "C" {}
