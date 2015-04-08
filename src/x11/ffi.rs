pub use x11::keysym::*;
pub use x11::xcursor::*;
pub use x11::xf86vmode::*;
pub use x11::xlib::*;

/// GLX bindings
pub mod glx {
    include!(concat!(env!("OUT_DIR"), "/glx_bindings.rs"));
}
pub use self::glx::types::GLXContext;

/// Functions that are not necessarly always available
pub mod glx_extra {
    include!(concat!(env!("OUT_DIR"), "/glx_extra_bindings.rs"));
}
