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
pub struct Context<T: ContextCurrentState> {
    pub(crate) context: platform::Context,
    pub(crate) phantom: PhantomData<T>,
}

/// A trait for types associated with a GL context.
pub trait ContextTrait
where
    Self: Sized,
{
    type PossiblyCurrentContext: PossiblyCurrentContextTrait
        + ContextTrait<
            PossiblyCurrentContext = Self::PossiblyCurrentContext,
            NotCurrentContext = Self::NotCurrentContext,
        >;
    type NotCurrentContext: ContextTrait<
        PossiblyCurrentContext = Self::PossiblyCurrentContext,
        NotCurrentContext = Self::NotCurrentContext,
    >;

    /// Sets the context as the current context. The previously current context
    /// (if any) is no longer current.
    unsafe fn make_current(
        self,
    ) -> Result<Self::PossiblyCurrentContext, (Self, ContextError)>;

    /// If this context is current, makes the context not current.
    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotCurrentContext, (Self, ContextError)>;

    /// Treats the context as not current, even if it is current. We do no
    /// checks to confirm that this is the case. Prefer to use
    /// `make_not_current` which will do nothing if this context is not current.
    unsafe fn treat_as_not_current(self) -> Self::NotCurrentContext;

    /// Returns true if this context is the current one in this thread.
    fn is_current(&self) -> bool;

    /// Returns the OpenGL API being used.
    fn get_api(&self) -> Api;
}

pub trait PossiblyCurrentContextTrait {
    /// Returns the address of an OpenGL function.
    fn get_proc_address(&self, addr: &str) -> *const ();
}

impl<T: ContextCurrentState> ContextTrait for Context<T> {
    type PossiblyCurrentContext = Context<PossiblyCurrentContext>;
    type NotCurrentContext = Context<NotCurrentContext>;

    unsafe fn make_current(
        self,
    ) -> Result<Self::PossiblyCurrentContext, (Self, ContextError)> {
        match self.context.make_current() {
            Ok(()) => Ok(Context {
                context: self.context,
                phantom: PhantomData,
            }),
            Err(err) => Err((
                Context {
                    context: self.context,
                    phantom: PhantomData,
                },
                err,
            )),
        }
    }

    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotCurrentContext, (Self, ContextError)> {
        match self.context.make_not_current() {
            Ok(()) => Ok(Context {
                context: self.context,
                phantom: PhantomData,
            }),
            Err(err) => Err((
                Context {
                    context: self.context,
                    phantom: PhantomData,
                },
                err,
            )),
        }
    }

    unsafe fn treat_as_not_current(self) -> Self::NotCurrentContext {
        Context {
            context: self.context,
            phantom: PhantomData,
        }
    }

    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    fn get_api(&self) -> Api {
        self.context.get_api()
    }
}

impl PossiblyCurrentContextTrait for Context<PossiblyCurrentContext> {
    fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }
}

impl<'a, T: ContextCurrentState> ContextBuilder<'a, T> {
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
        platform::Context::new_headless(el, &pf_reqs, &gl_attr, dims).map(
            |context| Context {
                context,
                phantom: PhantomData,
            },
        )
    }
}

// This is nightly only:
// impl !Send for Context<PossiblyCurrentContext> {}
// impl !Sync for Context<PossiblyCurrentContext> {}
//
// Instead we add a phantom type to PossiblyCurrentContext

#[derive(Debug)]
pub struct PossiblyCurrentContext {
    phantom: PhantomData<*mut ()>,
}

#[derive(Debug)]
pub enum NotCurrentContext {}

pub trait ContextCurrentState: std::fmt::Debug {}

impl ContextCurrentState for PossiblyCurrentContext {}
impl ContextCurrentState for NotCurrentContext {}

trait FailToCompileIfNotSendSync
where
    Self: Send + Sync,
{
}
impl FailToCompileIfNotSendSync for Context<NotCurrentContext> {}
