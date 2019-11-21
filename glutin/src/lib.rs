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

/// All APIs related to OpenGL that you can possibly get while using glutin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    /// The classical OpenGL. Available on Windows, Unix operating systems,
    /// OS/X.
    OpenGl,
    /// OpenGL embedded system. Available on Unix operating systems, Android.
    OpenGlEs,
    /// OpenGL for the web. Very similar to OpenGL ES.
    WebGl,
}

/// Describes the requested OpenGL [`Context`] profiles.
///
/// [`Context`]: struct.Context.html
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlProfile {
    /// Include all the immediate more functions and definitions.
    Compatibility,
    /// Include all the future-compatible functions and definitions.
    Core,
}

#[derive(Debug, Copy, Clone)]
struct GlVersion(u8, u8);

/// Describes the OpenGL API and version that are being requested when a context
/// is created.
#[derive(Debug, Copy, Clone)]
pub enum GlRequest {
    /// Request the latest version of the "best" API of this platform.
    ///
    /// On desktop, will try OpenGL.
    Latest,

    /// Request a specific version of a specific API.
    ///
    /// Example: `GlRequest::Specific(Api::OpenGl, (3, 3))`.
    Specific(Api, GlVersion),

    /// If OpenGL is available, create an OpenGL [`Context`] with the specified
    /// `opengl_version`. Else if OpenGL ES or WebGL is available, create a
    /// context with the specified `opengles_version`.
    ///
    /// [`Context`]: struct.Context.html
    GlThenGles {
        /// The version to use for OpenGL.
        opengl_version: GlVersion,
        /// The version to use for OpenGL ES.
        opengles_version: GlVersion,
    },
}

/// The minimum core profile GL context. Useful for getting the minimum
/// required GL version while still running on OSX, which often forbids
/// the compatibility profile features.
pub static GL_CORE: GlRequest =
    GlRequest::Specific(Api::OpenGl, GlVersion(3, 2));

/// Specifies the tolerance of the OpenGL [`Context`] to faults. If you accept
/// raw OpenGL commands and/or raw shader code from an untrusted source, you
/// should definitely care about this.
///
/// [`Context`]: struct.Context.html
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Robustness {
    /// Not everything is checked. Your application can crash if you do
    /// something wrong with your shaders.
    NotRobust,

    /// The driver doesn't check anything. This option is very dangerous.
    /// Please know what you're doing before using it. See the
    /// `GL_KHR_no_error` extension.
    ///
    /// Since this option is purely an optimization, no error will be returned
    /// if the backend doesn't support it. Instead it will automatically
    /// fall back to [`NotRobust`].
    ///
    /// [`NotRobust`]: enum.Robustness.html#variant.NotRobust
    NoError,

    /// Everything is checked to avoid any crash. The driver will attempt to
    /// avoid any problem, but if a problem occurs the behavior is
    /// implementation-defined. You are just guaranteed not to get a crash.
    RobustNoResetNotification,

    /// Same as [`RobustNoResetNotification`] but the context creation doesn't
    /// fail if it's not supported.
    ///
    /// [`RobustNoResetNotification`]:
    /// enum.Robustness.html#variant.RobustNoResetNotification
    TryRobustNoResetNotification,

    /// Everything is checked to avoid any crash. If a problem occurs, the
    /// context will enter a "context lost" state. It must then be
    /// recreated. For the moment, glutin doesn't provide a way to recreate
    /// a context with the same window :-/
    RobustLoseContextOnReset,

    /// Same as [`RobustLoseContextOnReset`] but the context creation doesn't
    /// fail if it's not supported.
    ///
    /// [`RobustLoseContextOnReset`]:
    /// enum.Robustness.html#variant.RobustLoseContextOnReset
    TryRobustLoseContextOnReset,
}

/// The behavior of the driver when you change the current context.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReleaseBehavior {
    /// Doesn't do anything. Most notably doesn't flush.
    None,

    /// Flushes the context that was previously current as if `glFlush` was
    /// called.
    Flush,
}

// Rectangles to submit as buffer damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
