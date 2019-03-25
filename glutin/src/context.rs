use super::*;

use std::marker::PhantomData;

/// Represents an OpenGL context.
///
/// A `Context` is normally associated with a single Window, however `Context`s
/// can be *shared* between multiple windows or be headless.
///
/// # Example
///
/// ```no_run
/// # use glutin::ContextTrait;
/// # fn main() {
/// # let el = glutin::EventsLoop::new();
/// # let wb = glutin::WindowBuilder::new();
/// # let some_context = glutin::ContextBuilder::new()
/// #    .build_windowed(wb, &el)
/// #    .unwrap();
/// let cb = glutin::ContextBuilder::new()
///     .with_vsync(true)
///     .with_multisampling(8)
///     .with_shared_lists(some_context.context());
/// # }
/// ```
#[derive(Debug)]
pub struct Context<T> {
    pub(crate) context: platform::Context<T>,
    phantom: PhantomData<T>,
}

#[derive(Debug)]
pub enum CurrentContext {}

#[derive(Debug)]
pub enum NotCurrentContext {}

impl<T> ContextTrait for Context<T> {
    type CurrentContext = Context<CurrentContext>;
    type NotCurrentContext = Context<NotCurrentContext>;

    unsafe fn make_current(self) -> Result<Self::CurrentContext, (Self, ContextError)> {
        self.context.make_current()
            .map(|context| Context { context, phantom: PhantomData })
            .map_err(|(context, err)| (Context { context, phantom: PhantomData }, err))
    }

    unsafe fn make_not_current(self) -> Result<Self::NotCurrentContext, (Self, ContextError)> {
        self.context.make_not_current()
            .map(|context| Context { context, phantom: PhantomData })
            .map_err(|(context, err)| (Context { context, phantom: PhantomData }, err))
    }

    unsafe fn treat_as_not_current(self) -> Self::NotCurrentContext {
        Context {
            context: self.context.treat_as_not_current(),
            phantom: PhantomData,
        }
    }

    fn is_current(&self) -> bool {
        self.context.is_current()
    }
}

impl CurrentContextTrait for Context<CurrentContext> {
    fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    fn get_api(&self) -> Api {
        self.context.get_api()
    }
}

impl<'a, T> ContextBuilder<'a, T> {
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
    pub fn build_headless(
        self,
        el: &EventsLoop,
        dims: dpi::PhysicalSize,
    ) -> Result<Context<NotCurrentContext>, CreationError> {
        let ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        platform::Context::new_headless(el, &pf_reqs, &gl_attr, dims)
            .map(|context| Context { context, phantom: PhantomData })
    }
}
