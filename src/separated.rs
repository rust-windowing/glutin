use super::*;

/// Represents an OpenGL context which has been associated with a preexisting
/// window.
///
/// # Example
///
/// ```no_run
/// # extern crate glutin;
/// # use glutin::ContextTrait;
/// # fn main() {
/// let mut el = glutin::EventsLoop::new();
/// let win = glutin::WindowBuilder::new().build(&el).unwrap();
/// let separated_context = glutin::ContextBuilder::new()
///     .build_separated(&win, &el)
///     .unwrap();
///
/// unsafe { separated_context.make_current().unwrap() };
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
///     separated_context.swap_buffers();
///     std::thread::sleep(std::time::Duration::from_millis(17));
/// }
/// # }
/// ```
pub struct SeparatedContext {
    context: Context,
}

impl SeparatedContext {
    /// Builds the GL context using the passed `Window`, returning the context
    /// as a `SeparatedContext`.
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
        window: &Window,
        cb: ContextBuilder,
        el: &EventsLoop,
    ) -> Result<Self, CreationError> {
        let ContextBuilder { pf_reqs, gl_attr } = cb;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);

        platform::Context::new_separated(window, el, &pf_reqs, &gl_attr).map(
            |context| SeparatedContext {
                context: Context { context },
            },
        )
    }

    /// Borrow the inner GL `Context`.
    pub fn context(&self) -> &Context {
        &self.context
    }

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

impl ContextTrait for SeparatedContext {
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

impl std::ops::Deref for SeparatedContext {
    type Target = Context;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
