use crate::config::Api;
use crate::config::Config;
use crate::platform_impl;
use crate::surface::{Surface, SurfaceTypeTrait};

use winit_types::error::Error;

use std::os::raw;

#[derive(Debug)]
pub struct Context(pub(crate) platform_impl::Context);

impl Context {
    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        self.0.make_current_surfaceless()
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        self.0.make_current(&surf.0)
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.0.make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_config(&self) -> Config {
        self.0.get_config()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.0.get_api()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const raw::c_void {
        self.0.get_proc_address(addr)
    }

    #[inline]
    pub fn update_after_resize(&self) {
        #[cfg(target_os = "macos")]
        self.0.update_after_resize()
    }
}

impl<'a> ContextBuilder<'a> {
    #[inline]
    pub fn build(self, conf: &Config) -> Result<Context, Error> {
        let cb = self.map_sharing(|ctx| &ctx.0);
        platform_impl::Context::new(cb, conf.as_ref()).map(Context)
    }
}

/// An object that allows you to build [`Context`]s, [`RawContext<T>`]s and
/// [`WindowedContext<T>`]s.
///
/// One notable limitation of the Wayland backend when it comes to shared
/// [`Context`]s is that both contexts must use the same events loop.
///
/// [`Context`]: struct.Context.html
/// [`WindowedContext<T>`]: type.WindowedContext.html
/// [`RawContext<T>`]: type.RawContext.html
#[derive(Debug, Clone)]
pub struct ContextBuilderWrapper<T> {
    /// An existing context with which some OpenGL objects get shared.
    ///
    /// The default is `None`.
    pub sharing: Option<T>,

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

    /// How the OpenGL [`Context`] should detect errors.
    ///
    /// The default is `NotRobust` because this is what is typically expected
    /// when you create an OpenGL [`Context`]. However for safety you should
    /// consider [`TryRobustLoseContextOnReset`].
    ///
    /// [`Context`]: struct.Context.html
    /// [`TryRobustLoseContextOnReset`]:
    /// enum.Robustness.html#variant.TryRobustLoseContextOnReset
    pub robustness: Robustness,
}

pub type ContextBuilder<'a> = ContextBuilderWrapper<&'a Context>;

impl<T> ContextBuilderWrapper<T> {
    /// Turns the `sharing` parameter into another type by calling a closure.
    #[inline]
    pub(crate) fn map_sharing<F, T2>(self, f: F) -> ContextBuilderWrapper<T2>
    where
        F: FnOnce(T) -> T2,
    {
        ContextBuilderWrapper {
            sharing: self.sharing.map(f),
            profile: self.profile,
            debug: self.debug,
            robustness: self.robustness,
        }
    }

    /// Turns the `sharing` parameter into another type.
    #[inline]
    pub(crate) fn set_sharing<T2>(self, sharing: Option<T2>) -> ContextBuilderWrapper<T2> {
        ContextBuilderWrapper {
            sharing,
            profile: self.profile,
            debug: self.debug,
            robustness: self.robustness,
        }
    }
}

impl<T> Default for ContextBuilderWrapper<T> {
    /// Initializes a new `ContextBuilder` with default values.
    fn default() -> Self {
        ContextBuilderWrapper {
            sharing: None,
            profile: None,
            debug: cfg!(debug_assertions),
            robustness: Robustness::NotRobust,
        }
    }
}

impl<T> ContextBuilderWrapper<T> {
    fn new() -> Self {
        Default::default()
    }
}

impl<T> ContextBuilderWrapper<T> {
    /// Sets the desired OpenGL [`Context`] profile.
    ///
    /// [`Context`]: struct.Context.html
    #[inline]
    pub fn with_gl_profile(mut self, profile: GlProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Sets the *debug* flag for the OpenGL [`Context`].
    ///
    /// The default value for this flag is `cfg!(debug_assertions)`, which means
    /// that it's enabled when you run `cargo build` and disabled when you run
    /// `cargo build --release`.
    ///
    /// [`Context`]: struct.Context.html
    #[inline]
    pub fn with_gl_debug_flag(mut self, flag: bool) -> Self {
        self.debug = flag;
        self
    }

    /// Sets the robustness of the OpenGL [`Context`]. See the docs of
    /// [`Robustness`].
    ///
    /// [`Context`]: struct.Context.html
    /// [`Robustness`]: enum.Robustness.html
    #[inline]
    pub fn with_gl_robustness(mut self, robustness: Robustness) -> Self {
        self.robustness = robustness;
        self
    }

    /// Share the display lists with the given [`Context`].
    ///
    /// [`Context`]: struct.Context.html
    #[inline]
    pub fn with_shared_lists<T2>(self, other: T2) -> ContextBuilderWrapper<T2> {
        self.set_sharing(Some(other.into()))
    }
}

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
    /// recreated.
    RobustLoseContextOnReset,

    /// Same as [`RobustLoseContextOnReset`] but the context creation doesn't
    /// fail if it's not supported.
    ///
    /// [`RobustLoseContextOnReset`]:
    /// enum.Robustness.html#variant.RobustLoseContextOnReset
    TryRobustLoseContextOnReset,
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
