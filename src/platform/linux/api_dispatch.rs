use libc;
use winit;
use wayland_client;

use ContextError;
use CreationError;
use EventsLoopClosed;
use GlAttributes;
use GlContext;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;

use super::wayland;
use super::x11;
use {ControlFlow, Event};

use winit::os::unix::WindowExt;

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;

pub struct Window {
    display_server: DisplayServer,
    winit_window: winit::Window,
}

pub enum EventsLoop {
    X(winit::EventsLoop),
    Wayland(wayland::EventsLoop)
}

impl EventsLoop {
    /// Builds a new events loop.
    pub fn new() -> EventsLoop {
        let winit_events_loop = winit::EventsLoop::new();
        // Ideally, winit would expose an API telling us wether we are
        // in Wayland mode or X11 mode
        if wayland_client::default_connect().is_ok() {
            EventsLoop::Wayland(wayland::EventsLoop::new(winit_events_loop))
        } else {
            EventsLoop::X(winit_events_loop)
        }
    }

    /// Fetches all the events that are pending, calls the callback function for each of them,
    /// and returns.
    #[inline]
    pub fn poll_events<F>(&self, callback: F)
        where F: FnMut(Event)
    {
        match *self {
            EventsLoop::X(ref evlp) => evlp.poll_events(callback),
            EventsLoop::Wayland(ref evlp) => evlp.poll_events(callback)
        }
    }

    /// Runs forever until `interrupt()` is called. Whenever an event happens, calls the callback.
    #[inline]
    pub fn run_forever<F>(&self, callback: F)
        where F: FnMut(Event) -> ControlFlow
    {
        match *self {
            EventsLoop::X(ref evlp) => evlp.run_forever(callback),
            EventsLoop::Wayland(ref evlp) => evlp.run_forever(callback)
        }
    }

    /// Creates an EventsLoopProxy that can be used to wake up the EventsLoop from another thread.
    #[inline]
    pub fn create_proxy(&self) -> EventsLoopProxy {
        match *self {
            EventsLoop::X(ref events_loop) => EventsLoopProxy::X(events_loop.create_proxy()),
            EventsLoop::Wayland(ref events_loop) => EventsLoopProxy::Wayland(events_loop.create_proxy()),
        }
    }
}

pub enum EventsLoopProxy {
    X(winit::EventsLoopProxy),
    Wayland(wayland::EventsLoopProxy),
}

impl EventsLoopProxy {
    /// Wake up the EventsLoop from which this proxy was created.
    ///
    /// This causes the EventsLoop to emit an Awakened event.
    ///
    /// Returns an Err if the associated EventsLoop no longer exists.
    #[inline]
    pub fn wakeup(&self) -> Result<(), EventsLoopClosed> {
        match *self {
            X(ref proxy) => self.proxy.wakeup(),
            Wayland(ref proxy) => self.proxy.wakeup(),
        }
    }
}

enum DisplayServer {
    X(x11::Window),
    Wayland(wayland::Window)
}

impl Window {
    #[inline]
    pub fn new(
        events_loop: &EventsLoop,
        _: &WindowAttributes,
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        _: &PlatformSpecificWindowBuilderAttributes,
        winit_builder: winit::WindowBuilder,
    ) -> Result<Window, CreationError> {
        let window = match *events_loop {
            EventsLoop::Wayland(ref evlp) => {
                let opengl = opengl.clone().map_sharing(|w| match w.display_server {
                    DisplayServer::Wayland(ref w) => w,
                    _ => panic!()       // TODO: return an error
                });
                let (display_server, winit_window) = try!(wayland::Window::new(
                    evlp,
                    pf_reqs,
                    &opengl,
                    winit_builder,
                ));
                Window {
                    display_server: DisplayServer::Wayland(display_server),
                    winit_window: winit_window,
                }
            },
            EventsLoop::X(ref evlp) => {
                let opengl = opengl.clone().map_sharing(|w| match w.display_server {
                    DisplayServer::X(ref w) => w,
                    _ => panic!()       // TODO: return an error
                });
                let (display_server, winit_window) = try!(x11::Window::new(
                    evlp,
                    pf_reqs,
                    &opengl,
                    winit_builder,
                ));
                Window {
                    display_server: DisplayServer::X(display_server),
                    winit_window: winit_window,
                }
            },
        };
        Ok(window)
    }

    pub fn id(&self) -> winit::WindowId {
        self.winit_window.id()
    }

    pub fn set_title(&self, title: &str) {
        self.winit_window.set_title(title)
    }

    pub fn show(&self) {
        self.winit_window.show()
    }

    pub fn hide(&self) {
        self.winit_window.hide()
    }

    pub fn get_position(&self) -> Option<(i32, i32)> {
        self.winit_window.get_position()
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.winit_window.set_position(x, y)
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_points(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_pixels(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size().map(|(x, y)| {
            let hidpi = self.hidpi_factor();
            ((x as f32 * hidpi) as u32, (y as f32 * hidpi) as u32)
        })
    }

    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_outer_size()
    }

    pub fn set_inner_size(&self, x: u32, y: u32) {
        match self.display_server {
            DisplayServer::X(_) => self.winit_window.set_inner_size(x, y),
            DisplayServer::Wayland(ref w) => w.set_inner_size(x, y, &self.winit_window)
        }
    }

    pub unsafe fn platform_display(&self) -> *mut libc::c_void {
        self.winit_window.platform_display()
    }

    pub unsafe fn platform_window(&self) -> *mut libc::c_void {
        self.winit_window.platform_window()
    }

    #[inline]
    pub fn as_winit_window(&self) -> &winit::Window {
        &self.winit_window
    }

    #[inline]
    pub fn as_winit_window_mut(&mut self) -> &mut winit::Window {
        &mut self.winit_window
    }

    pub fn set_cursor(&self, cursor: winit::MouseCursor) {
        self.winit_window.set_cursor(cursor);
    }

    pub fn hidpi_factor(&self) -> f32 {
        self.winit_window.hidpi_factor()
    }

    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        self.winit_window.set_cursor_position(x, y)
    }

    pub fn set_cursor_state(&self, state: winit::CursorState) -> Result<(), String> {
        self.winit_window.set_cursor_state(state)
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self.display_server {
            DisplayServer::X(ref w) => w.make_current(),
            DisplayServer::Wayland(ref w) => w.make_current()
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self.display_server {
            DisplayServer::X(ref w) => w.is_current(),
            DisplayServer::Wayland(ref w) => w.is_current()
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self.display_server {
            DisplayServer::X(ref w) => w.get_proc_address(addr),
            DisplayServer::Wayland(ref w) => w.get_proc_address(addr)
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self.display_server {
            DisplayServer::X(ref w) => w.swap_buffers(),
            DisplayServer::Wayland(ref w) => w.swap_buffers()
        }
    }

    #[inline]
    fn get_api(&self) -> ::Api {
        match self.display_server {
            DisplayServer::X(ref w) => w.get_api(),
            DisplayServer::Wayland(ref w) => w.get_api()
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self.display_server {
            DisplayServer::X(ref w) => w.get_pixel_format(),
            DisplayServer::Wayland(ref w) => w.get_pixel_format()
        }
    }
}
