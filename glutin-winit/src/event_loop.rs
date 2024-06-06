use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use winit::error::OsError;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

use crate::private::Sealed;

/// [`ActiveEventLoop`] is the recommended way to interact with the event
/// loop, but for compatibility purposes [`EventLoop`] is also supported
/// although not recommended anymore as it has been deprecated by Winit.
pub trait GlutinEventLoop: Sealed {
    /// Create the window.
    ///
    /// See [`ActiveEventLoop::create_window`] for details.
    fn create_window(&self, window_attributes: WindowAttributes) -> Result<Window, OsError>;

    /// Get a handle to the display controller of the windowing system.
    fn glutin_display_handle(&self) -> Result<DisplayHandle<'_>, HandleError>;
}

impl Sealed for ActiveEventLoop {}

impl GlutinEventLoop for ActiveEventLoop {
    fn create_window(&self, window_attributes: WindowAttributes) -> Result<Window, OsError> {
        self.create_window(window_attributes)
    }

    fn glutin_display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.display_handle()
    }
}

impl<T> Sealed for EventLoop<T> {}

impl<T> GlutinEventLoop for EventLoop<T> {
    #[allow(deprecated)]
    fn create_window(&self, window_attributes: WindowAttributes) -> Result<Window, OsError> {
        self.create_window(window_attributes)
    }

    fn glutin_display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.display_handle()
    }
}
