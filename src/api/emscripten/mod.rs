#![cfg(target_os = "emscripten")]

use std::ffi::CString;
use libc;
use Api;
use Event;
use CreationError;
use ContextError;
use CursorState;
use GlAttributes;
use GlContext;
use MouseCursor;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;

use winit;
pub use winit::WindowProxy;

use ElementState;
use MouseButton;
use VirtualKeyCode;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::Deref;
use platform::PlatformSpecificWindowBuilderAttributes;

mod ffi;
mod keyboard;

pub struct Window {
    context: ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
    events: Box<RefCell<VecDeque<Event>>>,
}

pub struct PollEventsIterator<'a> {
    window: &'a Window,
}

impl<'a> Iterator for PollEventsIterator<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Event> {
        self.window.events.deref().borrow_mut().pop_front()
    }
}

pub struct WaitEventsIterator<'a> {
    window: &'a Window,
}

impl<'a> Iterator for WaitEventsIterator<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Event> {
        // TODO
        None
    }
}

const CANVAS_NAME: &'static str = "#canvas\0";
const DOCUMENT_NAME: &'static str = "#document\0";

impl Window {
    pub fn new(_: &WindowAttributes,
               pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&Window>,
               _: &PlatformSpecificWindowBuilderAttributes,
               _: winit::WindowBuilder)
                -> Result<Window, CreationError> {

        // getting the default values of attributes
        let mut attributes = unsafe {
            use std::mem;
            let mut attributes: ffi::EmscriptenWebGLContextAttributes = mem::uninitialized();
            ffi::emscripten_webgl_init_context_attributes(&mut attributes);
            attributes
        };

        // setting the attributes
        // FIXME: 
        /*match builder.opengl.version {
            Some((major, minor)) => {
                attributes.majorVersion = major as libc::c_int;
                attributes.minorVersion = minor as libc::c_int;
            },
            None => ()
        };*/

        // creating the context
        let context = unsafe {
            use std::{mem, ptr};
            let context = ffi::emscripten_webgl_create_context(ptr::null(), &attributes);
            if context <= 0 {
                return Err(CreationError::OsError(format!("Error while calling emscripten_webgl_create_context: {}",
                    error_to_str(mem::transmute(context)))));
            }
            context
        };

        let events = Box::new(RefCell::new(VecDeque::new()));

        {
            use std::mem;
            // TODO: set up more event callbacks
            unsafe {
                ffi::emscripten_set_mousemove_callback(CANVAS_NAME.as_ptr(),
                                              mem::transmute(events.deref()),
                                              ffi::EM_FALSE,
                                              mouse_callback);
                ffi::emscripten_set_mousedown_callback(CANVAS_NAME.as_ptr(),
                                              mem::transmute(events.deref()),
                                              ffi::EM_FALSE,
                                              mouse_callback);
                ffi::emscripten_set_mouseup_callback(CANVAS_NAME.as_ptr(),
                                              mem::transmute(events.deref()),
                                              ffi::EM_FALSE,
                                              mouse_callback);
                ffi::emscripten_set_keydown_callback(DOCUMENT_NAME.as_ptr(),
                                            mem::transmute(events.deref()),
                                            ffi::EM_FALSE,
                                            keyboard_callback);
                ffi::emscripten_set_keyup_callback(DOCUMENT_NAME.as_ptr(),
                                            mem::transmute(events.deref()),
                                            ffi::EM_FALSE,
                                            keyboard_callback);
            }
        }

        // TODO: emscripten_set_webglcontextrestored_callback

        Ok(Window {
            context: context,
            events: events,
        })
    }

    #[inline]
    pub fn set_title(&self, _title: &str) {
    }

    #[inline]
    pub fn get_position(&self) -> Option<(i32, i32)> {
        Some((0, 0))
    }

    #[inline]
    pub fn set_position(&self, _: i32, _: i32) {
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        unsafe {
            use std::{mem, ptr};
            let mut width = mem::uninitialized();
            let mut height = mem::uninitialized();

            if ffi::emscripten_get_element_css_size(ptr::null(), &mut width, &mut height)
                != ffi::EMSCRIPTEN_RESULT_SUCCESS
            {
                None
            } else {
                Some((width as u32, height as u32))
            }
        }
    }

    #[inline]
    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        self.get_inner_size()
    }

    #[inline]
    pub fn set_inner_size(&self, width: u32, height: u32) {
        unsafe {
            use std::ptr;
            ffi::emscripten_set_element_css_size(ptr::null(), width as libc::c_double, height
                as libc::c_double);
        }
    }

    #[inline]
    pub fn poll_events(&self) -> PollEventsIterator {
        PollEventsIterator {
            window: self,
        }
    }

    #[inline]
    pub fn wait_events(&self) -> WaitEventsIterator {
        WaitEventsIterator {
            window: self,
        }
    }

    #[inline]
    pub fn create_window_proxy(&self) -> WindowProxy {
        unimplemented!()
    }

    #[inline]
    pub fn show(&self) {}
    #[inline]
    pub fn hide(&self) {}

    #[inline]
    pub fn platform_display(&self) -> *mut libc::c_void {
        unimplemented!()
    }

    #[inline]
    pub fn platform_window(&self) -> *mut libc::c_void {
        unimplemented!()
    }

    #[inline]
    pub fn set_window_resize_callback(&mut self, _: Option<fn(u32, u32)>) {
        // TODO
    }

    #[inline]
    pub fn set_cursor(&self, cursor: MouseCursor) {
    }

    #[inline]
    pub fn set_cursor_state(&self, state: CursorState) -> Result<(), String> {
        Ok(())
    }

    #[inline]
    pub fn hidpi_factor(&self) -> f32 {
        1.0
    }

    #[inline]
    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        Ok(())
    }

    #[inline]
    pub fn get_inner_size_points(&self) -> Option<(u32, u32)> {
        unimplemented!()
    }

    #[inline]
    pub fn get_inner_size_pixels(&self) -> Option<(u32, u32)> {
        unimplemented!()
    }

    #[inline]
    pub fn as_winit_window(&self) -> &winit::Window {
        unimplemented!()
    }

    #[inline]
    pub fn as_winit_window_mut(&mut self) -> &mut winit::Window {
        unimplemented!()
    }

    #[inline]
    pub fn hdpi_factor(&self) -> f32 {
        unimplemented!();
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        // TOOD: check if == EMSCRIPTEN_RESULT
        ffi::emscripten_webgl_make_context_current(self.context);
        Ok(())
    }

    #[inline]
    fn is_current(&self) -> bool {
        true        // FIXME: 
    }

    fn get_proc_address(&self, addr: &str) -> *const () {
        let addr = CString::new(addr).unwrap();

        unsafe {
            // FIXME: if `as_ptr()` is used, then wrong data is passed to emscripten
            ffi::emscripten_GetProcAddress(addr.into_raw() as *const _) as *const _
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        Ok(())
    }

    #[inline]
    fn get_api(&self) -> Api {
        Api::WebGl
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        unimplemented!();
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            ffi::emscripten_exit_fullscreen();
            ffi::emscripten_webgl_destroy_context(self.context);
        }
    }
}

fn error_to_str(code: ffi::EMSCRIPTEN_RESULT) -> &'static str {
    match code {
        ffi::EMSCRIPTEN_RESULT_SUCCESS | ffi::EMSCRIPTEN_RESULT_DEFERRED
            => "Internal error in the library (success detected as failure)",

        ffi::EMSCRIPTEN_RESULT_NOT_SUPPORTED => "Not supported",
        ffi::EMSCRIPTEN_RESULT_FAILED_NOT_DEFERRED => "Failed not deferred",
        ffi::EMSCRIPTEN_RESULT_INVALID_TARGET => "Invalid target",
        ffi::EMSCRIPTEN_RESULT_UNKNOWN_TARGET => "Unknown target",
        ffi::EMSCRIPTEN_RESULT_INVALID_PARAM => "Invalid parameter",
        ffi::EMSCRIPTEN_RESULT_FAILED => "Failed",
        ffi::EMSCRIPTEN_RESULT_NO_DATA => "No data",

        _ => "Undocumented error"
    }
}


extern fn mouse_callback(
        event_type: libc::c_int,
        event: *const ffi::EmscriptenMouseEvent,
        event_queue: *mut libc::c_void) -> ffi::EM_BOOL {
    unsafe {
        use std::mem;
        let queue: &RefCell<VecDeque<Event>> = mem::transmute(event_queue);
        match event_type {
            ffi::EMSCRIPTEN_EVENT_MOUSEMOVE => {
                queue.borrow_mut().push_back(Event::MouseMoved(
                        (*event).client_x as i32,
                        (*event).client_y as i32));
            },
            ffi::EMSCRIPTEN_EVENT_MOUSEDOWN => {
                queue.borrow_mut().push_back(Event::MouseInput(
                        ElementState::Pressed,
                        match (*event).button {
                            0 => MouseButton::Left,
                            1 => MouseButton::Middle,
                            2 => MouseButton::Right,
                            other => MouseButton::Other(other as u8),
                        }));
            },
            ffi::EMSCRIPTEN_EVENT_MOUSEUP => {
                queue.borrow_mut().push_back(Event::MouseInput(
                        ElementState::Released,
                        match (*event).button {
                            0 => MouseButton::Left,
                            1 => MouseButton::Middle,
                            2 => MouseButton::Right,
                            other => MouseButton::Other(other as u8),
                        }));
            },
            _ => {
            }
        }
    }
    ffi::EM_TRUE
}

extern fn keyboard_callback(
        event_type: libc::c_int,
        event: *const ffi::EmscriptenKeyboardEvent,
        event_queue: *mut libc::c_void) -> ffi::EM_BOOL {
    unsafe {
        use std::mem;
        let queue: &RefCell<VecDeque<Event>> = mem::transmute(event_queue);
        let code = keyboard::key_translate((*event).key);
        let virtual_key = keyboard::key_translate_virt(
            (*event).key, (*event).location);
        match event_type {
            ffi::EMSCRIPTEN_EVENT_KEYDOWN => {
                queue.borrow_mut().push_back(Event::KeyboardInput(
                        ElementState::Pressed,
                        code,
                        virtual_key));
            },
            ffi::EMSCRIPTEN_EVENT_KEYUP => {
                queue.borrow_mut().push_back(Event::KeyboardInput(
                        ElementState::Released,
                        code,
                        virtual_key));
            },
            _ => {
            }
        }
    }
    ffi::EM_TRUE
}

