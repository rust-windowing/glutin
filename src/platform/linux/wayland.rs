use std::ffi::CString;
use winit;
use winit::os::unix::WindowExt;
use {ContextError, CreationError, GlAttributes, GlContext, PixelFormat, PixelFormatRequirements};
use api::dlopen;
use api::egl;
use api::egl::Context as EglContext;
use wayland_client::egl as wegl;
use Event;

pub struct Window {
    egl_surface: wegl::WlEglSurface,
    context: EglContext,
}

pub struct WaitEventsIterator<'a> {
    window: &'a Window,
    winit_iterator: winit::WaitEventsIterator<'a>,
}

impl<'a> Iterator for WaitEventsIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        let event = self.winit_iterator.next();
        match event {
            Some(Event::Resized(x, y)) => self.window.egl_surface.resize(x as i32, y as i32, 0, 0),
            _ => {},
        }
        event
    }
}

pub struct PollEventsIterator<'a> {
    window: &'a Window,
    winit_iterator: winit::PollEventsIterator<'a>,
}

impl<'a> Iterator for PollEventsIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        let event = self.winit_iterator.next();
        match event {
            Some(Event::Resized(x, y)) => self.window.egl_surface.resize(x as i32, y as i32, 0, 0),
            _ => {},
        }
        event
    }
}

impl Window {
    pub fn new(
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        winit_builder: winit::WindowBuilder,
    ) -> Result<(Window, winit::Window), CreationError> {
        let winit_window = winit_builder.build().unwrap();
        let wayland_window = {
            let (w, h) = winit_window.get_inner_size().unwrap();
            let surface = winit_window.get_wayland_client_surface().unwrap();
            let egl_surface = wegl::WlEglSurface::new(surface, w as i32, h as i32);
            let context = {
                let libegl = unsafe { dlopen::dlopen(b"libEGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
                if libegl.is_null() {
                    return Err(CreationError::NotSupported);
                }
                let egl = ::api::egl::ffi::egl::Egl::load_with(|sym| {
                    let sym = CString::new(sym).unwrap();
                    unsafe { dlopen::dlsym(libegl, sym.as_ptr()) }
                });
                try!(EglContext::new(
                    egl,
                    pf_reqs, &opengl.clone().map_sharing(|_| unimplemented!()),        // TODO:
                    egl::NativeDisplay::Wayland(Some(winit_window.get_wayland_display().unwrap())))
                    .and_then(|p| p.finish(egl_surface.ptr() as *const _))
                )
            };
            Window {
                egl_surface: egl_surface,
                context: context,
            }
        };
        Ok((wayland_window, winit_window))
    }

    pub fn set_inner_size(&self, x: u32, y: u32, winit_window: &winit::Window) {
        winit_window.set_inner_size(x, y);
        self.egl_surface.resize(x as i32, y as i32, 0, 0);
    }

    #[inline]
    pub fn wait_events<'a>(&'a self, winit_window: &'a winit::Window) -> WaitEventsIterator {
        WaitEventsIterator {
            window: self,
            winit_iterator: winit_window.wait_events()
        }
    }

    #[inline]
    pub fn poll_events<'a>(&'a self, winit_window: &'a winit::Window) -> PollEventsIterator {
        PollEventsIterator {
            window: self,
            winit_iterator: winit_window.poll_events()
        }
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    #[inline]
    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        self.context.swap_buffers()
    }

    #[inline]
    fn get_api(&self) -> ::Api {
        self.context.get_api()
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format().clone()
    }
}
