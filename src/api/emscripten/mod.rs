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

use std::cell::RefCell;
use std::collections::{VecDeque, BTreeMap};
use std::ops::Deref;
use platform::PlatformSpecificWindowBuilderAttributes;

mod ffi;
mod keyboard;

pub struct InnerWindow {
    cursor_state: ::CursorState,
    events: VecDeque<Event>,
    active_touches: BTreeMap<libc::c_long, (i32, i32)>,
}

pub struct Window {
    inner: Box<RefCell<InnerWindow>>,
    context: ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
}

pub struct PollEventsIterator<'a> {
    window: &'a Window,
}

impl<'a> Iterator for PollEventsIterator<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Event> {
        self.window.inner.deref().borrow_mut().events.pop_front()
    }
}

pub struct WaitEventsIterator<'a> {
    window: &'a Window,
}

impl<'a> Iterator for WaitEventsIterator<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Event> {
        unimplemented!()
    }
}

const CANVAS_NAME: &'static str = "#canvas\0";
const DOCUMENT_NAME: &'static str = "#document\0";

impl Window {
    pub fn new(_: &WindowAttributes,
               _pf_reqs: &PixelFormatRequirements,
               _opengl: &GlAttributes<&Window>,
               _: &PlatformSpecificWindowBuilderAttributes,
               window_builder: winit::WindowBuilder)
                -> Result<Window, CreationError> {

        // getting the default values of attributes
        let attributes = unsafe {
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


        let window = Window {
            inner: Box::new(RefCell::new(InnerWindow {
                events: VecDeque::new(),
                cursor_state: ::CursorState::Normal,
                active_touches: BTreeMap::new(),
            })),
            context: context,
        };

        {
            use std::mem;
            // TODO: set up more event callbacks
            unsafe {
                em_try(ffi::emscripten_set_mousemove_callback(CANVAS_NAME.as_ptr(),
                                              mem::transmute(window.inner.deref()),
                                              ffi::EM_FALSE,
                                              mouse_callback))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_mousemove_callback: {}", e)))?;
                em_try(ffi::emscripten_set_mousedown_callback(CANVAS_NAME.as_ptr(),
                                              mem::transmute(window.inner.deref()),
                                              ffi::EM_FALSE,
                                              mouse_callback))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_mousedown_callback: {}", e)))?;
                em_try(ffi::emscripten_set_mouseup_callback(CANVAS_NAME.as_ptr(),
                                              mem::transmute(window.inner.deref()),
                                              ffi::EM_FALSE,
                                              mouse_callback))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_mouseup_callback: {}", e)))?;
                em_try(ffi::emscripten_set_keydown_callback(DOCUMENT_NAME.as_ptr(),
                                            mem::transmute(window.inner.deref()),
                                            ffi::EM_FALSE,
                                            keyboard_callback))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_keydown_callback: {}", e)))?;
                em_try(ffi::emscripten_set_keyup_callback(DOCUMENT_NAME.as_ptr(),
                                            mem::transmute(window.inner.deref()),
                                            ffi::EM_FALSE,
                                            keyboard_callback))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_keyup_callback: {}", e)))?;
                em_try(ffi::emscripten_set_touchstart_callback(CANVAS_NAME.as_ptr(),
                                            mem::transmute(window.inner.deref()),
                                            ffi::EM_FALSE,
                                            Some(touch_callback)))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_touchstart_callback: {}", e)))?;
                em_try(ffi::emscripten_set_touchend_callback(CANVAS_NAME.as_ptr(),
                                            mem::transmute(window.inner.deref()),
                                            ffi::EM_FALSE,
                                            Some(touch_callback)))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_touchend_callback: {}", e)))?;
                em_try(ffi::emscripten_set_touchmove_callback(CANVAS_NAME.as_ptr(),
                                            mem::transmute(window.inner.deref()),
                                            ffi::EM_FALSE,
                                            Some(touch_callback)))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_touchmove_callback: {}", e)))?;
                em_try(ffi::emscripten_set_touchcancel_callback(CANVAS_NAME.as_ptr(),
                                            mem::transmute(window.inner.deref()),
                                            ffi::EM_FALSE,
                                            Some(touch_callback)))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_touchcancel_callback: {}", e)))?;
                em_try(ffi::emscripten_set_pointerlockchange_callback(CANVAS_NAME.as_ptr(),
                                            mem::transmute(window.inner.deref()),
                                            ffi::EM_FALSE,
                                            Some(pointerlockchange_callback)))
                    .map_err(|e| ::CreationError::OsError(
                            format!("Error while calling emscripten_set_pointerlockchange_callback: {}", e)))?;
            }
        }

        // TODO: emscripten_set_webglcontextrestored_callback

        if window_builder.window.monitor.is_some() {
            use std::ptr;
            unsafe {
                em_try(ffi::emscripten_request_fullscreen(ptr::null(), ffi::EM_TRUE))
                    .map_err(|e| ::CreationError::OsError(format!("Error while calling emscripten_request_fullscreen: {}", e)))?;
                em_try(ffi::emscripten_set_fullscreenchange_callback(ptr::null(), 0 as *mut libc::c_void, ffi::EM_FALSE, Some(fullscreen_callback)))
                    .map_err(|e| ::CreationError::OsError(format!("Error while calling emscripten_set_fullscreenchange_callback: {}", e)))?;
            }
        } else if let Some((w, h)) = window_builder.window.dimensions {
            window.set_inner_size(w, h);
        }

        Ok(window)
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
            use std::mem;
            let mut width = mem::uninitialized();
            let mut height = mem::uninitialized();
            let mut fullscreen = mem::uninitialized();

            ffi::emscripten_get_canvas_size(&mut width, &mut height, &mut fullscreen);
            Some((width as u32, height as u32))
        }
    }

    #[inline]
    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        self.get_inner_size()
    }

    #[inline]
    pub fn set_inner_size(&self, width: u32, height: u32) {
        unsafe { ffi::emscripten_set_canvas_size(width as i32, height as i32); }
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
    pub fn set_cursor(&self, _cursor: MouseCursor) {
    }

    #[inline]
    pub fn set_cursor_state(&self, state: CursorState) -> Result<(), String> {
        use std::ptr;
        unsafe {
            use ::CursorState::*;

            let ref mut old_state = self.inner.borrow_mut().cursor_state;
            if state == *old_state {
                return Ok(());
            }

            // Go back to normal cursor state
            match *old_state {
                Hide => show_mouse(),
                Grab => em_try(ffi::emscripten_exit_pointerlock())
                    .map_err(|e| format!("Error while calling emscripten_exit_pointerlock: {}", e))?,
                Normal => (),
            }

            // Set cursor from normal cursor state
            match state {
                Hide => ffi::emscripten_hide_mouse(),
                Grab => em_try(ffi::emscripten_request_pointerlock(ptr::null(), ffi::EM_TRUE))
                    .map_err(|e| format!("Error while calling emscripten_request_pointerlock: {}", e))?,
                Normal => (),
            }

            // Update
            *old_state = state;

            Ok(())
        }
    }

    #[inline]
    pub fn hidpi_factor(&self) -> f32 {
        unsafe { ffi::emscripten_get_device_pixel_ratio() as f32 }
    }

    #[inline]
    pub fn set_cursor_position(&self, _x: i32, _y: i32) -> Result<(), ()> {
        Ok(())
    }

    #[inline]
    pub fn get_inner_size_points(&self) -> Option<(u32, u32)> {
        unimplemented!()
    }

    #[inline]
    pub fn get_inner_size_pixels(&self) -> Option<(u32, u32)> {
        self.get_inner_size()
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
        self.hidpi_factor()
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
        use std::ptr;
        unsafe {
            ffi::emscripten_set_pointerlockchange_callback(ptr::null(), 0 as *mut libc::c_void, ffi::EM_FALSE, None);
            ffi::emscripten_set_fullscreenchange_callback(ptr::null(), 0 as *mut libc::c_void, ffi::EM_FALSE, None);
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

fn em_try(res: ffi::EMSCRIPTEN_RESULT) -> Result<(), String> {
    match res {
        ffi::EMSCRIPTEN_RESULT_SUCCESS | ffi::EMSCRIPTEN_RESULT_DEFERRED => Ok(()),
        r @ _ => Err(error_to_str(r).to_string()),
    }
}

extern fn mouse_callback(
        event_type: libc::c_int,
        event: *const ffi::EmscriptenMouseEvent,
        user_data: *mut libc::c_void) -> ffi::EM_BOOL {
    unsafe {
        use std::mem;
        use std::ptr;

        let inner_window: &RefCell<InnerWindow> = mem::transmute(user_data);
        let mut inner_window = inner_window.borrow_mut();

        match event_type {
            ffi::EMSCRIPTEN_EVENT_MOUSEMOVE => {
                // send relative move if cursor is grabbed
                match inner_window.cursor_state {
                    ::CursorState::Grab => inner_window.events.push_back(
                        Event::MouseMoved(
                            (*event).movement_x as i32,
                            (*event).movement_y as i32)),
                    _ => inner_window.events.push_back(
                        Event::MouseMoved(
                            (*event).canvas_x as i32,
                            (*event).canvas_y as i32)),
                }
            },
            ffi::EMSCRIPTEN_EVENT_MOUSEDOWN => {
                // call pointerlock if needed
                if let ::CursorState::Grab = inner_window.cursor_state {
                    let mut pointerlock_status: ffi::EmscriptenPointerlockChangeEvent = mem::uninitialized();
                    ffi::emscripten_get_pointerlock_status(&mut pointerlock_status);
                    if pointerlock_status.isActive == ffi::EM_FALSE {
                        ffi::emscripten_request_pointerlock(ptr::null(), ffi::EM_TRUE);
                    }
                }

                inner_window.events.push_back(Event::MouseInput(
                        ElementState::Pressed,
                        match (*event).button {
                            0 => MouseButton::Left,
                            1 => MouseButton::Middle,
                            2 => MouseButton::Right,
                            other => MouseButton::Other(other as u8),
                        }));
            },
            ffi::EMSCRIPTEN_EVENT_MOUSEUP => {
                inner_window.events.push_back(Event::MouseInput(
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
        user_data: *mut libc::c_void) -> ffi::EM_BOOL {
    unsafe {
        use std::mem;
        let inner_window: &RefCell<InnerWindow> = mem::transmute(user_data);
        let mut inner_window = inner_window.borrow_mut();

        let code = keyboard::key_translate((*event).key);
        let virtual_key = keyboard::key_translate_virt(
            (*event).key, (*event).location);
        match event_type {
            ffi::EMSCRIPTEN_EVENT_KEYDOWN => {
                inner_window.events.push_back(Event::KeyboardInput(
                        ElementState::Pressed,
                        code,
                        virtual_key));
            },
            ffi::EMSCRIPTEN_EVENT_KEYUP => {
                inner_window.events.push_back(Event::KeyboardInput(
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

extern fn touch_callback(
        event_type: libc::c_int,
        event: *const ffi::EmscriptenTouchEvent,
        user_data: *mut libc::c_void) -> ffi::EM_BOOL {
    unsafe {
        use std::mem;

        let inner_window: &RefCell<InnerWindow> = mem::transmute(user_data);
        let mut inner_window = inner_window.borrow_mut();

        match event_type {
            ffi::EMSCRIPTEN_EVENT_TOUCHSTART => {
                // Check for all new identifier
                for touch in 0..(*event).numTouches as usize {
                    let touch = (*event).touches[touch];
                    if !inner_window.active_touches.contains_key(&touch.identifier) {
                        inner_window.active_touches.insert(touch.identifier, (touch.canvasX, touch.canvasY));
                        inner_window.events.push_back(Event::Touch( winit::Touch {
                            phase: winit::TouchPhase::Started,
                            location: (touch.canvasX as f64, touch.canvasY as f64),
                            id: touch.identifier as u64,
                        }))
                    }
                }
            },
            ffi::EMSCRIPTEN_EVENT_TOUCHEND | ffi::EMSCRIPTEN_EVENT_TOUCHCANCEL => {
                // Check for event that are not onTarget
                let phase = match event_type {
                    ffi::EMSCRIPTEN_EVENT_TOUCHEND => winit::TouchPhase::Ended,
                    ffi::EMSCRIPTEN_EVENT_TOUCHCANCEL => winit::TouchPhase::Cancelled,
                    _ => unreachable!(),
                };
                for touch in 0..(*event).numTouches  as usize {
                    let touch = (*event).touches[touch];
                    if touch.onTarget == 0 {
                        inner_window.active_touches.remove(&touch.identifier);
                        inner_window.events.push_back(Event::Touch( winit::Touch {
                            phase: phase,
                            location: (touch.canvasX as f64, touch.canvasY as f64),
                            id: touch.identifier as u64,
                        }))
                    }
                }
            }
            ffi::EMSCRIPTEN_EVENT_TOUCHMOVE => {
                // check for all event that have changed coordinates
                for touch in 0..(*event).numTouches  as usize {
                    let touch = (*event).touches[touch];

                    let diff = if let Some(active_touch) = inner_window.active_touches.get_mut(&touch.identifier) {
                        if active_touch.0 != touch.canvasX || active_touch.1 != touch.canvasY {
                            *active_touch = (touch.canvasX, touch.canvasY);
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if diff {
                        inner_window.events.push_back(Event::Touch( winit::Touch {
                            phase: winit::TouchPhase::Moved,
                            location: (touch.canvasX as f64, touch.canvasY as f64),
                            id: touch.identifier as u64,
                        }))
                    }
                }
            }
            _ => ()
        }
    }
    ffi::EM_TRUE
}

// In case of fullscreen window this method will request fullscreen on change
#[allow(non_snake_case)]
unsafe extern "C" fn fullscreen_callback(
    _eventType: libc::c_int,
    _fullscreenChangeEvent: *const ffi::EmscriptenFullscreenChangeEvent,
    _userData: *mut libc::c_void) -> ffi::EM_BOOL
{
    use std::ptr;
    ffi::emscripten_request_fullscreen(ptr::null(), ffi::EM_TRUE);
    ffi::EM_FALSE
}

// In case of pointer grabbed this method will request pointer lock on change
#[allow(non_snake_case)]
unsafe extern "C" fn pointerlockchange_callback(
    _eventType: libc::c_int,
    pointerlockChangeEvent: *const ffi::EmscriptenPointerlockChangeEvent,
    user_data: *mut libc::c_void) -> ffi::EM_BOOL
{
    use std::mem;
    use std::ptr;

    let inner_window: &RefCell<InnerWindow> = mem::transmute(user_data);
    let inner_window = inner_window.borrow();

    if let ::CursorState::Grab = inner_window.cursor_state {
        if (*pointerlockChangeEvent).isActive == ffi::EM_FALSE {
            ffi::emscripten_request_pointerlock(ptr::null(), ffi::EM_TRUE);
        }
    }
    ffi::EM_FALSE
}

fn show_mouse() {
    // Hide mouse hasn't show mouse equivalent.
    // There is a pull request on emscripten that hasn't been merged #4616
    // that contains:
    //
    // var styleSheet = document.styleSheets[0];
    // var rules = styleSheet.cssRules;
    // for (var i = 0; i < rules.length; i++) {
    //   if (rules[i].cssText.substr(0, 6) == 'canvas') {
    //     styleSheet.deleteRule(i);
    //     i--;
    //   }
    // }
    // styleSheet.insertRule('canvas.emscripten { border: none; cursor: auto; }', 0);
    unsafe {
            ffi::emscripten_asm_const(b"var styleSheet = document.styleSheets[0]; var rules = styleSheet.cssRules; for (var i = 0; i < rules.length; i++) { if (rules[i].cssText.substr(0, 6) == 'canvas') { styleSheet.deleteRule(i); i--; } } styleSheet.insertRule('canvas.emscripten { border: none; cursor: auto; }', 0);\0" as *const u8);
    }
}
