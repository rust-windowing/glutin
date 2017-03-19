#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libc;

pub const EM_FALSE: EM_BOOL = 0;
pub const EM_TRUE: EM_BOOL = 1;

pub type EM_BOOL = libc::c_int;
pub type EM_UTF8 = libc::c_char;
pub type EMSCRIPTEN_WEBGL_CONTEXT_HANDLE = libc::c_int;
pub type EMSCRIPTEN_RESULT = libc::c_int;

pub type em_webgl_context_callback = extern fn(libc::c_int, *const libc::c_void, *mut libc::c_void)
    -> EM_BOOL;

pub type em_callback_func = unsafe extern fn();

pub type em_fullscreenchange_callback_func = Option<unsafe extern "C" fn(
    eventType: ::libc::c_int,
    fullscreenChangeEvent: *const EmscriptenFullscreenChangeEvent,
    userData: *mut ::libc::c_void) -> EM_BOOL>;

#[repr(C)]
pub struct EmscriptenFullscreenChangeEvent {
    pub isFullscreen: ::libc::c_int,
    pub fullscreenEnabled: ::libc::c_int,
    pub nodeName: [::libc::c_char; 128usize],
    pub id: [::libc::c_char; 128usize],
    pub elementWidth: ::libc::c_int,
    pub elementHeight: ::libc::c_int,
    pub screenWidth: ::libc::c_int,
    pub screenHeight: ::libc::c_int,
}
#[test]
fn bindgen_test_layout_EmscriptenFullscreenChangeEvent() {
    assert_eq!(::std::mem::size_of::<EmscriptenFullscreenChangeEvent>(), 280usize);
    assert_eq!(::std::mem::align_of::<EmscriptenFullscreenChangeEvent>(), 4usize);
}

#[repr(C)]
pub struct EmscriptenPointerlockChangeEvent {
    pub isActive: ::libc::c_int,
    pub nodeName: [::libc::c_char; 128usize],
    pub id: [::libc::c_char; 128usize],
}
#[test]
fn bindgen_test_layout_EmscriptenPointerlockChangeEvent() {
    assert_eq!(::std::mem::size_of::<EmscriptenPointerlockChangeEvent>(), 260usize);
    assert_eq!(::std::mem::align_of::<EmscriptenPointerlockChangeEvent>(), 4usize);
}

pub const EMSCRIPTEN_EVENT_KEYDOWN: libc::c_int = 2;
pub const EMSCRIPTEN_EVENT_KEYUP: libc::c_int = 3;

// pub const EMSCRIPTEN_EVENT_CLICK: libc::c_int = 4;
pub const EMSCRIPTEN_EVENT_MOUSEDOWN: libc::c_int = 5;
pub const EMSCRIPTEN_EVENT_MOUSEUP: libc::c_int = 6;
pub const EMSCRIPTEN_EVENT_MOUSEMOVE: libc::c_int = 8;

pub const EM_HTML5_SHORT_STRING_LEN_BYTES: usize  = 32;

pub const DOM_KEY_LOCATION_STANDARD: libc::c_ulong = 0x00;
pub const DOM_KEY_LOCATION_LEFT: libc::c_ulong = 0x01;
pub const DOM_KEY_LOCATION_RIGHT: libc::c_ulong = 0x02;
pub const DOM_KEY_LOCATION_NUMPAD: libc::c_ulong = 0x03;

#[repr(C)]
pub struct EmscriptenMouseEvent {
    pub timestamp: libc::c_double,
    pub screen_x: libc::c_long,
    pub screen_y: libc::c_long,
    pub client_x: libc::c_long,
    pub client_y: libc::c_long,
    pub ctrl_key: EM_BOOL,
    pub shift_key: EM_BOOL,
    pub alt_key: EM_BOOL,
    pub meta_key: EM_BOOL,
    pub button: libc::c_ushort,
    pub buttons: libc::c_ushort,
    pub movement_x: libc::c_long,
    pub movement_y: libc::c_long,
    pub target_x: libc::c_long,
    pub target_y: libc::c_long,
    pub canvas_x: libc::c_long,
    pub canvas_y: libc::c_long,
    padding: libc::c_long
}

#[repr(C)]
pub struct EmscriptenKeyboardEvent {
  pub key: [EM_UTF8; EM_HTML5_SHORT_STRING_LEN_BYTES],
  pub code: [EM_UTF8; EM_HTML5_SHORT_STRING_LEN_BYTES],
  pub location: libc::c_ulong,
  pub ctrlKey: EM_BOOL,
  pub shiftKey: EM_BOOL,
  pub altKey: EM_BOOL,
  pub metaKey: EM_BOOL,
  pub repeat: EM_BOOL,
  pub locale: [EM_UTF8; EM_HTML5_SHORT_STRING_LEN_BYTES],
  pub charValue: [EM_UTF8; EM_HTML5_SHORT_STRING_LEN_BYTES],
  pub charCode: libc::c_ulong,
  pub keyCode: libc::c_ulong,
  pub which: libc::c_ulong,
}

pub type em_mouse_callback_func = extern fn(libc::c_int, *const EmscriptenMouseEvent, *mut libc::c_void) 
    -> EM_BOOL;
pub type em_keyboard_callback_func = extern fn(libc::c_int, *const EmscriptenKeyboardEvent, *mut libc::c_void)
    -> EM_BOOL;

pub type em_pointerlockchange_callback_func = Option<unsafe extern "C" fn(eventType: libc::c_int, pointerlockChangeEvent: *const EmscriptenPointerlockChangeEvent, userData: *mut libc::c_void) -> EM_BOOL>;

#[repr(C)]
pub struct EmscriptenWebGLContextAttributes {
    pub alpha: EM_BOOL,
    pub depth: EM_BOOL,
    pub stencil: EM_BOOL,
    pub antialias: EM_BOOL,
    pub premultipliedAlpha: EM_BOOL,
    pub preserveDrawingBuffer: EM_BOOL,
    pub preferLowPowerToHighPerformance: EM_BOOL,
    pub failIfMajorPerformanceCaveat: EM_BOOL,
    pub majorVersion: libc::c_int,
    pub minorVersion: libc::c_int,
    pub enableExtensionsByDefault: EM_BOOL,
}

// values for EMSCRIPTEN_RESULT
pub const EMSCRIPTEN_RESULT_SUCCESS: libc::c_int = 0;
pub const EMSCRIPTEN_RESULT_DEFERRED: libc::c_int = 1;
pub const EMSCRIPTEN_RESULT_NOT_SUPPORTED: libc::c_int = -1;
pub const EMSCRIPTEN_RESULT_FAILED_NOT_DEFERRED: libc::c_int = -2;
pub const EMSCRIPTEN_RESULT_INVALID_TARGET: libc::c_int = -3;
pub const EMSCRIPTEN_RESULT_UNKNOWN_TARGET: libc::c_int = -4;
pub const EMSCRIPTEN_RESULT_INVALID_PARAM: libc::c_int = -5;
pub const EMSCRIPTEN_RESULT_FAILED: libc::c_int = -6;
pub const EMSCRIPTEN_RESULT_NO_DATA: libc::c_int = -7;

extern {
    pub fn emscripten_webgl_init_context_attributes(attributes: *mut EmscriptenWebGLContextAttributes);
    pub fn emscripten_webgl_create_context(target: *const libc::c_char,
        attributes: *const EmscriptenWebGLContextAttributes) -> EMSCRIPTEN_WEBGL_CONTEXT_HANDLE;

    pub fn emscripten_webgl_make_context_current(context: EMSCRIPTEN_WEBGL_CONTEXT_HANDLE)
    -> EMSCRIPTEN_RESULT;

    pub fn emscripten_webgl_get_current_context() -> EMSCRIPTEN_WEBGL_CONTEXT_HANDLE;

    pub fn emscripten_webgl_destroy_context(context: EMSCRIPTEN_WEBGL_CONTEXT_HANDLE)
        -> EMSCRIPTEN_RESULT;

    pub fn emscripten_webgl_enable_extension(context: EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
        extension: *const libc::c_char) -> EM_BOOL;

    pub fn emscripten_set_webglcontextlost_callback(target: *const libc::c_char,
        userData: *mut libc::c_void, useCapture: EM_BOOL, callback: em_webgl_context_callback)
        -> EMSCRIPTEN_RESULT;
    pub fn emscripten_set_webglcontextrestored_callback(target: *const libc::c_char,
        userData: *mut libc::c_void, useCapture: EM_BOOL, callback: em_webgl_context_callback)
        -> EMSCRIPTEN_RESULT;

    pub fn emscripten_is_webgl_context_lost(target: *const libc::c_char) -> EM_BOOL;

    // note: this function is not documented but is used by the ports of glfw, SDL and EGL
    pub fn emscripten_GetProcAddress(name: *const libc::c_char) -> *const libc::c_void;

    pub fn emscripten_request_pointerlock(target: *const libc::c_char,
        deferUntilInEventHandler: EM_BOOL) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_exit_pointerlock() -> EMSCRIPTEN_RESULT;

    pub fn emscripten_request_fullscreen(target: *const libc::c_char,
        deferUntilInEventHandler: EM_BOOL) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_exit_fullscreen() -> EMSCRIPTEN_RESULT;

    pub fn emscripten_sleep(delay: libc::c_uint);

    pub fn emscripten_set_main_loop(func : em_callback_func, fps : libc::c_int, simulate_infinite_loop : libc::c_int);


    pub fn emscripten_set_mousemove_callback(target: *const libc::c_char, user_data: *mut libc::c_void, use_capture: EM_BOOL, callback: em_mouse_callback_func) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_set_mousedown_callback(target: *const libc::c_char, user_data: *mut libc::c_void, use_capture: EM_BOOL, callback: em_mouse_callback_func) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_set_mouseup_callback(target: *const libc::c_char, user_data: *mut libc::c_void, use_capture: EM_BOOL, callback: em_mouse_callback_func) -> EMSCRIPTEN_RESULT;


    pub fn emscripten_set_keydown_callback(target: *const libc::c_char, user_data: *mut libc::c_void, use_capture: EM_BOOL, callback: em_keyboard_callback_func) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_set_keyup_callback(target: *const libc::c_char, user_data: *mut libc::c_void, use_capture: EM_BOOL, callback: em_keyboard_callback_func) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_get_device_pixel_ratio() -> f64;

    pub fn emscripten_set_canvas_size(width: libc::c_int, height: libc::c_int);

    pub fn emscripten_get_canvas_size(width: *mut libc::c_int, height: *mut libc::c_int, isFullscreen: *mut libc::c_int);

    pub fn emscripten_set_fullscreenchange_callback(target: *const libc::c_char, userData: *mut libc::c_void, useCapture: EM_BOOL, callback: em_fullscreenchange_callback_func) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_set_pointerlockchange_callback(target: *const libc::c_char, userData: *mut libc::c_void, useCapture: EM_BOOL, callback: em_pointerlockchange_callback_func) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_hide_mouse();

    pub fn emscripten_asm_const(code: *const libc::c_char);
}
