//! The purpose of this library is to provide an OpenGL [`Context`] on as many
//! platforms as possible.
//!
//! # Building a [`WindowedContext<T>`]
//!
//! A [`WindowedContext<T>`] is composed of a [`Window`] and an OpenGL
//! [`Context`].
//!
//! Due to some operating-system-specific quirks, glutin prefers control over
//! the order of creation of the [`Context`] and [`Window`]. Here is an example
//! of building a [`WindowedContext<T>`]:
// //! ```no_run
// //! # fn main() {
// //! let el = glutin::event_loop::EventLoop::new();
// //! let wb = glutin::window::WindowBuilder::new()
// //!     .with_title("Hello world!")
// //!     .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0));
// //! let windowed_context = glutin::ContextBuilder::new()
// //!     .build_windowed(wb, &el)
// //!     .unwrap();
// //! # }
// //! ```
// FIXME update
//! You can, of course, create a [`RawContext<T>`] separately from an existing
//! window, however that may result in an suboptimal configuration of the window
//! on some platforms. In that case use the unsafe platform-specific
//! [`RawContextExt`] available on unix operating systems and Windows.
//!
//! You can also produce headless [`Context`]s via the
//! [`ContextBuilder::build_headless`] function.
//!
//! [`Window`]: struct.Window.html
//! [`Context`]: struct.Context.html
//! [`WindowedContext<T>`]: type.WindowedContext.html
//! [`RawContext<T>`]: type.RawContext.html
#![cfg_attr(
    target_os = "windows",
    doc = "\
[`RawContextExt`]: os/windows/trait.RawContextExt.html
"
)]
#![cfg_attr(
    not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "windows",
        target_os = "openbsd",
    )),
    doc = "\
[`RawContextExt`]: os/index.html
"
)]
#![cfg_attr(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ),
    doc = "\
[`RawContextExt`]: os/unix/trait.RawContextExt.html
"
)]
#![deny(
    missing_debug_implementations,
    //missing_docs,
)]

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate lazy_static;
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[macro_use]
extern crate objc;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate log;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate bitflags;

pub mod platform;

mod api;
mod config;
mod context;
mod display;
mod platform_impl;
mod surface;

pub use winit::*;

use crate::context::Context;

use winit::error::OsError;

use std::default::Default;
use std::io;

/// Error that can happen while creating a window or a headless renderer.
#[derive(Debug)]
pub enum CreationError {
    OsError(String),
    NotSupported(String),
    NoBackendAvailable(Box<dyn std::error::Error + Send + Sync>),
    RobustnessNotSupported,
    OpenGlVersionNotSupported,
    NoAvailableConfig,
    PlatformSpecific(String),
    Window(OsError),
    /// We received multiple errors, instead of one.
    CreationErrors(Vec<Box<CreationError>>),
}

impl CreationError {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    pub(crate) fn append(self, err: CreationError) -> Self {
        match self {
            CreationError::CreationErrors(mut errs) => {
                errs.push(Box::new(err));
                CreationError::CreationErrors(errs)
            }
            _ => CreationError::CreationErrors(vec![
                Box::new(err),
                Box::new(self),
            ]),
        }
    }

    fn to_string(&self) -> &str {
        match *self {
            CreationError::OsError(ref text)
            | CreationError::NotSupported(ref text) => &text,
            CreationError::NoBackendAvailable(_) => "No backend is available",
            CreationError::RobustnessNotSupported => {
                "You requested robustness, but it is not supported."
            }
            CreationError::OpenGlVersionNotSupported => {
                "The requested OpenGL version is not supported."
            }
            CreationError::NoAvailableConfig => {
                "Couldn't find any config that matches the criteria."
            }
            CreationError::PlatformSpecific(ref text) => &text,
            CreationError::Window(ref err) => {
                std::error::Error::description(err)
            }
            CreationError::CreationErrors(_) => "Received multiple errors.",
        }
    }
}

impl std::fmt::Display for CreationError {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> Result<(), std::fmt::Error> {
        formatter.write_str(self.to_string())?;

        if let CreationError::CreationErrors(ref es) = *self {
            use std::fmt::Debug;
            write!(formatter, " Errors: `")?;
            es.fmt(formatter)?;
            write!(formatter, "`")?;
        }

        if let Some(err) = std::error::Error::source(self) {
            write!(formatter, ": {}", err)?;
        }
        Ok(())
    }
}

impl std::error::Error for CreationError {
    fn description(&self) -> &str {
        self.to_string()
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            CreationError::NoBackendAvailable(ref err) => Some(&**err),
            CreationError::Window(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<OsError> for CreationError {
    fn from(err: OsError) -> Self {
        CreationError::Window(err)
    }
}

// Rectangles to submit as buffer damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
