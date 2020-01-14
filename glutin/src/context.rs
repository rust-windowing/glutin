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

/// An object that allows you to build a [`Context`].
///
/// For details on what each member controls, please scroll through the
/// [methods] bellow.
///
/// For what the defaults currently are, please refer to our [defaults
/// implementation].
///
/// **WARNING:** Glutin clients should use the [`ContextBuilder`] type in their
/// code, not this type. If I had a choice, I'd hide this type, but alas, due to
/// limitations in rustdoc, I cannot.
///
/// **WARNING:** [`Context`]s are built with the annoyingly hidden [`build`]
/// function. Once again, rustdoc!
///
/// [`Context`]: crate::context::Context
/// [methods]: ./struct.ContextBuilderWrapper.html#methods
/// [defaults implementation]: ./struct.ContextBuilderWrapper.html#impl-Default
/// [`ContextBuilder`]: crate::context::ContextBuilder
/// [`build`]: crate::context::ContextBuilderWrapper::build
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ContextBuilderWrapper<T> {
    pub sharing: Option<T>,
    pub profile: Option<GlProfile>,
    pub debug: bool,
    pub robustness: Robustness,
    pub release_behavior: ReleaseBehavior,
}

/// A simple type alias for [`ContextBuilderWrapper`]. Glutin clients should use
/// this type in their code, not [`ContextBuilderWrapper`]. If I had a choice,
/// I'd hide [`ContextBuilderWrapper`], but alas, due to limitations in rustdoc,
/// I cannot.
///
/// [`ContextBuilderWrapper`]: crate::context::ContextBuilderWrapper
pub type ContextBuilder<'a> = ContextBuilderWrapper<&'a Context>;

impl<'a> ContextBuilder<'a> {
    /// Builds a [`Context`] that matches the specified requirements.
    ///
    /// [`Context`]: crate::context::Context
    #[inline]
    pub fn build(self, conf: &Config) -> Result<Context, Error> {
        let cb = self.map_sharing(|ctx| &ctx.0);
        platform_impl::Context::new(cb, conf.as_ref()).map(Context)
    }
}

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
            release_behavior: self.release_behavior,
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
            release_behavior: self.release_behavior,
        }
    }
}

impl<T> Default for ContextBuilderWrapper<T> {
    /// Initializes a new `ContextBuilder` with default values.
    #[inline]
    fn default() -> Self {
        ContextBuilderWrapper {
            sharing: None,
            profile: None,
            debug: cfg!(debug_assertions),
            release_behavior: Default::default(),
            robustness: Default::default(),
        }
    }
}

impl<T> ContextBuilderWrapper<T> {
    #[inline]
    fn new() -> Self {
        Default::default()
    }
}

impl<T> ContextBuilderWrapper<T> {
    /// Sets the desired OpenGL [`Context`] profile.
    ///
    /// Please refer to the docs of [`GlProfile`].
    ///
    /// [`Context`]: crate::context::Context
    /// [`GlProfile`]: crate::context::GlProfile
    #[inline]
    pub fn with_profile(mut self, profile: GlProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Sets the *debug* flag for the OpenGL [`Context`].
    ///
    /// Debug contexts are usually slower but give better error reporting.
    ///
    /// The default value for this flag is `cfg!(debug_assertions)`, which means
    /// that it's enabled when you run `cargo build` and disabled when you run
    /// `cargo build --release`.
    ///
    /// [`Context`]: crate::context::Context
    #[inline]
    pub fn with_debug_flag(mut self, flag: bool) -> Self {
        self.debug = flag;
        self
    }

    /// Sets the robustness of the OpenGL [`Context`]. See the docs of
    /// [`Robustness`].
    ///
    /// The default is [`NotRobust`] because this is what is typically expected
    /// when you create an OpenGL [`Context`]. However for safety you should
    /// consider [`TryRobustLoseContextOnReset`].
    ///
    /// [`Context`]: crate::context::Context
    /// [`Robustness`]: crate::context::Robustness
    /// [`NotRobust`]: crate::context::Robustness::NotRobust
    /// [`TryRobustLoseContextOnReset`]: crate::context::Robustness::TryRobustLoseContextOnReset
    #[inline]
    pub fn with_robustness(mut self, robustness: Robustness) -> Self {
        self.robustness = robustness;
        self
    }

    /// Share the display lists with the given [`Context`].
    ///
    /// One notable limitation of the Wayland backend when it comes to shared
    /// [`Context`]s is that both contexts must use the same events loop.
    ///
    /// It should come to no one's surprise that [`Context`]s can only share
    /// display lists if they use the same implementation of OpenGL.
    ///
    /// Some platforms (e.g. Windows) only guarantee success if both [`Context`]s
    /// are made with the same [`Config`eration] while others appear to be more
    /// lenient. As with all things graphics related, the best way to check that
    /// something works is to test!
    ///
    /// [`Context`]: crate::context::Context
    /// [`Config`eration]: crate::config::ConfigWrapper
    #[inline]
    pub fn with_shared_lists<T2>(self, other: T2) -> ContextBuilderWrapper<T2> {
        self.set_sharing(Some(other.into()))
    }

    /// The behavior when changing the current [`Context`].
    ///
    /// Please refer to [`ReleaseBehavior`]'s docs for more details.
    ///
    /// [`Context`]: crate::context::Context
    /// [`ReleaseBehavior`]: crate::context::ReleaseBehavior
    #[inline]
    pub fn with_release_behaviour(mut self, release_behavior: ReleaseBehavior) -> Self {
        self.release_behavior = release_behavior;
        self
    }
}

/// Specifies the tolerance of the OpenGL [`Context`] to faults. If you accept
/// raw OpenGL commands and/or raw shader code from an untrusted source, you
/// should definitely care about this.
///
/// [`Context`]: crate::context::Context
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
    /// [`NotRobust`]: crate::context::Robustness::NotRobust
    NoError,

    /// Everything is checked to avoid any crash. The driver will attempt to
    /// avoid any problem, but if a problem occurs the behavior is
    /// implementation-defined. You are just guaranteed not to get a crash.
    RobustNoResetNotification,

    /// Same as [`RobustNoResetNotification`] but the context creation doesn't
    /// fail if it's not supported.
    ///
    /// [`RobustNoResetNotification`]: crate::context::Robustness::RobustNoResetNotification
    TryRobustNoResetNotification,

    /// Everything is checked to avoid any crash. If a problem occurs, the
    /// context will enter a "context lost" state. It must then be
    /// recreated.
    RobustLoseContextOnReset,

    /// Same as [`RobustLoseContextOnReset`] but the context creation doesn't
    /// fail if it's not supported.
    ///
    /// [`RobustLoseContextOnReset`]: crate::context::Robustness::RobustLoseContextOnReset
    TryRobustLoseContextOnReset,
}
impl Default for Robustness {
    #[inline]
    fn default() -> Self {
        Robustness::NotRobust
    }
}

/// Describes the requested OpenGL [`Context`] profiles.
///
/// [`Context`]: crate::context::Context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlProfile {
    /// Include all the immediate more functions and definitions.
    Compatibility,
    /// Include all the future-compatible functions and definitions.
    Core,
}

/// The behavior of the driver when you change the current context.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReleaseBehavior {
    /// Doesn't do anything. Most notably doesn't flush. Not supported by all
    /// drivers.
    None,

    /// Flushes the context that was previously current as if `glFlush` was
    /// called. This is the default behaviour.
    Flush,
}

impl Default for ReleaseBehavior {
    #[inline]
    fn default() -> Self {
        ReleaseBehavior::Flush
    }
}
