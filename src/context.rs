use super::*;

/// Represents an OpenGL context.
///
/// A `Context` is normally associated with a single Window, however `Context`s
/// can be *shared* between multiple windows or be headless.
///
/// # Example
///
/// ```no_run
/// # extern crate glutin;
/// # use glutin::ContextTrait;
/// # fn main() {
/// # let el = glutin::EventsLoop::new();
/// # let wb = glutin::WindowBuilder::new();
/// # let some_context = glutin::ContextBuilder::new()
/// #    .build_combined(wb, &el)
/// #    .unwrap();
/// let cb = glutin::ContextBuilder::new()
///     .with_vsync(true)
///     .with_multisampling(8)
///     .with_shared_lists(some_context.context());
/// # }
/// ```
pub struct Context {
    pub(crate) context: platform::Context,
}

impl ContextTrait for Context {
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    fn get_api(&self) -> Api {
        self.context.get_api()
    }
}

impl Context {
    /// Builds the given GL context.
    ///
    /// One notable limitation of the Wayland backend when it comes to shared
    /// contexts is that both contexts must use the same events loop.
    ///
    /// Errors can occur in two scenarios:
    ///  - If the window could not be created (via permission denied,
    ///  incompatible system, out of memory, etc.). This should be very rare.
    ///  - If the OpenGL context could not be created. This generally happens
    ///  because the underlying platform doesn't support a requested feature.
    pub fn new(
        el: &winit::EventsLoop,
        cb: ContextBuilder,
    ) -> Result<Self, CreationError> {
        let ContextBuilder { pf_reqs, gl_attr } = cb;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        platform::Context::new_context(el, &pf_reqs, &gl_attr)
            .map(|context| Context { context })
    }
}
