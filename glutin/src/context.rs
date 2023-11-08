//! OpenGL context creation and initialization.

#![allow(unreachable_patterns)]
use std::ffi;

use raw_window_handle::RawWindowHandle;

use crate::config::{Config, GetGlConfig};
use crate::display::{Display, GetGlDisplay};
use crate::error::Result;
use crate::private::{gl_api_dispatch, Sealed};
use crate::surface::{GlSurface, Surface, SurfaceTypeTrait};

#[cfg(cgl_backend)]
use crate::api::cgl::context::{
    NotCurrentContext as NotCurrentCglContext, PossiblyCurrentContext as PossiblyCurrentCglContext,
};
#[cfg(egl_backend)]
use crate::api::egl::context::{
    NotCurrentContext as NotCurrentEglContext, PossiblyCurrentContext as PossiblyCurrentEglContext,
};
#[cfg(glx_backend)]
use crate::api::glx::context::{
    NotCurrentContext as NotCurrentGlxContext, PossiblyCurrentContext as PossiblyCurrentGlxContext,
};
#[cfg(wgl_backend)]
use crate::api::wgl::context::{
    NotCurrentContext as NotCurrentWglContext, PossiblyCurrentContext as PossiblyCurrentWglContext,
};

/// A trait to group common context operations.
pub trait GlContext: Sealed {
    /// Get the [`ContextApi`] used by the context.
    ///
    /// The returned value's [`Version`] will always be `None`.
    fn context_api(&self) -> ContextApi;
}

/// A trait to group common not current operations.
pub trait NotCurrentGlContext: Sealed {
    /// The type of possibly current context.
    type PossiblyCurrentContext: PossiblyCurrentGlContext;

    /// The surface supported by the context.
    type Surface<T: SurfaceTypeTrait>: GlSurface<T>;

    /// Treat the not current context as possibly current. The operation is safe
    /// because the possibly current context is more restricted and not
    /// guaranteed to be current.
    fn treat_as_possibly_current(self) -> Self::PossiblyCurrentContext;

    /// Make [`Self::Surface`] on the calling thread producing the
    /// [`Self::PossiblyCurrentContext`] indicating that the context could
    /// be current on the theard.
    ///
    /// # Platform specific
    ///
    /// - **macOS: this will block if your main thread is blocked**;
    /// - **Wayland:** this call may latch the underlying back buffer (will do
    ///   with mesa drivers), meaning that all resize operations will apply
    ///   after the next [`GlSurface::swap_buffers`].
    fn make_current<T: SurfaceTypeTrait>(
        self,
        surface: &Self::Surface<T>,
    ) -> Result<Self::PossiblyCurrentContext>;

    /// The same as [`Self::make_current`], but provides a way to set read and
    /// draw surfaces.
    ///
    /// # Api-specific
    ///
    /// - **WGL/CGL:** not supported.
    fn make_current_draw_read<T: SurfaceTypeTrait>(
        self,
        surface_draw: &Self::Surface<T>,
        surface_read: &Self::Surface<T>,
    ) -> Result<Self::PossiblyCurrentContext>;
}

/// A trait to group common context operations.
pub trait PossiblyCurrentGlContext: Sealed {
    /// The not current context type.
    type NotCurrentContext: NotCurrentGlContext;

    /// The surface supported by the context.
    type Surface<T: SurfaceTypeTrait>: GlSurface<T>;

    /// Returns `true` if this context is the current one in this thread.
    fn is_current(&self) -> bool;

    /// Make the context not current to the current thread and returns a
    /// [`Self::NotCurrentContext`] to indicate that the context is a not
    /// current to allow sending it to the different thread.
    ///
    /// # Platform specific
    ///
    /// - **macOS: this will block if your main thread is blocked.**
    fn make_not_current(self) -> Result<Self::NotCurrentContext>;

    /// Make [`Self::Surface`] current on the calling thread.
    ///
    /// # Platform specific
    ///
    /// - **macOS: this will block if your main thread is blocked.**
    fn make_current<T: SurfaceTypeTrait>(&self, surface: &Self::Surface<T>) -> Result<()>;

    /// The same as [`Self::make_current`] but provides a way to set read and
    /// draw surfaces explicitly.
    ///
    /// # Api-specific
    ///
    /// - **CGL/WGL:** not supported.
    fn make_current_draw_read<T: SurfaceTypeTrait>(
        &self,
        surface_draw: &Self::Surface<T>,
        surface_read: &Self::Surface<T>,
    ) -> Result<()>;
}

/// A trait that provides raw context.
pub trait AsRawContext {
    /// Get the raw context handle.
    fn raw_context(&self) -> RawContext;
}

/// The builder to help customizing context
#[derive(Default, Debug, Clone)]
pub struct ContextAttributesBuilder {
    attributes: ContextAttributes,
}

impl ContextAttributesBuilder {
    /// Create new builder.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the *debug* flag for the OpenGL context.
    ///
    /// Debug contexts are usually slower, but give better error reporting.
    /// This option is ignored when using [`Robustness::NoError`].
    ///
    /// The default value for this flag is `false`.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.attributes.debug = debug;
        self
    }

    /// Share the display lists with the given context.
    ///
    /// To get sharing working it's recommended to use the same [`Config`] when
    /// creating contexts that are going to be shared.
    ///
    /// # Platform-specific
    ///
    /// - **Wayland:** both contexts must use the same Wayland connection.
    ///
    /// [`Config`]: crate::config::Config
    pub fn with_sharing(mut self, context: &impl AsRawContext) -> Self {
        self.attributes.shared_context = Some(context.raw_context());
        self
    }

    /// Sets the robustness of the OpenGL context. See the docs of
    /// [`Robustness`].
    ///
    /// The default is [`Robustness::NotRobust`], because this is what typically
    /// expected when you create an OpenGL context.  However for safety you
    /// should consider [`Robustness::RobustLoseContextOnReset`].
    pub fn with_robustness(mut self, robustness: Robustness) -> Self {
        self.attributes.robustness = robustness;
        self
    }

    /// The behavior when changing the current context. See the docs of
    /// [`ReleaseBehavior`].
    ///
    /// The default is [`ReleaseBehavior::Flush`].
    pub fn with_release_behavior(mut self, release_behavior: ReleaseBehavior) -> Self {
        self.attributes.release_behavior = release_behavior;
        self
    }

    /// Set the desired OpenGL context profile. See the docs of [`GlProfile`].
    ///
    /// By default the profile is unspecified.
    ///
    /// # Api-specific
    ///
    /// - **macOS:** not supported, the latest is picked automatically.
    pub fn with_profile(mut self, profile: GlProfile) -> Self {
        self.attributes.profile = Some(profile);
        self
    }

    /// Set the desired OpenGL context api. See the docs of [`ContextApi`].
    ///
    /// By default the supported api will be picked.
    pub fn with_context_api(mut self, api: ContextApi) -> Self {
        self.attributes.api = Some(api);
        self
    }

    /// Build the context attributes.
    ///
    /// The `raw_window_handle` isn't required and here for WGL compatibility.
    ///
    /// # Api-specific
    ///
    /// - **WGL:** you **must** pass a `raw_window_handle` if you plan to use
    ///   this context with that window.
    pub fn build(mut self, raw_window_handle: Option<RawWindowHandle>) -> ContextAttributes {
        self.attributes.raw_window_handle = raw_window_handle;
        self.attributes
    }
}

/// The attributes that are used to create a graphics context.
#[derive(Default, Debug, Clone)]
pub struct ContextAttributes {
    pub(crate) release_behavior: ReleaseBehavior,

    pub(crate) debug: bool,

    pub(crate) robustness: Robustness,

    pub(crate) profile: Option<GlProfile>,

    pub(crate) api: Option<ContextApi>,

    pub(crate) shared_context: Option<RawContext>,

    pub(crate) raw_window_handle: Option<RawWindowHandle>,
}

/// Specifies the tolerance of the OpenGL context to faults. If you accept
/// raw OpenGL commands and/or raw shader code from an untrusted source, you
/// should definitely care about this.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Robustness {
    /// Not everything is checked. Your application can crash if you do
    /// something wrong with your shaders.
    #[default]
    NotRobust,

    /// The driver doesn't check anything. This option is very dangerous.
    /// Please know what you're doing before using it. See the
    /// `GL_KHR_no_error` extension.
    ///
    /// Since this option is purely an optimization, no error will be returned
    /// if the backend doesn't support it. Instead it will automatically
    /// fall back to [`Robustness::NotRobust`].
    NoError,

    /// Everything is checked to avoid any crash. The driver will attempt to
    /// avoid any problem, but if a problem occurs the behavior is
    /// implementation-defined. You are just guaranteed not to get a crash.
    RobustNoResetNotification,

    /// Everything is checked to avoid any crash. If a problem occurs, the
    /// context will enter a "context lost" state. It must then be
    /// recreated.
    RobustLoseContextOnReset,
}

/// Describes the requested OpenGL context profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlProfile {
    /// Include all the future-compatible functions and definitions.
    ///
    /// The requested OpenGL version with [`ContextApi`] should be at least 3.3.
    Core,
    /// Include all the immediate more functions and definitions.
    ///
    /// Use it only when it's really needed, otherwise use [`Self::Core`].
    Compatibility,
}

/// The rendering Api context should support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextApi {
    /// OpenGL Api version that should be used by the context.
    ///
    /// When using `None` as `Version` any OpenGL context will be picked,
    /// however when the [`GlProfile::Core`] is used at least 3.3 will be
    /// requested.
    OpenGl(Option<Version>),

    /// OpenGL Api version that should be used by the context.
    ///
    /// When using `None` as `Version` the latest **known** major version is
    /// picked. Versions that are higher than what was picked automatically
    /// could still be supported.
    Gles(Option<Version>),
}

#[cfg(any(egl_backend, glx_backend, wgl_backend))]
impl ContextApi {
    pub(crate) fn version(&self) -> Option<Version> {
        match self {
            Self::OpenGl(version) => *version,
            Self::Gles(version) => *version,
            _ => None,
        }
    }
}

impl Default for ContextApi {
    fn default() -> Self {
        Self::OpenGl(None)
    }
}

/// The version used to index the Api.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major version of the Api.
    pub major: u8,
    /// Minor version of the Api.
    pub minor: u8,
}

impl Version {
    /// Create new version with the given `major` and `minor` values.
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

/// The behavior of the driver when you change the current context.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ReleaseBehavior {
    /// Doesn't do anything. Most notably doesn't flush. Not supported by all
    /// drivers.
    ///
    /// # Api-specific
    ///
    /// - **macOS:** not supported, [`Self::Flush`] is always used.
    None,

    /// Flushes the context that was previously current as if `glFlush` was
    /// called. This is the default behavior.
    #[default]
    Flush,
}

/// A context that is known to be not current on the current thread.
///
/// This type is a safe wrapper around the context to indicate that it could be
/// `Send` to the different thread, since the context must be not current before
/// doing so.
///
/// ```no_run
/// fn test_send<T: Send>() {}
/// test_send::<glutin::context::NotCurrentContext>();
/// ```
/// However it's not `Sync`.
/// ```compile_fail
/// fn test_sync<T: Sync>() {}
/// test_sync::<glutin::context::NotCurrentContext>();
/// ```
#[derive(Debug)]
pub enum NotCurrentContext {
    /// The EGL context.
    #[cfg(egl_backend)]
    Egl(NotCurrentEglContext),

    /// The GLX context.
    #[cfg(glx_backend)]
    Glx(NotCurrentGlxContext),

    /// The WGL context.
    #[cfg(wgl_backend)]
    Wgl(NotCurrentWglContext),

    /// The CGL context.
    #[cfg(cgl_backend)]
    Cgl(NotCurrentCglContext),
}

impl NotCurrentGlContext for NotCurrentContext {
    type PossiblyCurrentContext = PossiblyCurrentContext;
    type Surface<T: SurfaceTypeTrait> = Surface<T>;

    fn treat_as_possibly_current(self) -> Self::PossiblyCurrentContext {
        gl_api_dispatch!(self; Self(context) => context.treat_as_possibly_current(); as PossiblyCurrentContext)
    }

    fn make_current<T: SurfaceTypeTrait>(
        self,
        surface: &Self::Surface<T>,
    ) -> Result<Self::PossiblyCurrentContext> {
        match (self, surface) {
            #[cfg(egl_backend)]
            (Self::Egl(context), Surface::Egl(surface)) => {
                Ok(PossiblyCurrentContext::Egl(context.make_current(surface)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(context), Surface::Glx(surface)) => {
                Ok(PossiblyCurrentContext::Glx(context.make_current(surface)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(context), Surface::Wgl(surface)) => {
                Ok(PossiblyCurrentContext::Wgl(context.make_current(surface)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(context), Surface::Cgl(surface)) => {
                Ok(PossiblyCurrentContext::Cgl(context.make_current(surface)?))
            },
            _ => unreachable!(),
        }
    }

    fn make_current_draw_read<T: SurfaceTypeTrait>(
        self,
        surface_draw: &Self::Surface<T>,
        surface_read: &Self::Surface<T>,
    ) -> Result<Self::PossiblyCurrentContext> {
        match (self, surface_draw, surface_read) {
            #[cfg(egl_backend)]
            (Self::Egl(context), Surface::Egl(draw), Surface::Egl(read)) => {
                Ok(PossiblyCurrentContext::Egl(context.make_current_draw_read(draw, read)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(context), Surface::Glx(draw), Surface::Glx(read)) => {
                Ok(PossiblyCurrentContext::Glx(context.make_current_draw_read(draw, read)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(context), Surface::Wgl(draw), Surface::Wgl(read)) => {
                Ok(PossiblyCurrentContext::Wgl(context.make_current_draw_read(draw, read)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(context), Surface::Cgl(draw), Surface::Cgl(read)) => {
                Ok(PossiblyCurrentContext::Cgl(context.make_current_draw_read(draw, read)?))
            },
            _ => unreachable!(),
        }
    }
}

impl GlContext for NotCurrentContext {
    fn context_api(&self) -> ContextApi {
        gl_api_dispatch!(self; Self(context) => context.context_api())
    }
}

impl GetGlConfig for NotCurrentContext {
    type Target = Config;

    fn config(&self) -> Self::Target {
        gl_api_dispatch!(self; Self(context) => context.config(); as Config)
    }
}

impl GetGlDisplay for NotCurrentContext {
    type Target = Display;

    fn display(&self) -> Self::Target {
        gl_api_dispatch!(self; Self(context) => context.display(); as Display)
    }
}

impl AsRawContext for NotCurrentContext {
    fn raw_context(&self) -> RawContext {
        gl_api_dispatch!(self; Self(context) => context.raw_context())
    }
}

impl Sealed for NotCurrentContext {}

/// A context that is possibly current on the current thread.
///
/// The context that could be current on the current thread can neither be
/// [`Send`] nor [`Sync`]. In case you need to use it on a different thread
/// [make it not current].
/// ```compile_fail
/// fn test_send<T: Send>() {}
/// test_send::<glutin::context::PossiblyCurrentContext>();
/// ```
///
/// ```compile_fail
/// fn test_sync<T: Sync>() {}
/// test_sync::<glutin::context::PossiblyCurrentContext>();
/// ```
///
/// [make it not current]: crate::context::PossiblyCurrentGlContext::make_not_current
#[derive(Debug)]
pub enum PossiblyCurrentContext {
    /// The EGL context.
    #[cfg(egl_backend)]
    Egl(PossiblyCurrentEglContext),

    /// The GLX context.
    #[cfg(glx_backend)]
    Glx(PossiblyCurrentGlxContext),

    /// The WGL context.
    #[cfg(wgl_backend)]
    Wgl(PossiblyCurrentWglContext),

    /// The CGL context.
    #[cfg(cgl_backend)]
    Cgl(PossiblyCurrentCglContext),
}

impl PossiblyCurrentGlContext for PossiblyCurrentContext {
    type NotCurrentContext = NotCurrentContext;
    type Surface<T: SurfaceTypeTrait> = Surface<T>;

    fn is_current(&self) -> bool {
        gl_api_dispatch!(self; Self(context) => context.is_current())
    }

    fn make_not_current(self) -> Result<Self::NotCurrentContext> {
        Ok(
            gl_api_dispatch!(self; Self(context) => context.make_not_current()?; as NotCurrentContext),
        )
    }

    fn make_current<T: SurfaceTypeTrait>(&self, surface: &Self::Surface<T>) -> Result<()> {
        match (self, surface) {
            #[cfg(egl_backend)]
            (Self::Egl(context), Surface::Egl(surface)) => context.make_current(surface),
            #[cfg(glx_backend)]
            (Self::Glx(context), Surface::Glx(surface)) => context.make_current(surface),
            #[cfg(wgl_backend)]
            (Self::Wgl(context), Surface::Wgl(surface)) => context.make_current(surface),
            #[cfg(cgl_backend)]
            (Self::Cgl(context), Surface::Cgl(surface)) => context.make_current(surface),
            _ => unreachable!(),
        }
    }

    fn make_current_draw_read<T: SurfaceTypeTrait>(
        &self,
        surface_draw: &Self::Surface<T>,
        surface_read: &Self::Surface<T>,
    ) -> Result<()> {
        match (self, surface_draw, surface_read) {
            #[cfg(egl_backend)]
            (Self::Egl(context), Surface::Egl(draw), Surface::Egl(read)) => {
                context.make_current_draw_read(draw, read)
            },
            #[cfg(glx_backend)]
            (Self::Glx(context), Surface::Glx(draw), Surface::Glx(read)) => {
                context.make_current_draw_read(draw, read)
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(context), Surface::Wgl(draw), Surface::Wgl(read)) => {
                context.make_current_draw_read(draw, read)
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(context), Surface::Cgl(draw), Surface::Cgl(read)) => {
                context.make_current_draw_read(draw, read)
            },
            _ => unreachable!(),
        }
    }
}

impl GlContext for PossiblyCurrentContext {
    fn context_api(&self) -> ContextApi {
        gl_api_dispatch!(self; Self(context) => context.context_api())
    }
}

impl GetGlConfig for PossiblyCurrentContext {
    type Target = Config;

    fn config(&self) -> Self::Target {
        gl_api_dispatch!(self; Self(context) => context.config(); as Config)
    }
}

impl GetGlDisplay for PossiblyCurrentContext {
    type Target = Display;

    fn display(&self) -> Self::Target {
        gl_api_dispatch!(self; Self(context) => context.display(); as Display)
    }
}

impl AsRawContext for PossiblyCurrentContext {
    fn raw_context(&self) -> RawContext {
        gl_api_dispatch!(self; Self(context) => context.raw_context())
    }
}

impl Sealed for PossiblyCurrentContext {}

/// Raw context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawContext {
    /// Raw EGL context.
    #[cfg(egl_backend)]
    Egl(*const ffi::c_void),

    /// Raw GLX context.
    #[cfg(glx_backend)]
    Glx(*const ffi::c_void),

    /// HGLRC pointer.
    #[cfg(wgl_backend)]
    Wgl(*const ffi::c_void),

    /// Pointer to NSOpenGLContext.
    #[cfg(cgl_backend)]
    Cgl(*const ffi::c_void),
}

/// Pick `GlProfile` and `Version` based on the provided params.
#[cfg(any(egl_backend, glx_backend, wgl_backend))]
pub(crate) fn pick_profile(
    profile: Option<GlProfile>,
    version: Option<Version>,
) -> (GlProfile, Version) {
    match (profile, version) {
        (Some(GlProfile::Core), Some(version)) => (GlProfile::Core, version),
        (Some(GlProfile::Compatibility), Some(version)) => (GlProfile::Compatibility, version),
        (None, Some(version)) if version >= Version::new(3, 3) => (GlProfile::Core, version),
        (None, Some(version)) => (GlProfile::Compatibility, version),
        (Some(GlProfile::Core), None) => (GlProfile::Core, Version::new(3, 3)),
        (Some(GlProfile::Compatibility), None) => (GlProfile::Compatibility, Version::new(2, 1)),
        (None, None) => (GlProfile::Core, Version::new(3, 3)),
    }
}
