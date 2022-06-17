//! The purpose of this library is to provide OpenGL `[context]` on as many
//! platforms as possible abstracting away the underlying platforms shenanigans,
//! while keeping direct access to them to make the use of platform specific
//! extensions easier.
//!
//! However the library doesn't force into using a cross platform abstraction,
//! for example, when only `[EGL]` is desired, it can be used directly.
//!
//! The initialization starts by loading and connecting to the underlying
//! graphics platform Api when creating a `[display]`. This object is used to
//! create all the OpenGL objects, such as `[config]`, `[context]`, and
//! `[surface]`.
//!
//! [display]: crate::display
//! [context]: crate::context
//! [surface]: crate::surface
//! [config]: crate::config
//! [EGL]: crate::api::egl

#![deny(rust_2018_idioms)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::all)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", deny(warnings))]

#[cfg(all(not(egl_backend), not(glx_backend), not(wgl_backend), not(cgl_backend)))]
compile_error!("Please select at least one api backend");

pub mod api;
pub mod config;
pub mod context;
pub mod display;
pub mod error;
pub mod platform;
pub mod prelude;
pub mod surface;

#[cfg(any(egl_backend, glx_backend))]
mod lib_loading;

#[cfg(cgl_backend)]
#[macro_use]
extern crate objc;

pub(crate) mod private {
    /// Prevent traits from being implemented downstream, since those are used
    /// purely for documentation organization and simplify platform api
    /// implementation maintaining.
    pub trait Sealed {}

    /// `gl_api_dispatch!(match expr; Enum(foo) => foo.something())`
    /// expands to the equivalent of
    /// ```ignore
    /// match self {
    ///    Enum::Egl(foo) => foo.something(),
    ///    Enum::Glx(foo) => foo.something(),
    ///    Enum::Wgl(foo) => foo.something(),
    ///    Enum::Cgl(foo) => foo.something(),
    /// }
    /// ```
    /// The result can be converted to another enum by adding `; as AnotherEnum`
    macro_rules! gl_api_dispatch {
        ($what:ident; $enum:ident ( $($c1:tt)* ) => $x:expr; as $enum2:ident ) => {
            match $what {
                #[cfg(egl_backend)]
                $enum::Egl($($c1)*) => $enum2::Egl($x),
                #[cfg(glx_backend)]
                $enum::Glx($($c1)*) => $enum2::Glx($x),
                #[cfg(wgl_backend)]
                $enum::Wgl($($c1)*) => $enum2::Wgl($x),
                #[cfg(cgl_backend)]
                $enum::Cgl($($c1)*) => $enum2::Cgl($x),
            }
        };
        ($what:ident; $enum:ident ( $($c1:tt)* ) => $x:expr) => {
            match $what {
                #[cfg(egl_backend)]
                $enum::Egl($($c1)*) => $x,
                #[cfg(glx_backend)]
                $enum::Glx($($c1)*) => $x,
                #[cfg(wgl_backend)]
                $enum::Wgl($($c1)*) => $x,
                #[cfg(cgl_backend)]
                $enum::Cgl($($c1)*) => $x,
            }
        };
    }

    pub(crate) use gl_api_dispatch;
}
