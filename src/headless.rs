use Api;
use ContextError;
use CreationError;
use GlAttributes;
use GlContext;
use GlProfile;
use GlRequest;
use PixelFormat;
use PixelFormatRequirements;
use Robustness;

use platform;

/// Object that allows you to build headless contexts.
#[derive(Clone)]
pub struct HeadlessRendererBuilder<'a> {
    /// The dimensions to use.
    pub dimensions: (u32, u32),

    /// The OpenGL attributes to build the context with.
    pub opengl: GlAttributes<&'a platform::HeadlessContext>,

    // Should be made public once it's stabilized.
    pf_reqs: PixelFormatRequirements,

    /// Platform-specific configuration.
    platform_specific: platform::PlatformSpecificHeadlessBuilderAttributes,
}

impl<'a> HeadlessRendererBuilder<'a> {
    /// Initializes a new `HeadlessRendererBuilder` with default values.
    #[inline]
    pub fn new(width: u32, height: u32) -> HeadlessRendererBuilder<'a> {
        HeadlessRendererBuilder {
            dimensions: (width, height),
            pf_reqs: Default::default(),
            opengl: Default::default(),
            platform_specific: Default::default(),
        }
    }

    /// Sets how the backend should choose the OpenGL API and version.
    #[inline]
    pub fn with_gl(mut self, request: GlRequest) -> HeadlessRendererBuilder<'a> {
        self.opengl.version = request;
        self
    }

    /// Sets the desired OpenGL context profile.
    #[inline]
    pub fn with_gl_profile(mut self, profile: GlProfile) -> HeadlessRendererBuilder<'a> {
        self.opengl.profile = Some(profile);
        self
    }

    /// Sets the *debug* flag for the OpenGL context.
    ///
    /// The default value for this flag is `cfg!(ndebug)`, which means that it's enabled
    /// when you run `cargo build` and disabled when you run `cargo build --release`.
    #[inline]
    pub fn with_gl_debug_flag(mut self, flag: bool) -> HeadlessRendererBuilder<'a> {
        self.opengl.debug = flag;
        self
    }

    /// Sets the robustness of the OpenGL context. See the docs of `Robustness`.
    #[inline]
    pub fn with_gl_robustness(mut self, robustness: Robustness) -> HeadlessRendererBuilder<'a> {
        self.opengl.robustness = robustness;
        self
    }

    /// Builds the headless context.
    ///
    /// Error should be very rare and only occur in case of permission denied, incompatible system,
    ///  out of memory, etc.
    #[inline]
    pub fn build(self) -> Result<HeadlessContext, CreationError> {
        platform::HeadlessContext::new(self.dimensions, &self.pf_reqs, &self.opengl,
                                       &self.platform_specific)
                .map(|w| HeadlessContext { context: w })
    }

    /// Builds the headless context.
    ///
    /// The context is build in a *strict* way. That means that if the backend couldn't give
    /// you what you requested, an `Err` will be returned.
    #[inline]
    pub fn build_strict(self) -> Result<HeadlessContext, CreationError> {
        self.build()
    }
}

/// Represents a headless OpenGL context.
pub struct HeadlessContext {
    pub(crate) context: platform::HeadlessContext,
}

impl GlContext for HeadlessContext {
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

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        self.context.swap_buffers()
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format()
    }

    #[inline]
    fn resize(&self, _width: u32, _height: u32) {
        // This method does not mean anything for a HeadlessContext.
        unimplemented!()
    }
}
