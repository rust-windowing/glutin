use libc;
use api::egl::Context;

use BuilderAttribs;
use ContextError;
use CreationError;
use GlContext;
use Event;
use PixelFormat;
use CursorState;
use MouseCursor;

use std::collections::VecDeque;
use std::ffi::CString;

use api::dlopen;
use api::egl::ffi::egl::Egl;
use api::egl::Context as EglContext;

mod libgbm;
mod device;

pub struct Window {
    device: device::GbmDevice,
    context: EglContext,
}

#[derive(Clone)]
pub struct WindowProxy;

impl WindowProxy {
    pub fn wakeup_event_loop(&self) {
        unimplemented!()
    }
}

pub struct MonitorID;

pub fn get_available_monitors() -> VecDeque<MonitorID> {
    VecDeque::new()
}
pub fn get_primary_monitor() -> MonitorID {
    MonitorID
}

impl MonitorID {
    pub fn get_name(&self) -> Option<String> {
        unimplemented!();
    }

    pub fn get_native_identifier(&self) -> ::native_monitor::NativeMonitorId {
        ::native_monitor::NativeMonitorId::Unavailable
    }

    pub fn get_dimensions(&self) -> (u32, u32) {
        unimplemented!();
    }
}


pub struct PollEventsIterator<'a> {
    window: &'a Window,
}

impl<'a> Iterator for PollEventsIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        None
    }
}

pub struct WaitEventsIterator<'a> {
    window: &'a Window,
}

impl<'a> Iterator for WaitEventsIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        loop {}
    }
}

impl Window {
    pub fn new(builder: BuilderAttribs) -> Result<Window, CreationError> {
        // TODO: use something safer than raw "dlopen"
        let egl = {
            let libegl = unsafe { dlopen::dlopen(b"libEGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };

            if libegl.is_null() {
                None
            } else {
                Some(Egl::load_with(|sym| {
                    let sym = CString::new(sym).unwrap();
                    unsafe { dlopen::dlsym(libegl, sym.as_ptr()) }
                }))
            }.unwrap()
        };

        let mut device = device::GbmDevice::open();
        let context = try!(EglContext::new(egl, &builder, Some(device.get_device()), device.get_surface()));

        Ok(Window {
            device: device,
            context: context,
        })
    }

    pub fn is_closed(&self) -> bool {
        false
    }

    pub fn set_title(&self, title: &str) {
    }

    pub fn show(&self) {
    }

    pub fn hide(&self) {
    }

    pub fn get_position(&self) -> Option<(i32, i32)> {
        unimplemented!()
    }

    pub fn set_position(&self, x: i32, y: i32) {
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        unimplemented!()
    }

    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        unimplemented!()
    }

    pub fn set_inner_size(&self, _x: u32, _y: u32) {
        unimplemented!()
    }

    pub fn create_window_proxy(&self) -> WindowProxy {
        unimplemented!()
    }

    pub fn poll_events(&self) -> PollEventsIterator {
        PollEventsIterator {
            window: self
        }
    }

    pub fn wait_events(&self) -> WaitEventsIterator {
        WaitEventsIterator {
            window: self
        }
    }

    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr) as *const _
    }

    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.context.swap_buffers()
    }

    pub fn platform_display(&self) -> *mut libc::c_void {
        unimplemented!()
    }

    pub fn platform_window(&self) -> *mut libc::c_void {
        unimplemented!()
    }

    pub fn get_api(&self) -> ::Api {
        self.context.get_api()
    }

    pub fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format()
    }

    pub fn set_window_resize_callback(&mut self, _: Option<fn(u32, u32)>) {
    }

    pub fn set_cursor(&self, cursor: MouseCursor) {
    }

    pub fn set_cursor_state(&self, state: CursorState) -> Result<(), String> {
        Ok(())
    }

    pub fn hidpi_factor(&self) -> f32 {
        1.0
    }

    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        Ok(())
    }
}
