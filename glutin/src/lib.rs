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
// FIXME: Remove before 0.23
#![allow(unused_imports)]

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
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate winit_types;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate glutin_x11_sym;

pub mod platform;

mod api;
mod config;
mod context;
mod platform_impl;
mod surface;
mod utils;
