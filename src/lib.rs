//! The purpose of this library is to provide an OpenGL [context] on as many
//! platforms as possible, as well as optionnaly a [surface] to go along with it.
//!
//! Before either can be created, however, you need to decide on a [config] for
//! your [context]s and [surface]s.
//!
//! # Basic Usage
//! ```rust,ignore
//! // First you must find a set of configerations that match your criteria.
//! //
//! // You need any type that implements glutin_interface's `NativeDisplay`
//! // trait, e.g. winit's `EventLoopWindowTarget`.
//! use glutin::config::ConfigsFinder;
//! let nd = /* ... */;
//! let confs = unsafe { ConfigsFinder::new().find(&nd).unwrap() };
//! //                                                    ^
//! //                               Notice this unwrap? -/
//! // If we don't find any configerations, glutin will provide a list of reasons
//! // for why each config was excluded. Maybe retry with some more lax settings?
//!
//! // You then need to choose which one of the configs you want. If you don't
//! // care, generally the first one is fine.
//! let conf = &confs[0];
//!
//! // Then you need to make a Context or Surface, the order doesn't particularly
//! // matter.
//! //
//! // Lets make a context first.
//! use glutin::context::ContextBuilder;
//! let ctx = unsafe { ContextBuilder::new().build(conf).unwrap() };
//!
//! // Now lets make a `Surface<Window>`.
//! //
//! // You can only do this if your `NativeDisplay` type also implements
//! // glutin_interface's `NativeWindowSource` trait.
//! //
//! // You also need to pass the `NativeWindowSource`'s `WindowBuilder` type.
//! let wb = /* ... */;
//! use glutin::surface::Surface;
//! let (win, surf) = unsafe { Surface::new_window(conf, &nd, wb).unwrap() };
//! //    ^    ^                                          ^
//! //    |    \- Surface<Window>                         |
//! //    \- `NativeWindowSource::Window`.     Make sure this is the same
//! //                                               nd as before!
//!
//! // Now, just make everything current!
//! unsafe { ctx.make_current(&surf).unwrap() }
//! //        ^                 ^
//! //        \--------\/-------/
//! //                 |
//! // These two thing's configs need to be compatible. Compatibility is a
//! // highly platform-dependent type of thing.
//! //
//! // Safest just to keep their two configs the same.
//!
//! // ...
//! // Do your OpenGL magic here!
//! // ...
//!
//! // What if you made a second window?
//! let win2 = /* ... */;
//!
//! // If this window implements `NativeWindow` and is compatible with your
//! // `NativeDisplay`, you can make a surface out of it.
//! let surf2 = unsafe { Surface::new_from_existing_window(conf, &win2).unwrap() };
//! //    ^                                                        ^
//! //    \- Surface<Window> too                                   |
//! //                              You can't use `win` here, however as it currently
//! //                                 in use by `surf`. Only one surface can use a
//! //                                               window at a time.
//!
//! // You can make it current too, if its config is compatible with the
//! // context's, which it is, since you made it with the same config.
//! unsafe { ctx.make_current(&surf2).unwrap() }
//!
//! // You can also read from one surface and write to a different one.
//! //
//! // Just make sure all three configs are compatible.
//! unsafe { ctx.make_current_rw(&surf, &surf2).unwrap() }
//!
//! // Don't forget to drop your surfaces before your windows.
//! //
//! // You can drop your configs at any time, as long as the `NativeDisplay` is
//! // still alive.
//! //
//! // Be sure to drop the `NativeDisplay` last.
//! //
//! // Failing to do so will result in a segmentation fault if you are lucky!
//! ```
//!
//! # How about Pixmaps?
//!
//! ```rust,ignore
//! // Use your `NativeDisplay`-implementing type as usual, for example winit's
//! // `EventLoopWindowTarget`.
//! let nd = /* ... */;
//!
//! // You need to find a configuration like usual.
//! use glutin::config::ConfigsFinder;
//! let confs = unsafe {
//!     ConfigsFinder::new()
//!         .find(&nd)
//!         // You need to tell us in advanced that you want to support pixmaps.
//!         .with_must_support_pixmaps(true)
//!         // If you don't want to support windows, consider not requesting it.
//!         // Doing so will give you more options.
//!         .with_must_support_windows(false)
//!         .unwrap()
//! };
//!
//! // Proceed as usual
//! let conf = &confs[0];
//! let ctx = /* ... */;
//!
//! // Unfortunately, you probably have to write your own types implementing
//! // `NativePixmapSource` and `NativePixmap`, as winit currently doesn't
//! // suppport pixmaps.
//! //
//! // Luckily, this is pretty easy, being only a small handfull of lines. Please
//! // refer to our pixmap example on github at `examples/pixmap.rs`.
//! use glutin::surface::Surface;
//! let your_custom_nd = /* ... */;
//! let your_custom_wb = /* ... */;
//! let (pix, surf) = unsafe {
//!     Surface::new_pixmap(conf, &your_custom_nd, &your_custom_wb).unwrap()
//! };
//! //    ^    ^
//! //    |    \- Surface<Pixmap>
//! //    \- `NativePixmapSource::Window`.
//!
//! // Like windows, you can also use new_from_existing_pixmap to make a
//! // Surface<Pixmap> from a prexisting pixmap.
//!
//! // Proceed as usual
//! unsafe { ctx.make_current(&surf).unwrap() }
//!
//! // ...
//! // Do your OpenGL magic here!
//! // ...
//!
//! // Remember to drop your surface before your pixmap!
//! ```
//!
//! # How about PBuffers?
//!
//! ```rust,ignore
//! // Use your `NativeDisplay`-implementing type as usual, for example winit's
//! // `EventLoopWindowTarget`.
//! let nd = /* ... */;
//!
//! // You need to find a configuration like usual.
//! use glutin::config::ConfigsFinder;
//! let confs = unsafe {
//!     ConfigsFinder::new()
//!         .find(&nd)
//!         // You need to tell us in advanced that you want to support pbuffers.
//!         .with_must_support_pbuffers(true)
//!         // If you don't want to support windows, consider not requesting it.
//!         // Doing so will give you more options.
//!         .with_must_support_windows(false)
//!         .unwrap()
//! };
//!
//! // Proceed as usual
//! let conf = &confs[0];
//! let ctx = /* ... */;
//!
//! // PBuffers don't need the native APIs, instead being allocated by the GL
//! // driver.
//! //
//! // We do need to know the size, however.
//! let size = winit_types::dpi::PhysicalSize::new(256, 256);
//! //                                              ^
//! //                                              |
//! //        Some drivers only support powers of two! You have been warned!
//!
//! // Sometimes drivers don't have enough memory. If you set `largest` to true
//! // and the driver doesn't have enough space then glutin will try to give
//! // you the largest PBuffer it can provide.
//! //
//! // The drivers will preserve the aspect ratio of your size.
//! //
//! // Lets set `largest` to false, since we want to panic if the driver can't
//! // meet our demands.
//! let largest = false;
//!
//! use glutin::surface::Surface;
//! let surf = unsafe { Surface::new_pbuffer(conf, size, largest).unwrap() };
//! //   ^
//! //   \- Surface<PBuffer>
//!
//! // Proceed as usual
//! unsafe { ctx.make_current(&surf).unwrap() }
//!
//! // ...
//! // Do your OpenGL magic here!
//! // ...
//!
//! // Remember to drop your surface before your pixmap!
//!
//! ```
//!
//! # How about EGL Surfaceless?
//!
//! ```rust,ignore
//! // Use your `NativeDisplay`-implementing type as usual, for example winit's
//! // `EventLoopWindowTarget`.
//! let nd = /* ... */;
//!
//! // You need to find a configuration like usual.
//! use glutin::config::ConfigsFinder;
//! let confs = unsafe {
//!     ConfigsFinder::new()
//!         .find(&nd)
//!         // Surfaceless is an all or nothing type of thing- either all your
//!         // configs support it, or none of them do.
//!         .with_must_support_surfaceless(true)
//!         .unwrap()
//! };
//!
//! // Proceed as usual
//! let conf = &confs[0];
//! let ctx = /* ... */;
//!
//! // And as promiced, you don't need a surface!
//! unsafe { ctx.make_current_surfaceless().unwrap() }
//!
//! // ...
//! // Do your OpenGL magic here!
//! // ...
//!
//! // Remember: `NativeDisplay` drops last!
//!
//! ```
//!
//! # A high-level overview
//! ```text
//! +-| glutin |---------------------------------------------------------------------+
//! | +-| context |----------------+ +-| surface |----------------------------------+|
//! | | +-----------+             | | +-----------+                                 ||
//! | | | `Context` |             | | | `Surface` |                                 ||
//! | | +-----------+             | | +-----------+                                 ||
//! | |      ^                    | |    ^   ^                                      ||
//! | |      |                    | |    |   |                                      ||
//! | | +----+----+               | |    |   |    +-------------------+             ||
//! | | | Creates |               | |    |   \----+    Specializes    |             ||
//! | | +---------+               | |    |        +-------------------+             ||
//! | |       ^                   | |    |          ^         ^   ^                 ||
//! | |       |                   | |    |          |         |   \------\          ||
//! | | +-----+------------+      | |    |  +-------+---+ +---+------+ +-+--------+ ||
//! | | | `ContextBuilder` |      | |    |  | `PBuffer` | | `Pixmap` | | `Window` | ||
//! | | +------------------+      | |    |  +-----------+ +----------+ +----------+ ||
//! | |           ^               | |    |                    ^          ^          ||
//! | +---------< | >-------------+ |    |                    |          |          ||
//! |             |                 +--< | >----------------< | >------< | >--------+|
//! | +-----------+--------------+       |                    |          |           |
//! | | Needed when creating     +-------/                    |          |           |
//! | | the Context and Surface. |                            |          |           |
//! | +--------------------------+                            |          |           |
//! |        ^                                                |          |           |
//! |        |                                                |          |           |
//! | +----< | >-----| config |-----+                         |          |           |
//! | |      |                      |                         |          |           |
//! | | +----+-----+                |           /-------------/          |           |
//! | | | `Config` |<-----\         |           |                        |           |
//! | | +----------+      |         |           |                        |           |
//! | |               +---+---+     |           |                        |           |
//! | |               | Finds |     |           |                        |           |
//! | |               +-------+     |           |                        |           |
//! | |                   ^         |           |                        |           |
//! | |                   |         |           |                        |           |
//! | |         +---------+-------+ |           |                        |           |
//! | |         | `ConfigsFinder` | |           |                        |           |
//! | |         +-----------------+ |           |                        |           |
//! | |                ^            |           |                        |           |
//! | +--------------< | >----------+           |                        |           |
//! +----------------< | >--------------------< | >--------------------< | >---------+
//!                    |                        |                        |
//! +------------------+------+ +---------------+----------+ +-----------+--------------+
//! | Needs type implementing | | Needs types implementing | | Needs types implementing |
//! | trait.                  | | both traits.             | | both traits.             |
//! +-------------------------+ +--------------------------+ +--------------------------+
//!     ^                                 ^                           ^
//! +-< | >--| glutin_interface |-------< | >-----------------------< | >----------+
//! |   |                                 |                           |            |
//! |   |                        /--------/       /------------------/ \------\    |
//! |   |                        |                |                           |    |
//! | +-+-----------------------+ | +--------------+----------+ +-------------+--+ |
//! | | `NativeDisplay`         | | | `NativeWindowSource`    | | `NativeWindow` | |
//! | | e.g. gbm-rs's `Device`  | | | e.g. gbm-rs's           | | e.g. winit's   | |
//! | | type or winit's         | | | `DeviceGlutinWrapper`   | | `Window` type  | |
//! | | `EventLoopWindowTarget` | | | type or winit's         | | or gbm-rs's    | |
//! | | type.                   | | | `EventLoopWindowTarget` | | `Surface` type | |
//! | |                         | | | type.                   | |                | |
//! | +-------------------------+ | +-------------------------+ +----------------+ |
//! |                 /----------/ \--\                                            |
//! |                 |               |                                            |
//! |     +-----------+----------+ +--+-------------+                              |
//! |     | `NativePixmapSource` | | `NativePixmap` |                              |
//! |     +----------------------+ +----------------+                              |
//! +------------------------------------------------------------------------------+
//! ```
//!
//! [context]: crate::context
//! [surface]: crate::surface
//! [config]: crate::config

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
pub mod config;
pub mod context;
mod platform_impl;
pub mod surface;
mod utils;
