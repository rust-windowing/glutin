#![cfg(target_os = "windows")]

use libc;
use std::ptr;

use winit;

use ContextError;
use CreationError;
use GlAttributes;
use GlContext;
use GlRequest;
use Api;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;

use winapi;

use api::wgl::Context as WglContext;
use api::egl::Context as EglContext;
use api::egl::ffi::egl::Egl;
use api::egl;

/// The Win32 implementation of the main `Window` object.
pub struct Window {
    context: Context,
    winit_window: winit::Window,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

enum Context {
    Egl(EglContext),
    Wgl(WglContext),
}

impl Window {
    /// See the docs in the crate root file.
    pub fn new(_: &WindowAttributes,
               pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&Window>,
               egl: Option<&Egl>,
               winit_builder: winit::WindowBuilder)
               -> Result<Window, CreationError> {
        let winit_window = winit_builder.build().unwrap();
        let opengl = opengl.clone().map_sharing(|sharing| {
            match sharing.context {
                Context::Wgl(ref c) => c.get_hglrc(),
                Context::Egl(_) => unimplemented!(),        // FIXME:
            }
        });
        let context = unsafe {
            let w = winit_window.platform_window() as winapi::HWND;
            match opengl.version {
                GlRequest::Specific(Api::OpenGlEs, (_major, _minor)) => {
                    if let Some(egl) = egl {
                        if let Ok(c) =
                               EglContext::new(egl.clone(),
                                               &pf_reqs,
                                               &opengl.clone().map_sharing(|_| unimplemented!()),
                                               egl::NativeDisplay::Other(Some(ptr::null())))
                            .and_then(|p| p.finish(w)) {
                            Context::Egl(c)
                        } else {
                            try!(WglContext::new(&pf_reqs, &opengl, w).map(Context::Wgl))
                        }

                    } else {
                        // falling back to WGL, which is always available
                        try!(WglContext::new(&pf_reqs, &opengl, w).map(Context::Wgl))
                    }
                }
                _ => try!(WglContext::new(&pf_reqs, &opengl, w).map(Context::Wgl)),
            }
        };
        Ok(Window {
            context: context,
            winit_window: winit_window,
        })
    }

    pub fn set_title(&self, title: &str) {
        self.winit_window.set_title(title)
    }

    #[inline]
    pub fn to_winit_window(self) -> winit::Window {
        self.winit_window
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
        match self.context {
            Context::Wgl(ref c) => c.make_current(),
            Context::Egl(ref c) => c.make_current(),
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self.context {
            Context::Wgl(ref c) => c.is_current(),
            Context::Egl(ref c) => c.is_current(),
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self.context {
            Context::Wgl(ref c) => c.get_proc_address(addr),
            Context::Egl(ref c) => c.get_proc_address(addr),
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self.context {
            Context::Wgl(ref c) => c.swap_buffers(),
            Context::Egl(ref c) => c.swap_buffers(),
        }
    }

    #[inline]
    fn get_api(&self) -> Api {
        match self.context {
            Context::Wgl(ref c) => c.get_api(),
            Context::Egl(ref c) => c.get_api(),
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self.context {
            Context::Wgl(ref c) => c.get_pixel_format(),
            Context::Egl(ref c) => c.get_pixel_format(),
        }
    }
}