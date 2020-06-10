use super::*;

use std::marker::PhantomData;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

/// Represents an OpenGL [`Context`] and the [`Window`] with which it is
/// associated.
///
/// Please see [`ContextWrapper<T, Window>`].
///
/// # Example
///
/// ```no_run
/// # fn main() {
/// let mut el = glutin::event_loop::EventLoop::new();
/// let wb = glutin::window::WindowBuilder::new();
/// let windowed_context = glutin::ContextBuilder::new()
///     .build_windowed(wb, &el)
///     .unwrap();
///
/// let windowed_context = unsafe { windowed_context.make_current().unwrap() };
/// # }
/// ```
///
/// [`ContextWrapper<T, Window>`]: struct.ContextWrapper.html
/// [`Window`]: struct.Window.html
/// [`Context`]: struct.Context.html
pub type WindowedContext<T> = ContextWrapper<T, Window>;

/// Represents an OpenGL [`Context`] which has an underlying window that is
/// stored separately.
///
/// This type can only be created via one of three ways:
///
///  * [`platform::unix::RawContextExt`]
///  * [`platform::windows::RawContextExt`]
///  * [`WindowedContext<T>::split`]
///
/// Please see [`ContextWrapper<T, ()>`].
///
/// [`ContextWrapper<T, ()>`]: struct.ContextWrapper.html
/// [`WindowedContext<T>::split`]: type.WindowedContext.html#method.split
/// [`Context`]: struct.Context.html
#[cfg_attr(
    target_os = "windows",
    doc = "\
[`platform::windows::RawContextExt`]: os/windows/enum.RawHandle.html
"
)]
#[cfg_attr(
    not(target_os = "windows",),
    doc = "\
[`platform::windows::RawContextExt`]: os/index.html
"
)]
#[cfg_attr(
    not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    )),
    doc = "\
[`platform::unix::RawContextExt`]: os/index.html
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
[`platform::unix::RawContextExt`]: os/unix/enum.RawHandle.html
"
)]
pub type RawContext<T> = ContextWrapper<T, ()>;

/// A context which has an underlying window, which may or may not be stored
/// separately.
///
/// If the window is stored separately, it is a [`RawContext<T>`]. Otherwise,
/// it is a [`WindowedContext<T>`].
///
/// [`WindowedContext<T>`]: type.WindowedContext.html
/// [`RawContext<T>`]: type.RawContext.html
/// [`Context`]: struct.Context.html
#[derive(Debug)]
pub struct ContextWrapper<T: ContextCurrentState, W> {
    pub(crate) context: Context<T>,
    pub(crate) window: W,
}

impl<T: ContextCurrentState> WindowedContext<T> {
    /// Borrow the inner `W`.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Split the [`Window`] apart from the OpenGL [`Context`]. Should only be
    /// used when intending to transfer the [`RawContext<T>`] to another
    /// thread.
    ///
    /// Unsaftey:
    ///   - The OpenGL [`Context`] must be dropped before the [`Window`].
    ///
    /// [`RawContext<T>`]: type.RawContext.html
    /// [`Window`]: struct.Window.html
    /// [`Context`]: struct.Context.html
    pub unsafe fn split(self) -> (RawContext<T>, Window) {
        (
            RawContext {
                context: self.context,
                window: (),
            },
            self.window,
        )
    }
}

impl<W> ContextWrapper<PossiblyCurrent, W> {
    /// Swaps the buffers in case of double or triple buffering.
    ///
    /// You should call this function every time you have finished rendering, or
    /// the image may not be displayed on the screen.
    ///
    /// **Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.context.context.swap_buffers()
    }

    /// Swaps the buffers in case of double or triple buffering using specified
    /// damage rects.
    ///
    /// You should call this function every time you have finished rendering, or
    /// the image may not be displayed on the screen.
    ///
    /// **Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        self.context.context.swap_buffers_with_damage(rects)
    }

    /// Returns whether or not swap_buffer_with_damage is available. If this
    /// function returns false, any call to swap_buffers_with_damage will
    /// return an error.
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        self.context.context.swap_buffers_with_damage_supported()
    }

    /// Returns the pixel format of the main framebuffer of the context.
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.context.context.get_pixel_format()
    }

    /// Resize the context.
    ///
    /// Some platforms (macOS, Wayland) require being manually updated when
    /// their window or surface is resized.
    ///
    /// The easiest way of doing this is to take every [`Resized`] window event
    /// that is received and pass its [`PhysicalSize`] into this function.
    ///
    /// [`PhysicalSize`]: dpi/struct.PhysicalSize.html
    /// [`Resized`]: event/enum.WindowEvent.html#variant.Resized
    pub fn resize(&self, size: dpi::PhysicalSize<u32>) {
        let (width, height) = size.into();
        self.context.context.resize(width, height);
    }
}

impl<T: ContextCurrentState, W> ContextWrapper<T, W> {
    /// Borrow the inner GL [`Context`].
    ///
    /// [`Context`]: struct.Context.html
    pub fn context(&self) -> &Context<T> {
        &self.context
    }

    /// Sets this context as the current context. The previously current context
    /// (if any) is no longer current.
    ///
    /// A failed call to `make_current` might make this, or no context
    /// current. It could also keep the previous context current. What happens
    /// varies by platform and error.
    ///
    /// To attempt to recover and get back into a know state, either:
    ///
    ///  * attempt to use [`is_current`] to find the new current context; or
    ///  * call [`make_not_current`] on both the previously
    ///  current context and this context.
    ///
    /// # An higher level overview.
    ///
    /// In OpenGl, only a single context can be current in a thread at a time.
    /// Making a new context current will make the old one not current.
    /// Contexts can only be sent to different threads if they are not current.
    ///
    /// If you call `make_current` on some context, you should call
    /// [`treat_as_not_current`] as soon as possible on the previously current
    /// context.
    ///
    /// If you wish to move a currently current context to a different thread,
    /// you should do one of two options:
    ///
    ///  * Call `make_current` on another context, then call
    ///  [`treat_as_not_current`] on this context.
    ///  * Call [`make_not_current`] on this context.
    ///
    /// If you are aware of what context you intend to make current next, it is
    /// preferable for performance reasons to call `make_current` on that
    /// context, then [`treat_as_not_current`] on this context.
    ///
    /// If you are not aware of what context you intend to make current next,
    /// consider waiting until you do. If you need this context not current
    /// immediately (e.g. to transfer it to another thread), then call
    /// [`make_not_current`] on this context.
    ///
    /// Please avoid calling [`make_not_current`] on one context only to call
    /// `make_current` on another context before and/or after. This hurts
    /// performance by requiring glutin to:
    ///
    ///  * Check if this context is current; then
    ///  * If it is, change the current context from this context to none; then
    ///  * Change the current context from none to the new context.
    ///
    /// Instead prefer the method we mentioned above with `make_current` and
    /// [`treat_as_not_current`].
    ///
    /// [`make_not_current`]: struct.ContextWrapper.html#method.make_not_current
    /// [`treat_as_not_current`]:
    /// struct.ContextWrapper.html#method.treat_as_not_current
    /// [`is_current`]: struct.ContextWrapper.html#method.is_current
    pub unsafe fn make_current(
        self,
    ) -> Result<ContextWrapper<PossiblyCurrent, W>, (Self, ContextError)> {
        let window = self.window;
        match self.context.make_current() {
            Ok(context) => Ok(ContextWrapper { window, context }),
            Err((context, err)) => {
                Err((ContextWrapper { window, context }, err))
            }
        }
    }

    /// If this context is current, makes this context not current. If this
    /// context is not current however, this function does nothing.
    ///
    /// Please see [`make_current`].
    ///
    /// [`make_current`]: struct.ContextWrapper.html#method.make_current
    pub unsafe fn make_not_current(
        self,
    ) -> Result<ContextWrapper<NotCurrent, W>, (Self, ContextError)> {
        let window = self.window;
        match self.context.make_not_current() {
            Ok(context) => Ok(ContextWrapper { window, context }),
            Err((context, err)) => {
                Err((ContextWrapper { window, context }, err))
            }
        }
    }

    /// Treats this context as not current, even if it is current. We do no
    /// checks to confirm that this is actually case.
    ///
    /// If unsure whether or not this context is current, please use
    /// [`make_not_current`] which will do nothing if this context is not
    /// current.
    ///
    /// Please see [`make_current`].
    ///
    /// [`make_not_current`]: struct.ContextWrapper.html#method.make_not_current
    /// [`make_current`]: struct.ContextWrapper.html#method.make_current
    pub unsafe fn treat_as_not_current(self) -> ContextWrapper<NotCurrent, W> {
        ContextWrapper {
            context: self.context.treat_as_not_current(),
            window: self.window,
        }
    }

    /// Treats this context as current, even if it is not current. We do no
    /// checks to confirm that this is actually case.
    ///
    /// This function should only be used if you intend to track context
    /// currency without the limited aid of glutin, and you wish to store
    /// all the [`Context`]s as [`NotCurrent`].
    ///
    /// Please see [`make_current`] for the prefered method of handling context
    /// currency.
    ///
    /// [`make_current`]: struct.ContextWrapper.html#method.make_current
    /// [`NotCurrent`]: enum.NotCurrent.html
    /// [`Context`]: struct.Context.html
    pub unsafe fn treat_as_current(self) -> ContextWrapper<PossiblyCurrent, W> {
        ContextWrapper {
            context: self.context.treat_as_current(),
            window: self.window,
        }
    }

    /// Returns true if this context is the current one in this thread.
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    /// Returns the OpenGL API being used.
    pub fn get_api(&self) -> Api {
        self.context.get_api()
    }
}

impl<W> ContextWrapper<PossiblyCurrent, W> {
    /// Returns the address of an OpenGL function.
    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        self.context.get_proc_address(addr)
    }
}

impl<T: ContextCurrentState, W> std::ops::Deref for ContextWrapper<T, W> {
    type Target = Context<T>;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<'a, T: ContextCurrentState> ContextBuilder<'a, T> {
    /// Builds the given window along with the associated GL context, returning
    /// the pair as a [`WindowedContext<T>`].
    ///
    /// Errors can occur in two scenarios:
    ///  - If the window could not be created (via permission denied,
    ///  incompatible system, out of memory, etc.). This should be very rare.
    ///  - If the OpenGL [`Context`] could not be created. This generally
    ///    happens
    ///  because the underlying platform doesn't support a requested feature.
    ///
    /// [`WindowedContext<T>`]: type.WindowedContext.html
    /// [`Context`]: struct.Context.html
    pub fn build_windowed<TE>(
        self,
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<TE>,
    ) -> Result<WindowedContext<NotCurrent>, CreationError> {
        let ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        platform_impl::Context::new_windowed(wb, el, &pf_reqs, &gl_attr).map(
            |(window, context)| WindowedContext {
                window,
                context: Context {
                    context,
                    phantom: PhantomData,
                },
            },
        )
    }
}
