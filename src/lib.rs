//! The purpose of this library is to provide an OpenGL context on as many
//!  platforms as possible.
//!
//! # Building a window
//!
//! There are two ways to create a window:
//!
//!  - Calling `Window::new()`.
//!  - Calling `let builder = WindowBuilder::new()` then `builder.build()`.
//!
//! The first way is the simpliest way and will give you default values.
//!
//! The second way allows you to customize the way your window and GL context
//!  will look and behave.
//!
//! # Features
//!
//! This crate has two Cargo features: `window` and `headless`.
//!
//!  - `window` allows you to create regular windows and enables the `WindowBuilder` object.
//!  - `headless` allows you to do headless rendering, and enables
//!     the `HeadlessRendererBuilder` object.
//!
//! By default only `window` is enabled.

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate shared_library;

extern crate libc;

extern crate winit;

#[cfg(target_os = "windows")]
extern crate winapi;
#[cfg(target_os = "windows")]
extern crate kernel32;
#[cfg(target_os = "windows")]
extern crate shell32;
#[cfg(target_os = "windows")]
extern crate gdi32;
#[cfg(target_os = "windows")]
extern crate user32;
#[cfg(target_os = "windows")]
extern crate dwmapi;
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[macro_use]
extern crate objc;
#[cfg(target_os = "macos")]
extern crate cgl;
#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_graphics;
#[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]
extern crate x11_dl;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "dragonfly", target_os = "openbsd"))]
#[macro_use(wayland_env)]
extern crate wayland_client;

pub use events::*;
pub use headless::{HeadlessRendererBuilder, HeadlessContext};
pub use window::{AvailableMonitorsIter, MonitorId, WindowId, get_available_monitors, get_primary_monitor};
pub use winit::NativeMonitorId;

use std::io;

mod api;
mod platform;
mod events;
mod headless;
mod window;

pub mod os;

/// Represents an OpenGL context and the Window or environment around it.
///
/// # Example
///
/// ```ignore
/// let window = Window::new(&events_loop).unwrap();
///
/// unsafe { window.make_current() };
///
/// loop {
///     events_loop.poll_events(|event| {
///         match(event) {
///             // process events here
///             _ => ()
///         }
///     });
///
///     // draw everything here
///
///     window.swap_buffers();
///     std::thread::sleep(std::time::Duration::from_millis(17));
/// }
/// ```
pub struct Window {
    window: platform::Window,
}

/// Object that allows you to build windows.
#[derive(Clone)]
pub struct WindowBuilder<'a> {
    winit_builder: winit::WindowBuilder,

    /// The attributes to use to create the context.
    pub opengl: GlAttributes<&'a platform::Window>,

    // Should be made public once it's stabilized.
    pf_reqs: PixelFormatRequirements,
}

/// Provides a way to retreive events from the windows that are registered to it.
// TODO: document usage in multiple threads
pub struct EventsLoop {
    events_loop: platform::EventsLoop,
}

impl EventsLoop {
    /// Builds a new events loop.
    pub fn new() -> EventsLoop {
        EventsLoop {
            events_loop: platform::EventsLoop::new(),
        }
    }

    /// Fetches all the events that are pending, calls the callback function for each of them,
    /// and returns.
    #[inline]
    pub fn poll_events<F>(&self, callback: F)
        where F: FnMut(Event)
    {
        self.events_loop.poll_events(callback)
    }

    /// Runs forever until `interrupt()` is called. Whenever an event happens, calls the callback.
    #[inline]
    pub fn run_forever<F>(&self, callback: F)
        where F: FnMut(Event)
    {
        self.events_loop.run_forever(callback)
    }

    /// If we called `run_forever()`, stops the process of waiting for events.
    #[inline]
    pub fn interrupt(&self) {
        self.events_loop.interrupt()
    }
}

/// Trait that describes objects that have access to an OpenGL context.
pub trait GlContext {
    /// Sets the context as the current context.
    unsafe fn make_current(&self) -> Result<(), ContextError>;

    /// Returns true if this context is the current one in this thread.
    fn is_current(&self) -> bool;

    /// Returns the address of an OpenGL function.
    fn get_proc_address(&self, addr: &str) -> *const ();

    /// Swaps the buffers in case of double or triple buffering.
    ///
    /// You should call this function every time you have finished rendering, or the image
    /// may not be displayed on the screen.
    ///
    /// **Warning**: if you enabled vsync, this function will block until the next time the screen
    /// is refreshed. However drivers can choose to override your vsync settings, which means that
    /// you can't know in advance whether `swap_buffers` will block or not.
    fn swap_buffers(&self) -> Result<(), ContextError>;

    /// Returns the OpenGL API being used.
    fn get_api(&self) -> Api;

    /// Returns the pixel format of the main framebuffer of the context.
    fn get_pixel_format(&self) -> PixelFormat;
}

/// Error that can happen while creating a window or a headless renderer.
#[derive(Debug)]
pub enum CreationError {
    OsError(String),
    /// TODO: remove this error
    NotSupported,
    NoBackendAvailable(Box<std::error::Error + Send>),
    RobustnessNotSupported,
    OpenGlVersionNotSupported,
    NoAvailablePixelFormat,
}

impl CreationError {
    fn to_string(&self) -> &str {
        match *self {
            CreationError::OsError(ref text) => &text,
            CreationError::NotSupported => "Some of the requested attributes are not supported",
            CreationError::NoBackendAvailable(_) => "No backend is available",
            CreationError::RobustnessNotSupported => "You requested robustness, but it is \
                                                      not supported.",
            CreationError::OpenGlVersionNotSupported => "The requested OpenGL version is not \
                                                         supported.",
            CreationError::NoAvailablePixelFormat => "Couldn't find any pixel format that matches \
                                                      the criterias.",
        }
    }
}

impl std::fmt::Display for CreationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        formatter.write_str(self.to_string())?;
        if let Some(err) = std::error::Error::cause(self) {
            write!(formatter, ": {}", err)?;
        }
        Ok(())
    }
}

impl std::error::Error for CreationError {
    fn description(&self) -> &str {
        self.to_string()
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            CreationError::NoBackendAvailable(ref err) => Some(&**err),
            _ => None
        }
    }
}

/// Error that can happen when manipulating an OpenGL context.
#[derive(Debug)]
pub enum ContextError {
    IoError(io::Error),
    ContextLost,
}

impl ContextError {
    fn to_string(&self) -> &str {
        use std::error::Error;
        match *self {
            ContextError::IoError(ref err) => err.description(),
            ContextError::ContextLost => "Context lost"
        }
    }
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        formatter.write_str(self.to_string())
    }
}

impl std::error::Error for ContextError {
    fn description(&self) -> &str {
        self.to_string()
    }
}

/// All APIs related to OpenGL that you can possibly get while using glutin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    /// The classical OpenGL. Available on Windows, Linux, OS/X.
    OpenGl,
    /// OpenGL embedded system. Available on Linux, Android.
    OpenGlEs,
    /// OpenGL for the web. Very similar to OpenGL ES.
    WebGl,
}

/// Describes the requested OpenGL context profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlProfile {
    /// Include all the immediate more functions and definitions.
    Compatibility,
    /// Include all the future-compatible functions and definitions.
    Core,
}

/// Describes the OpenGL API and version that are being requested when a context is created.
#[derive(Debug, Copy, Clone)]
pub enum GlRequest {
    /// Request the latest version of the "best" API of this platform.
    ///
    /// On desktop, will try OpenGL.
    Latest,

    /// Request a specific version of a specific API.
    ///
    /// Example: `GlRequest::Specific(Api::OpenGl, (3, 3))`.
    Specific(Api, (u8, u8)),

    /// If OpenGL is available, create an OpenGL context with the specified `opengl_version`.
    /// Else if OpenGL ES or WebGL is available, create a context with the
    /// specified `opengles_version`.
    GlThenGles {
        /// The version to use for OpenGL.
        opengl_version: (u8, u8),
        /// The version to use for OpenGL ES.
        opengles_version: (u8, u8),
    },
}

impl GlRequest {
    /// Extract the desktop GL version, if any.
    pub fn to_gl_version(&self) -> Option<(u8, u8)> {
        match self {
            &GlRequest::Specific(Api::OpenGl, version) => Some(version),
            &GlRequest::GlThenGles { opengl_version: version, .. } => Some(version),
            _ => None,
        }
    }
}

/// The minimum core profile GL context. Useful for getting the minimum
/// required GL version while still running on OSX, which often forbids
/// the compatibility profile features.
pub static GL_CORE: GlRequest = GlRequest::Specific(Api::OpenGl, (3, 2));

/// Specifies the tolerance of the OpenGL context to faults. If you accept raw OpenGL commands
/// and/or raw shader code from an untrusted source, you should definitely care about this.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Robustness {
    /// Not everything is checked. Your application can crash if you do something wrong with your
    /// shaders.
    NotRobust,

    /// The driver doesn't check anything. This option is very dangerous. Please know what you're
    /// doing before using it. See the `GL_KHR_no_error` extension.
    ///
    /// Since this option is purely an optimisation, no error will be returned if the backend
    /// doesn't support it. Instead it will automatically fall back to `NotRobust`.
    NoError,

    /// Everything is checked to avoid any crash. The driver will attempt to avoid any problem,
    /// but if a problem occurs the behavior is implementation-defined. You are just guaranteed not
    /// to get a crash.
    RobustNoResetNotification,

    /// Same as `RobustNoResetNotification` but the context creation doesn't fail if it's not
    /// supported.
    TryRobustNoResetNotification,

    /// Everything is checked to avoid any crash. If a problem occurs, the context will enter a
    /// "context lost" state. It must then be recreated. For the moment, glutin doesn't provide a
    /// way to recreate a context with the same window :-/
    RobustLoseContextOnReset,

    /// Same as `RobustLoseContextOnReset` but the context creation doesn't fail if it's not
    /// supported.
    TryRobustLoseContextOnReset,
}

/// The behavior of the driver when you change the current context.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReleaseBehavior {
    /// Doesn't do anything. Most notably doesn't flush.
    None,

    /// Flushes the context that was previously current as if `glFlush` was called.
    Flush,
}

pub use winit::MouseCursor;

pub use winit::CursorState;

/// Describes a possible format. Unused.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct PixelFormat {
    pub hardware_accelerated: bool,
    pub color_bits: u8,
    pub alpha_bits: u8,
    pub depth_bits: u8,
    pub stencil_bits: u8,
    pub stereoscopy: bool,
    pub double_buffer: bool,
    pub multisampling: Option<u16>,
    pub srgb: bool,
}

/// Describes how the backend should choose a pixel format.
// TODO: swap method? (swap, copy)
#[derive(Clone, Debug)]
pub struct PixelFormatRequirements {
    /// If true, only hardware-accelerated formats will be conisdered. If false, only software
    /// renderers. `None` means "don't care". Default is `Some(true)`.
    pub hardware_accelerated: Option<bool>,

    /// Minimum number of bits for the color buffer, excluding alpha. `None` means "don't care".
    /// The default is `Some(24)`.
    pub color_bits: Option<u8>,

    /// If true, the color buffer must be in a floating point format. Default is `false`.
    ///
    /// Using floating points allows you to write values outside of the `[0.0, 1.0]` range.
    pub float_color_buffer: bool,

    /// Minimum number of bits for the alpha in the color buffer. `None` means "don't care".
    /// The default is `Some(8)`.
    pub alpha_bits: Option<u8>,

    /// Minimum number of bits for the depth buffer. `None` means "don't care".
    /// The default value is `Some(24)`.
    pub depth_bits: Option<u8>,

    /// Minimum number of bits for the depth buffer. `None` means "don't care".
    /// The default value is `Some(8)`.
    pub stencil_bits: Option<u8>,

    /// If true, only double-buffered formats will be considered. If false, only single-buffer
    /// formats. `None` means "don't care". The default is `Some(true)`.
    pub double_buffer: Option<bool>,

    /// Contains the minimum number of samples per pixel in the color, depth and stencil buffers.
    /// `None` means "don't care". Default is `None`.
    /// A value of `Some(0)` indicates that multisampling must not be enabled.
    pub multisampling: Option<u16>,

    /// If true, only stereoscopic formats will be considered. If false, only non-stereoscopic
    /// formats. The default is `false`.
    pub stereoscopy: bool,

    /// If true, only sRGB-capable formats will be considered. If false, don't care.
    /// The default is `false`.
    pub srgb: bool,

    /// The behavior when changing the current context. Default is `Flush`.
    pub release_behavior: ReleaseBehavior,
}

impl Default for PixelFormatRequirements {
    #[inline]
    fn default() -> PixelFormatRequirements {
        PixelFormatRequirements {
            hardware_accelerated: Some(true),
            color_bits: Some(24),
            float_color_buffer: false,
            alpha_bits: Some(8),
            depth_bits: Some(24),
            stencil_bits: Some(8),
            double_buffer: None,
            multisampling: None,
            stereoscopy: false,
            srgb: false,
            release_behavior: ReleaseBehavior::Flush,
        }
    }
}

pub use winit::WindowAttributes; // TODO

/// Attributes to use when creating an OpenGL context.
#[derive(Clone)]
pub struct GlAttributes<S> {
    /// An existing context to share the new the context with.
    ///
    /// The default is `None`.
    pub sharing: Option<S>,

    /// Version to try create. See `GlRequest` for more infos.
    ///
    /// The default is `Latest`.
    pub version: GlRequest,

    /// OpenGL profile to use.
    ///
    /// The default is `None`.
    pub profile: Option<GlProfile>,

    /// Whether to enable the `debug` flag of the context.
    ///
    /// Debug contexts are usually slower but give better error reporting.
    ///
    /// The default is `true` in debug mode and `false` in release mode.
    pub debug: bool,

    /// How the OpenGL context should detect errors.
    ///
    /// The default is `NotRobust` because this is what is typically expected when you create an
    /// OpenGL context. However for safety you should consider `TryRobustLoseContextOnReset`.
    pub robustness: Robustness,

    /// Whether to use vsync. If vsync is enabled, calling `swap_buffers` will block until the
    /// screen refreshes. This is typically used to prevent screen tearing.
    ///
    /// The default is `false`.
    pub vsync: bool,
}

impl<S> GlAttributes<S> {
    /// Turns the `sharing` parameter into another type by calling a closure.
    #[inline]
    pub fn map_sharing<F, T>(self, f: F) -> GlAttributes<T> where F: FnOnce(S) -> T {
        GlAttributes {
            sharing: self.sharing.map(f),
            version: self.version,
            profile: self.profile,
            debug: self.debug,
            robustness: self.robustness,
            vsync: self.vsync,
        }
    }
}

impl<S> Default for GlAttributes<S> {
    #[inline]
    fn default() -> GlAttributes<S> {
        GlAttributes {
            sharing: None,
            version: GlRequest::Latest,
            profile: None,
            debug: cfg!(debug_assertions),
            robustness: Robustness::NotRobust,
            vsync: false,
        }
    }
}

