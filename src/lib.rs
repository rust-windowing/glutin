//! The purpose of this library is to provide an OpenGL [context] on as many
//! platforms as possible, as well as a [surface] to go along with it. Before
//! you can do that, however, you need to decide on a [config] for your
//! [context]s and [surface]s.
//!
//! ```text
//! +-| glutin |--------------------------------------------------------------+
//! | +-| context |--------------+ +-| surface |-----------------------------+|
//! | |    +---------+           | | +---------+                             ||
//! | |    | Context |           | | | Surface |                             ||
//! | |    +---------+           | | +---------+                             ||
//! | |       ^                  | |    ^   ^                                ||
//! | |       | +---------+      | |    |   |    +-------------------+       ||
//! | |       *-+ Creates |      | |    |   \----+    Specializes    |       ||
//! | |       | +---------+      | |    |        +-------------------+       ||
//! | |       |                  | |    |          ^       ^        ^        ||
//! | |       |                  | |    |          |       |        |        ||
//! | | +-----+----------+       | |    |  +-------+-+ +---+----+ +-+------+ ||
//! | | | ContextBuilder |<--\   | |    |  | PBuffer | | Pixmap | | Window | ||
//! | | +----------------+   |   | |    |  +---------+ +--------+ +--------+ ||
//! | +--------------------< | >-+ |    |                  ^          ^      ||
//! |                        |     +--< | >--------------< | >------< | >----+|
//! |                        |          |                  |          |       |
//! |        /--------------/-----------/                  |          |       |
//! |        |                                             |          |       |
//! |        | +--------------------------+                |          |       |
//! |        *-+ Needed when creating     |                |          |       |
//! |        | | the Context and Surface. |                |          |       |
//! |        | +--------------------------+                |          |       |
//! |        |                                             |          |       |
//! | +----< | >-----| config |-----+                      |          |       |
//! | |      |                      |                      |          |       |
//! | | +----+-----+                |           /----------/          |       |
//! | | |  Config  |<-----\         |           |                     |       |
//! | | +----------+      |         |           |                     |       |
//! | |               +---+---+     |           |                     |       |
//! | |               | Finds |     |           |                     |       |
//! | |               +-------+     |           |                     |       |
//! | |                   ^         |           |                     |       |
//! | |                   |         |           |                     |       |
//! | |           +-------+-------+ |           |                     |       |
//! | |           | ConfigsFinder | |           |                     |       |
//! | |           +---------------+ |           |                     |       |
//! | |                ^            |           |                     |       |
//! | +--------------< | >----------+           |                     |       |
//! +----------------< | >--------------------< | >-----------------< | >-----+
//!                    |                        |                     |
//! +------------------+------+ +---------------+----------+ +--------+-----------------+
//! | Needs type implementing | | Needs types implementing | | Needs types implementing |
//! | trait.                  | | both traits.             | | both traits.             |
//! +-------------------------+ +--------------------------+ +--------------------------+
//!     ^                               ^                              ^
//! +-< | >--| glutin_interface |-----< | >--------------------------< | >-----------------------+
//! |   |                               |                              |                         |
//! |   |                               |       /---------------------/ \-------------\          |
//! |   |                               |       |                                     |          |
//! | +-+-----------------------------+ | +-----+-------------------------+ +---------+--------+ |
//! | | NativeDisplay                 | | | NativeWindowSource            | | NativeWindow     | |
//! | | e.g.:                         | | | e.g.:                         | | e.g.:            | |
//! | | gbm-rs's Device               | | | gbm-rs's DeviceGlutinWrapper  | | winit's Window   | |
//! | | winit's EventLoopWindowTarget | | | winit's EventLoopWindowTarget | | gbm-rs's Surface | |
//! | +-------------------------------+ | +-------------------------------+ +------------------+ |
//! |                      /---------/--/                                                        |
//! |                      |         |                                                           |
//! |       +--------------+-----+ +-+------------+                                              |
//! |       | NativePixmapSource | | NativePixmap |                                              |
//! |       +--------------------+ +--------------+                                              |
//! +--------------------------------------------------------------------------------------------+
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
