use super::*;

/// Represents an OpenGL context and the `Window` with which it is associated.
///
/// # Example
///
/// ```no_run
/// # extern crate glutin;
/// # use glutin::ContextTrait;
/// # fn main() {
/// let mut el = glutin::EventsLoop::new();
/// let wb = glutin::WindowBuilder::new();
/// let combined_context = glutin::ContextBuilder::new()
///     .build_combined(wb, &el)
///     .unwrap();
///
/// unsafe { combined_context.make_current().unwrap() };
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
///     combined_context.swap_buffers();
///     std::thread::sleep(std::time::Duration::from_millis(17));
/// }
/// # }
/// ```
pub struct CombinedContext {
    context: Context,
    window: Window,
}

impl CombinedContext {
    /// Builds the given window along with the associated GL context, returning
    /// the pair as a `CombinedContext`.
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
        wb: WindowBuilder,
        cb: ContextBuilder,
        el: &EventsLoop,
    ) -> Result<Self, CreationError> {
        let ContextBuilder { pf_reqs, gl_attr } = cb;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        platform::Context::new(wb, el, &pf_reqs, &gl_attr).map(
            |(window, context)| CombinedContext {
                window,
                context: Context { context },
            },
        )
    }

    /// Borrow the inner `Window`.
    pub fn window(&self) -> &Window {
        &self.window
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

impl ContextTrait for CombinedContext {
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

impl std::ops::Deref for CombinedContext {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}
