use Api;
use ContextError;
use CreationError;
use EventsLoop;
use GlAttributes;
use GlContext;
use GlProfile;
use GlRequest;
use Robustness;

use platform;

/// Object that allows you to build pure contexts.
#[derive(Clone)]
pub struct PureRendererBuilder<'a> {
    /// The OpenGL attributes to build the context with.
    pub opengl: GlAttributes<&'a platform::PureContext>,

    /// Platform-specific configuration.
    platform_specific: platform::PlatformSpecificPureBuilderAttributes,
}

impl<'a> PureRendererBuilder<'a> {
    /// Initializes a new `PureRendererBuilder` with default values.
    #[inline]
    pub fn new() -> Self {
        PureRendererBuilder {
            opengl: Default::default(),
            platform_specific: Default::default(),
        }
    }

    /// Sets how the backend should choose the OpenGL API and version.
    #[inline]
    pub fn with_gl(mut self, request: GlRequest) -> Self {
        self.opengl.version = request;
        self
    }

    /// Sets the desired OpenGL context profile.
    #[inline]
    pub fn with_gl_profile(mut self, profile: GlProfile) -> Self {
        self.opengl.profile = Some(profile);
        self
    }

    /// Sets the *debug* flag for the OpenGL context.
    ///
    /// The default value for this flag is `cfg!(ndebug)`, which means that it's enabled
    /// when you run `cargo build` and disabled when you run `cargo build --release`.
    #[inline]
    pub fn with_gl_debug_flag(mut self, flag: bool) -> Self {
        self.opengl.debug = flag;
        self
    }

    /// Sets the robustness of the OpenGL context. See the docs of `Robustness`.
    #[inline]
    pub fn with_gl_robustness(mut self, robustness: Robustness) -> Self {
        self.opengl.robustness = robustness;
        self
    }

    /// Builds the headless context.
    ///
    /// Error should be very rare and only occur in case of permission denied, incompatible system,
    ///  out of memory, etc.
    #[inline]
    pub fn build(self, events_loop: &EventsLoop) -> Result<PureContext, CreationError> {
        platform::PureContext::new(events_loop, &self.opengl, &self.platform_specific)
            .map(|w| PureContext { context: w })
    }

    /// Builds the headless context.
    ///
    /// The context is build in a *strict* way. That means that if the backend couldn't give
    /// you what you requested, an `Err` will be returned.
    #[inline]
    pub fn build_strict(self, events_loop: &EventsLoop) -> Result<PureContext, CreationError> {
        self.build(events_loop)
    }
}

/// Represents a pure OpenGL context.
/// Pure contexts don't have main framebuffers. Users can only
/// render to their own surfaces/texture in custom framebuffers.
pub struct PureContext {
    pub(crate) context: platform::PureContext,
}

impl GlContext for PureContext {
    /// Creates a new OpenGL context
    /// Sets the context as the current context.
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    /// Returns true if this context is the current one in this thread.
    #[inline]
    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    /// Returns the address of an OpenGL function.
    ///
    /// Contrary to `wglGetProcAddress`, all available OpenGL functions return an address.
    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    /// Returns the API that is currently provided by this window.
    ///
    /// See `Window::get_api` for more infos.
    #[inline]
    fn get_api(&self) -> Api {
        self.context.get_api()
    }
}
