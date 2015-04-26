#![cfg(target_os = "linux")]

use libc;
use api::dlopen;
use api::egl::Context as EglContext;

use BuilderAttribs;
use CreationError;
use Event;
use PixelFormat;
use CursorState;
use MouseCursor;

use std::collections::VecDeque;
use std::ffi::CString;
use std::mem;
use std::ptr;

mod libvc;

pub struct Window {
    libvc: libvc::LibVc,
    context: Option<EglContext>,
    display: libc::uint32_t,
    gles2: *mut libc::c_void,
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
        let libvc = match libvc::LibVc::open() {
            Ok(l) => l,
            Err(_) => return Err(CreationError::NotSupported),
        };

        unsafe { libvc.bcm_host_init() };

        let (screen_width, screen_height) = unsafe {
            let mut width = mem::uninitialized();
            let mut height = mem::uninitialized();
            let success = libvc.graphics_get_display_size(0 /* LCD */, &mut width, &mut height);
            if success < 0 {
                return Err(CreationError::OsError("graphics_get_display_size returned -1".to_string()))
            }
            (width, height)
        };

        let src_rect = libvc::VC_RECT_T {
            x: 0,
            y: 0,
            width: (screen_width << 16) as libc::int32_t,
            height: (screen_height << 16) as libc::int32_t,
        };

        let dest_rect = libvc::VC_RECT_T {
            x: 0,
            y: 0,
            width: screen_width as libc::int32_t,
            height: screen_height as libc::int32_t,
        };

        let dispman_display = unsafe {
            libvc.vc_dispmanx_display_open(0 /* LCD */)
        };
        if dispman_display == 0 {
            return Err(CreationError::OsError("vc_dispmanx_display_open failed".to_string()));
        }

        let dispman_update = unsafe {
            libvc.vc_dispmanx_update_start(0)
        };

        let dispman_element = unsafe {
            libvc.vc_dispmanx_element_add(dispman_update, dispman_display, 0, &dest_rect, 0,
                                          &src_rect, libvc::DISPMANX_PROTECTION_NONE, ptr::null(),
                                          ptr::null(), 0)
        };
        if dispman_element == 0 {
            return Err(CreationError::OsError("vc_dispmanx_element_add failed".to_string()));
        }

        let window = libvc::EGL_DISPMANX_WINDOW_T {
            element: dispman_element,
            width: screen_width as libc::int32_t,
            height: screen_height as libc::int32_t,
        };
        let window: *const libvc::EGL_DISPMANX_WINDOW_T = &window;

        unsafe { libvc.vc_dispmanx_update_submit_sync(dispman_update) };


        let gles2 = unsafe { dlopen::dlopen(b"/opt/vc/lib/libGLESv2.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
        if gles2.is_null() {
            return Err(CreationError::NotSupported);
        }

        // creating the EGL context
        let libegl = unsafe { dlopen::dlopen(b"/opt/vc/lib/libEGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
        if libegl.is_null() {
            return Err(CreationError::NotSupported);
        }
        let egl = ::api::egl::ffi::egl::Egl::load_with(|sym| {
            let sym = CString::new(sym).unwrap();
            unsafe { dlopen::dlsym(libegl, sym.as_ptr()) }
        });
        let context = try!(EglContext::new(egl, builder, None, window as *const _));

        Ok(Window {
            libvc: libvc,
            context: Some(context),
            display: dispman_display,
            gles2: gles2,
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

    pub unsafe fn make_current(&self) {
        self.context.as_ref().unwrap().make_current();
    }

    pub fn is_current(&self) -> bool {
        self.context.as_ref().unwrap().is_current()
    }

    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let sym = CString::new(addr).unwrap();
        let ptr = unsafe { dlopen::dlsym(self.gles2, sym.as_ptr()) };
        if !ptr.is_null() {
            return ptr as *const _;
        }

        self.context.as_ref().unwrap().get_proc_address(addr)
    }

    pub fn swap_buffers(&self) {
        self.context.as_ref().unwrap().swap_buffers();
    }

    pub fn platform_display(&self) -> *mut libc::c_void {
        unimplemented!()
    }

    pub fn platform_window(&self) -> *mut libc::c_void {
        unimplemented!()
    }

    pub fn get_api(&self) -> ::Api {
        self.context.as_ref().unwrap().get_api()
    }

    pub fn get_pixel_format(&self) -> PixelFormat {
        unimplemented!();
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

impl Drop for Window {
    fn drop(&mut self) {
        self.context = None;

        unsafe {
            self.libvc.vc_dispmanx_display_close(self.display);
        }
    }
}
