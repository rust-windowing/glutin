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

impl<T: ContextCurrentState> Context<T> {
    /// See [`ContextWrapper::make_current`].
    ///
    /// [`ContextWrapper::make_current`]:
    /// struct.ContextWrapper.html#method.make_current
    pub unsafe fn make_current(
        self,
    ) -> Result<Context<PossiblyCurrentContext>, (Self, ContextError)> {
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

    /// See [`ContextWrapper::make_not_current`].
    ///
    /// [`ContextWrapper::make_not_current`]:
    /// struct.ContextWrapper.html#method.make_not_current
    pub unsafe fn make_not_current(
        self,
    ) -> Result<Context<NotCurrentContext>, (Self, ContextError)> {
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

    /// See [`ContextWrapper::treat_as_not_current`].
    ///
    /// [`ContextWrapper::treat_as_not_current`]:
    /// struct.ContextWrapper.html#method.treat_as_not_current
    pub unsafe fn treat_as_not_current(self) -> Context<NotCurrentContext> {
        Context {
            context: self.context,
            phantom: PhantomData,
        }
    }

    /// See [`ContextWrapper::is_current`].
    ///
    /// [`ContextWrapper::is_current`]:
    /// struct.ContextWrapper.html#method.is_current
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    /// See [`ContextWrapper::get_api`].
    ///
    /// [`ContextWrapper::get_api`]: struct.ContextWrapper.html#method.get_api
    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }
}

impl Context<PossiblyCurrentContext> {
    /// See [`ContextWrapper::get_proc_address`].
    ///
    /// [`ContextWrapper::get_proc_address`]:
    /// struct.ContextWrapper.html#method.get_proc_address
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }
}

impl<'a, T: ContextCurrentState> ContextBuilder<'a, T> {
    /// Builds the given GL context.
    ///
    /// One notable limitation of the Wayland backend when it comes to shared
    /// contexts is that both contexts must use the same events loop.
    ///
    /// When on linux, prefer [`build_surfaceless`]. If both
    /// [`build_surfaceless`] and `build_headless` fail, try using a hidden
    /// window, or [`build_osmesa`]. Please note that if you choose to use a
    /// hidden window, you must still handle the events it generates on the
    /// events loop.
    ///
    /// Errors can occur in two scenarios:
    ///  - If the window could not be created (via permission denied,
    ///  incompatible system, out of memory, etc.). This should be very rare.
    ///  - If the OpenGL context could not be created. This generally happens
    ///  because the underlying platform doesn't support a requested feature.
    #[cfg_attr(
        not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        )),
        doc = "\
    [`build_surfaceless`]: os/index.html
    [`build_osmesa`]: os/index.html
    "
    )]
    #[cfg_attr(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ),
        doc = "\
    [`build_surfaceless`]: os/unix/trait.HeadlessContextExt.html#tymethod.build_surfaceless
    [`build_osmesa`]: os/unix/trait.HeadlessContextExt.html#tymethod.build_osmesa
    "
    )]
    pub fn build_headless(
        self,
        el: &EventsLoop,
        size: dpi::PhysicalSize,
    ) -> Result<Context<NotCurrentContext>, CreationError> {
        let ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        platform::Context::new_headless(el, &pf_reqs, &gl_attr, size).map(
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

/// A type that contexts which might possibly be currently current on some
/// thread take as a generic.
///
/// See [`ContextWrapper::make_current`] for more details.
///
/// [`ContextWrapper::make_current`]:
/// struct.ContextWrapper.html#method.make_current
#[derive(Debug, Clone, Copy)]
pub struct PossiblyCurrentContext {
    phantom: PhantomData<*mut ()>,
}

/// A type that contexts which are not currently current on any thread take as
/// a generic.
///
/// See [`ContextWrapper::make_current`] for more details.
///
/// [`ContextWrapper::make_current`]:
/// struct.ContextWrapper.html#method.make_current
#[derive(Debug, Clone, Copy)]
pub enum NotCurrentContext {}

/// A trait implemented on both [`NotCurrentContext`] and
/// [`PossiblyCurrentContext`].
///
/// [`NotCurrentContext`]: enum.NotCurrentContext.html
/// [`PossiblyCurrentContext`]: struct.PossiblyCurrentContext.html
pub trait ContextCurrentState: std::fmt::Debug + Clone {}

impl ContextCurrentState for PossiblyCurrentContext {}
impl ContextCurrentState for NotCurrentContext {}

trait FailToCompileIfNotSendSync
where
    Self: Send + Sync,
{
}
impl FailToCompileIfNotSendSync for Context<NotCurrentContext> {}
