use std::cell::RefCell;
use std::sync::{Mutex, Weak, Arc};
use std::collections::HashMap;
use std::ffi::CString;
use winit;
use winit::os::unix::WindowExt;
use {ContextError, CreationError, GlAttributes, GlContext, PixelFormat, PixelFormatRequirements};
use api::dlopen;
use api::egl;
use api::egl::Context as EglContext;
use wayland_client::egl as wegl;
use {ControlFlow, Event, WindowEvent, EventsLoopClosed};

pub struct Window {
    egl_surface: Arc<wegl::WlEglSurface>,
    context: EglContext,
}


pub struct EventsLoop {
    winit_events_loop: RefCell<winit::EventsLoop>,
    egl_surfaces: Mutex<HashMap<winit::WindowId, Weak<wegl::WlEglSurface>>>,
}

impl EventsLoop {
    /// Builds a new events loop.
    pub fn new(events_loop: winit::EventsLoop) -> EventsLoop {
        EventsLoop {
            winit_events_loop: RefCell::new(events_loop),
            egl_surfaces: Mutex::new(HashMap::new()),
        }
    }

    fn resize_surface(&self, window_id: winit::WindowId, x: u32, y: u32) {
        if let Ok(egl_surfaces) = self.egl_surfaces.lock() {
            if let Some(surface) = egl_surfaces[&window_id].upgrade() {
                surface.resize(x as i32, y as i32, 0, 0)
            }
        }
    }

    fn insert_window(&self,
                     window_id: winit::WindowId,
                     egl_surface: &Arc<wegl::WlEglSurface>)
    {
        if let Ok(mut my_surfaces) = self.egl_surfaces.lock() {
            my_surfaces.insert(window_id, Arc::downgrade(egl_surface));
        }
    }

    /// Fetches all the events that are pending, calls the callback function for each of them,
    /// and returns.
    #[inline]
    pub fn poll_events<F>(&mut self, mut callback: F)
        where F: FnMut(Event)
    {
        self.winit_events_loop.borrow_mut().poll_events(|event| {
            if let Event::WindowEvent { window_id, event: WindowEvent::Resized(x, y) } = event {
                self.resize_surface(window_id, x, y)
            }
            callback(event)
        })
    }

    /// Runs forever until `interrupt()` is called. Whenever an event happens, calls the callback.
    #[inline]
    pub fn run_forever<F>(&mut self, mut callback: F)
        where F: FnMut(Event) -> ControlFlow
    {
        self.winit_events_loop.borrow_mut().run_forever(|event| {
            if let Event::WindowEvent { window_id, event: WindowEvent::Resized(x, y) } = event {
                self.resize_surface(window_id, x, y)
            }
            callback(event)
        })
    }

    /// Creates an EventsLoopProxy that can be used to wake up the EventsLoop from another thread.
    #[inline]
    pub fn create_proxy(&self) -> EventsLoopProxy {
        let proxy = self.winit_events_loop.borrow().create_proxy();
        EventsLoopProxy { proxy: proxy }
    }
}

pub struct EventsLoopProxy {
    proxy: winit::EventsLoopProxy,
}

impl EventsLoopProxy {
    /// Wake up the EventsLoop from which this proxy was created.
    ///
    /// This causes the EventsLoop to emit an Awakened event.
    ///
    /// Returns an Err if the associated EventsLoop no longer exists.
    #[inline]
    pub fn wakeup(&self) -> Result<(), EventsLoopClosed> {
        self.proxy.wakeup()
    }
}

impl Window {
    pub fn new(
        events_loop: &EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        winit_builder: winit::WindowBuilder,
    ) -> Result<(Window, winit::Window), CreationError> {
        let winit_window = winit_builder.build(&*events_loop.winit_events_loop.borrow()).unwrap();
        let wayland_window = {
            let (w, h) = winit_window.get_inner_size().unwrap();
            let surface = winit_window.get_wayland_surface().unwrap();
            let egl_surface = unsafe { wegl::WlEglSurface::new_from_raw(surface as *mut _, w as i32, h as i32) };
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
                egl_surface: Arc::new(egl_surface),
                context: context,
            }
        };
        // Store a copy of the `context`'s `IdRef` so that we can `update` it on `Resized` events.
        events_loop.insert_window(winit_window.id(), &wayland_window.egl_surface);
        Ok((wayland_window, winit_window))
    }

    pub fn set_inner_size(&self, x: u32, y: u32, winit_window: &winit::Window) {
        winit_window.set_inner_size(x, y);
        self.egl_surface.resize(x as i32, y as i32, 0, 0);
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
