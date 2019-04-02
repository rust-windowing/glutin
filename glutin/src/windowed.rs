use super::*;

use std::marker::PhantomData;

/// Represents an OpenGL context and the `Window` with which it is associated.
///
/// # Example
///
/// ```no_run
/// # use glutin::ContextTrait;
/// # fn main() {
/// let mut el = glutin::EventsLoop::new();
/// let wb = glutin::WindowBuilder::new();
/// let windowed_context = glutin::ContextBuilder::new()
///     .build_windowed(wb, &el)
///     .unwrap();
///
/// let windowed_context = unsafe { windowed_context.make_current().unwrap() };
///
/// loop {
///     el.poll_events(|event| {
///         match event {
///             // process events here
///             _ => (),
///         }
///     });
///
///     // draw everything here
///
///     windowed_context.swap_buffers();
///     std::thread::sleep(std::time::Duration::from_millis(17));
/// }
/// # }
/// ```
pub type WindowedContext<T> = ContextWrapper<T, Window>;

/// Represents a raw OpenGL context.
pub type RawContext<T> = ContextWrapper<T, ()>;

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

    /// Split the Window apart from the OpenGL context. Should only be used
    /// when intending to transfer the Context to an other thread.
    ///
    /// Unsaftey:
    ///   - The OpenGL context must be dropped before the window.
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

impl<T: ContextCurrentState, W> ContextWrapper<T, W> {
    /// Borrow the inner GL `Context`.
    pub fn context(&self) -> &Context<T> {
        &self.context
    }
}

impl<W> ContextWrapper<PossiblyCurrentContext, W> {
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

    /// Returns the pixel format of the main framebuffer of the context.
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.context.context.get_pixel_format()
    }

    /// Resize the context.
    ///
    /// Some platforms (macOS, Wayland) require being manually updated when
    /// their window or surface is resized.
    ///
    /// The easiest way of doing this is to take every `Resized` window event
    /// that is received with a `LogicalSize` and convert it to a
    /// `PhysicalSize` and pass it into this function.
    pub fn resize(&self, size: dpi::PhysicalSize) {
        let (width, height) = size.into();
        self.context.context.resize(width, height);
    }
}

impl<T: ContextCurrentState, W> ContextTrait for ContextWrapper<T, W> {
    type PossiblyCurrentContext = ContextWrapper<PossiblyCurrentContext, W>;
    type NotCurrentContext = ContextWrapper<NotCurrentContext, W>;

    unsafe fn make_current(
        self,
    ) -> Result<Self::PossiblyCurrentContext, (Self, ContextError)> {
        let window = self.window;
        match self.context.make_current() {
            Ok(context) => Ok(ContextWrapper { window, context }),
            Err((context, err)) => {
                Err((ContextWrapper { window, context }, err))
            }
        }
    }

    unsafe fn make_not_current(
        self,
    ) -> Result<Self::NotCurrentContext, (Self, ContextError)> {
        let window = self.window;
        match self.context.make_not_current() {
            Ok(context) => Ok(ContextWrapper { window, context }),
            Err((context, err)) => {
                Err((ContextWrapper { window, context }, err))
            }
        }
    }

    unsafe fn treat_as_not_current(self) -> Self::NotCurrentContext {
        ContextWrapper {
            context: self.context.treat_as_not_current(),
            window: self.window,
        }
    }

    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    fn get_api(&self) -> Api {
        self.context.get_api()
    }
}

impl<W> PossiblyCurrentContextTrait
    for ContextWrapper<PossiblyCurrentContext, W>
{
    fn get_proc_address(&self, addr: &str) -> *const () {
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
    /// the pair as a `WindowedContext`.
    ///
    /// One notable limitation of the Wayland backend when it comes to shared
    /// contexts is that both contexts must use the same events loop.
    ///
    /// Errors can occur in two scenarios:
    ///  - If the window could not be created (via permission denied,
    ///  incompatible system, out of memory, etc.). This should be very rare.
    ///  - If the OpenGL context could not be created. This generally happens
    ///  because the underlying platform doesn't support a requested feature.
    pub fn build_windowed(
        self,
        wb: WindowBuilder,
        el: &EventsLoop,
    ) -> Result<WindowedContext<NotCurrentContext>, CreationError> {
        let ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        platform::Context::new_windowed(wb, el, &pf_reqs, &gl_attr).map(
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
