//! Everything related to creating and manipulating [`Context`]s.
//!
//! You can use a [`ContextBuilder`] along side with a [`Config`] to get
//! a [`Context`] with the requested parameters.
//!
//! [`Context`]s can be made current either via [`make_current_surfaceless`] or
//! [`make_current`]. Please refer to those functions for more details, if
//! interested.
//!
//! **WARNING:** Glutin clients should use the [`ContextBuilder`] type in their
//! code, not [`ContextBuilderWrapper`]. If I had a choice, I'd hide that type,
//! but alas, due to limitations in rustdoc, I cannot. Unfortunately, almost all
//! of [`ContextBuilder`]'s methods are only visible on
//! [`ContextBuilderWrapper`], which exception of the [`build`] function which
//! can only be found on the former.
//!
//! [`ContextBuilder`]: crate::context::ContextBuilder
//! [`Context`]: crate::context::Context
//! [`ContextBuilderWrapper`]: crate::context::ContextBuilderWrapper
//! [`ContextBuilder`]: crate::context::ContextBuilderWrapper
//! [`build`]: crate::context::ContextBuilderWrapper::build
//! [`make_current_surfaceless`]: crate::context::Context::make_current_surfaceless
//! [`make_current`]: crate::context::Context::make_current
//! [`Config`]: crate::config::ConfigWrapper

use crate::config::Api;
use crate::config::Config;
use crate::platform_impl;
use crate::surface::{Surface, SurfaceTypeTrait};

use winit_types::error::{Error, ErrorType};

use std::os::raw;

/// Represents an OpenGL context, which is the structure that holds the OpenGL
/// state.
///
/// Is built by a [`ContextBuilder`]. Can have some its resources shared with
/// another context via [`with_sharing`].
///
/// A context must be made current before using [`get_proc_address`] or any of
/// the functions returned by [`get_proc_address`].
///
/// Contexts can be made current either via [`make_current_surfaceless`] or
/// [`make_current`]. Please refer to those functions for more details, if
/// interested.
///
/// **WARNING** On MacOS, Glutin clients must call [`update_after_resize`],
/// [`make_current`], or [`make_current_surfaceless`] on the context whenever
/// the backing surface's size changes.
///
/// **WARNING** `Context`s cannot be used from threads they are not current on.
/// If dropped from a different thread than the one they are currently on, UB can
/// happen. If a context is current, please call [`make_not_current`] before
/// moving it between two threads.
///
/// [`ContextBuilder`]: crate::context::ContextBuilderWrapper
/// [`with_sharing`]: crate::context::ContextBuilderWrapper::with_sharing
/// [`get_proc_address`]: crate::context::Context::get_proc_address
/// [`make_current_surfaceless`]: crate::context::Context::make_current_surfaceless
/// [`make_current`]: crate::context::Context::make_current
/// [`make_not_current`]: crate::context::Context::make_not_current
/// [`update_after_resize`]: crate::context::Context::update_after_resize
#[derive(Debug, PartialEq, Eq)]
pub struct Context(pub(crate) platform_impl::Context);

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.make_not_current().unwrap();
        }
    }
}

impl Context {
    /// Sets this context as the current context. The previously current context
    /// on this thread (if any) is no longer current. The `Context`'s
    /// [`Config`] must have [`supports_surfaceless`] set to `true`.
    ///
    /// For how to handle errors, refer to [`make_current`].
    ///
    /// The previously current [`Context`] might get `glFlush`ed if its
    /// [`ReleaseBehaviour`] is equal to [`Flush`].
    ///
    /// [`make_current`]: crate::context::Context::make_current
    /// [`Config`]: crate::config::ConfigWrapper
    /// [`supports_surfaceless`]: crate::config::ConfigAttribs::supports_surfaceless
    /// [`ReleaseBehaviour`]: crate::context::ReleaseBehaviour
    /// [`Flush`]: crate::context::ReleaseBehaviour::Flush
    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        if !self.get_config().attribs().supports_surfaceless {
            return Err(make_error!(ErrorType::BadApiUsage(
                "`make_current_surfaceless` called on context with config without `supports_surfaceless`.".to_string()
            )));
        }
        self.0.make_current_surfaceless()
    }

    /// Sets this context as the current context. The previously current context
    /// on this thread (if any) is no longer current. The passed in [`Surface`]
    /// is also now current drawable.
    ///
    /// The [`Surface`] and the `Context` must have be made with the same
    /// [`Config`] or two [`Config`]s which are, due to some
    /// platform-specific reason, compatible. The [`Config`] must support
    /// the [`Surface`]'s type.
    ///
    /// The previously current [`Context`] might get `glFlush`ed if its
    /// [`ReleaseBehaviour`] is equal to [`Flush`].
    ///
    /// # Errors
    ///
    /// A failed call to `make_current`, [`make_current_surfaceless`] or
    /// [`make_not_current`] might make this, or no context current. It could
    /// also keep the previous context current. What happens varies by platform
    /// and error.
    ///
    /// To attempt to recover and get back into a know state, either:
    ///
    ///  * Attempt to use [`is_current`] to find the new current context,
    ///  * Call [`make_not_current`] on both the previously current context and
    ///  this context; or
    ///  * Call `make_current` or [`make_current_surfaceless`] on some context
    ///  successfully.
    ///
    /// [`make_current_surfaceless`]: crate::context::Context::make_current_surfaceless
    /// [`is_current`]: crate::context::Context::is_current
    /// [`make_not_current`]: crate::context::Context::make_not_current
    /// [`surface`]: crate::surface::Surface
    /// [`Config`]: crate::config::ConfigWrapper
    /// [`ReleaseBehaviour`]: crate::context::ReleaseBehaviour
    /// [`Flush`]: crate::context::ReleaseBehaviour::Flush
    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        if self.get_config() != surf.get_config() {
            warn!("[glutin] `make_current`: Your surface's and context's configurations don't match. Are you sure this is intentional?")
        }
        self.0.make_current(&surf.0)
    }

    /// If this context is current, makes this context not current. If this
    /// context is not current, however, then this function does nothing.
    ///
    /// The current [`Surface`], if any, will also become not current.
    ///
    /// The previously current [`Context`] might get `glFlush`ed if its
    /// [`ReleaseBehaviour`] is equal to [`Flush`].
    ///
    /// For how to handle errors, refer to [`make_current`].
    ///
    /// [`make_current`]: crate::context::Context::make_current
    /// [`Surface`]: crate::surface::Surface
    /// [`ReleaseBehaviour`]: crate::context::ReleaseBehaviour
    /// [`Flush`]: crate::context::ReleaseBehaviour::Flush
    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self.is_current() {
            true => self.0.make_not_current(),
            false => Ok(()),
        }
    }

    /// Returns `true` if this context is the current one in this thread.
    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    /// Returns the [`Config`] that the context was created with.
    ///
    /// [`Config`]: crate::config::ConfigWrapper
    #[inline]
    pub fn get_config(&self) -> Config {
        self.0.get_config()
    }

    /// Returns the address of an OpenGL function. This context should be current
    /// when doing so.
    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        if cfg!(debug_assertions) && !self.is_current() {
            return Err(make_error!(ErrorType::BadApiUsage(
                "`get_proc_address` called on context that is not current.".to_string()
            )));
        }
        self.0.get_proc_address(addr)
    }

    /// On MacOS, Glutin clients must call `update_after_resize`,
    /// [`make_current`], or [`make_current_surfaceless`] on the context whenever
    /// the backing [`Surface`]`<`[`Window`]`>`'s size changes.
    ///
    /// No-ops on other platforms. Please make sure to also call your
    /// [`Surface`]'s [`update_after_resize`].
    ///
    /// [`update_after_resize`]: crate::surface::Surface::update_after_resize
    /// [`make_current_surfaceless`]: crate::context::Context::make_current_surfaceless
    /// [`make_current`]: crate::context::Context::make_current
    /// [`Surface`]: crate::surface::Surface
    /// [`Window`]: crate::surface::Window
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
/// [`build`]: ./type.ContextBuilder.html#method.build
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBuilderWrapper<T> {
    pub sharing: Option<T>,
    pub profile: Option<GlProfile>,
    pub debug: bool,
    pub robustness: Robustness,
    pub release_behavior: ReleaseBehaviour,
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
    pub fn new() -> Self {
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
    /// are made with the same [`Config`] while others appear to be more
    /// lenient. As with all things graphics related, the best way to check that
    /// something works is to test!
    ///
    /// [`Context`]: crate::context::Context
    /// [`Config`]: crate::config::ConfigWrapper
    #[inline]
    pub fn with_sharing<T2>(self, sharing: Option<T2>) -> ContextBuilderWrapper<T2> {
        ContextBuilderWrapper {
            sharing,
            profile: self.profile,
            debug: self.debug,
            robustness: self.robustness,
            release_behavior: self.release_behavior,
        }
    }

    /// The behavior when changing the current [`Context`].
    ///
    /// Please refer to [`ReleaseBehaviour`]'s docs for more details.
    ///
    /// [`Context`]: crate::context::Context
    /// [`ReleaseBehaviour`]: crate::context::ReleaseBehaviour
    #[inline]
    pub fn with_release_behaviour(mut self, release_behavior: ReleaseBehaviour) -> Self {
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
pub enum ReleaseBehaviour {
    /// Doesn't do anything. Most notably doesn't flush. Not supported by all
    /// drivers.
    None,

    /// Flushes the context that was previously current as if `glFlush` was
    /// called. This is the default behaviour.
    Flush,
}

impl Default for ReleaseBehaviour {
    #[inline]
    fn default() -> Self {
        ReleaseBehaviour::Flush
    }
}
