use winimpl;

/// Object that allows you to build headless contexts.
pub struct HeadlessRendererBuilder {
    dimensions: (uint, uint),
    gl_version: Option<(uint, uint)>,
}

impl HeadlessRendererBuilder {
    /// Initializes a new `HeadlessRendererBuilder` with default values.
    pub fn new(width: uint, height: uint) -> HeadlessRendererBuilder {
        HeadlessRendererBuilder {
            dimensions: (width, height),
            gl_version: None,
        }
    }

    /// Requests to use a specific OpenGL version.
    ///
    /// Version is a (major, minor) pair. For example to request OpenGL 3.3
    ///  you would pass `(3, 3)`.
    pub fn with_gl_version(mut self, version: (uint, uint)) -> HeadlessRendererBuilder {
        self.gl_version = Some(version);
        self
    }

    /// Builds the headless context.
    ///
    /// Error should be very rare and only occur in case of permission denied, incompatible system,
    ///  out of memory, etc.
    pub fn build(self) -> Result<HeadlessContext, String> {
        winimpl::HeadlessContext::new(self).map(|w| HeadlessContext { context: w })
    }
}

/// Represents a headless OpenGL context.
pub struct HeadlessContext {
    context: winimpl::HeadlessContext,
}

impl HeadlessContext {
    /// Creates a new OpenGL context
    /// Sets the context as the current context.
    #[inline]
    pub unsafe fn make_current(&self) {
        self.context.make_current()
    }

    /// Returns the address of an OpenGL function.
    ///
    /// Contrary to `wglGetProcAddress`, all available OpenGL functions return an address.
    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const libc::c_void {
        self.context.get_proc_address(addr) as *const libc::c_void
    }
}
