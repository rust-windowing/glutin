use libc;
use winit;

use ContextError;
use CreationError;
use GlAttributes;
use GlContext;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;

use api::wayland;
use api::x11;

use winit::os::unix::WindowExt;

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;

pub struct Window {
    display_server: DisplayServer,
    winit_window: winit::Window,
}

enum DisplayServer {
    X(x11::Window),
    Wayland(wayland::Window)
}

impl Window {
    #[inline]
    pub fn new(
        _: &WindowAttributes,
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        _: &PlatformSpecificWindowBuilderAttributes,
        winit_builder: winit::WindowBuilder,
    ) -> Result<Window, CreationError> {
        let winit_window = winit_builder.build().unwrap();
        let is_x11 = winit_window.get_xlib_display().is_some();
        let display_server = if is_x11 {
            let opengl = opengl.clone().map_sharing(|w| match w.display_server {
                DisplayServer::X(ref w) => w,
                _ => panic!()       // TODO: return an error
            });
            DisplayServer::X(try!(x11::Window::new(
                pf_reqs,
                &opengl,
                &winit_window,
            )))
        } else {
            let opengl = opengl.clone().map_sharing(|w| match w.display_server {
                DisplayServer::Wayland(ref w) => w,
                _ => panic!()       // TODO: return an error
            });
            DisplayServer::Wayland(try!(wayland::Window::new(
                pf_reqs,
                &opengl,
                &winit_window,
            )))
        };
        Ok(Window {
            display_server: display_server,
            winit_window: winit_window,
        })
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
        self.winit_window.set_inner_size(x, y)
    }

    pub fn poll_events(&self) -> winit::PollEventsIterator {
        self.winit_window.poll_events()
    }

    pub fn wait_events(&self) -> winit::WaitEventsIterator {
        self.winit_window.wait_events()
    }

    pub unsafe fn platform_display(&self) -> *mut libc::c_void {
        self.winit_window.platform_display()
    }

    pub unsafe fn platform_window(&self) -> *mut libc::c_void {
        self.winit_window.platform_window()
    }

    pub fn create_window_proxy(&self) -> winit::WindowProxy {
        self.winit_window.create_window_proxy()
    }

    pub fn set_window_resize_callback(&mut self, callback: Option<fn(u32, u32)>) {
        self.winit_window.set_window_resize_callback(callback);
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
