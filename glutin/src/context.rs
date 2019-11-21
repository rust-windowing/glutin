use super::*;
use crate::config::Config;
use crate::display::Display;
use crate::surface::{PBuffer, WindowSurface};
use std::ffi::c_void;

#[derive(Debug)]
pub struct Context {
    pub(crate) context: platform_impl::Context,
}

impl Context {
    #[inline]
    pub(crate) fn inner(&self) -> &platform_impl::Context {
        &self.context
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), ContextError> {
        self.context.make_current_surfaceless()
    }

    #[inline]
    pub unsafe fn make_current_surface(
        &self,
        surface: &WindowSurface,
    ) -> Result<(), ContextError> {
        self.context.make_current_surface(surface.inner())
    }

    #[inline]
    pub unsafe fn make_current_pbuffer(
        &self,
        pbuffer: &PBuffer,
    ) -> Result<(), ContextError> {
        self.context.make_current_pbuffer(pbuffer.inner())
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        self.context.make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    #[inline]
    pub fn get_config(&self) -> Config {
        self.context.get_config()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const c_void {
        self.context.get_proc_address(addr)
    }

    #[inline]
    pub fn update_after_resize(&self) {
        #[cfg(target_os = "macos")]
        self.context.update_after_resize()
    }
}

impl<'a> ContextBuilder<'a> {
    #[inline]
    pub fn build<TE>(
        self,
        el: &Display,
        supports_surfaceless: bool,
        conf: &Config,
    ) -> Result<Context, CreationError> {
        let cb = self.map_sharing(|ctx| &ctx.context);
        platform_impl::Context::new(
            el,
            cb,
            supports_surfaceless,
            conf.with_config(&conf.config),
        )
        .map(|context| Context { context })
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

    /// Platform specific attributes
    pub plat_attr: platform_impl::ContextPlatformAttributes,
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
            plat_attr: self.plat_attr,
        }
    }

    /// Turns the `sharing` parameter into another type.
    #[inline]
    pub(crate) fn set_sharing<T2>(
        self,
        sharing: Option<T2>,
    ) -> ContextBuilderWrapper<T2> {
        ContextBuilderWrapper {
            sharing,
            profile: self.profile,
            debug: self.debug,
            robustness: self.robustness,
            plat_attr: self.plat_attr,
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
            plat_attr: Default::default(),
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

/// Error that can happen when manipulating an OpenGL [`Context`].
///
/// [`Context`]: struct.Context.html
#[derive(Debug)]
pub enum ContextError {
    /// General platform error.
    OsError(String),
    IoError(io::Error),
    ContextLost,
}

impl ContextError {
    fn to_string(&self) -> &str {
        use std::error::Error;
        match *self {
            ContextError::OsError(ref string) => string,
            ContextError::IoError(ref err) => err.description(),
            ContextError::ContextLost => "Context lost",
        }
    }
}

impl std::fmt::Display for ContextError {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> Result<(), std::fmt::Error> {
        formatter.write_str(self.to_string())
    }
}

impl std::error::Error for ContextError {
    fn description(&self) -> &str {
        self.to_string()
    }
}
