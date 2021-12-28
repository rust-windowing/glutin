#![cfg(target_os = "emscripten")]
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use std::os::raw;

pub type EM_BOOL = raw::c_int;
pub type EM_UTF8 = raw::c_char;
pub type EMSCRIPTEN_WEBGL_CONTEXT_HANDLE = raw::c_int;
pub type EMSCRIPTEN_RESULT = raw::c_int;

pub type em_webgl_context_callback =
    extern "C" fn(raw::c_int, *const raw::c_void, *mut raw::c_void) -> EM_BOOL;

pub type em_callback_func = unsafe extern "C" fn();

#[repr(C)]
#[derive(Debug)]
pub struct EmscriptenWebGLContextAttributes {
    pub alpha: EM_BOOL,
    pub depth: EM_BOOL,
    pub stencil: EM_BOOL,
    pub antialias: EM_BOOL,
    pub premultipliedAlpha: EM_BOOL,
    pub preserveDrawingBuffer: EM_BOOL,
    pub preferLowPowerToHighPerformance: EM_BOOL,
    pub failIfMajorPerformanceCaveat: EM_BOOL,
    pub majorVersion: raw::c_int,
    pub minorVersion: raw::c_int,
    pub enableExtensionsByDefault: EM_BOOL,
    pub explicitSwapControl: EM_BOOL,
}

// values for EMSCRIPTEN_RESULT
pub const EMSCRIPTEN_RESULT_SUCCESS: raw::c_int = 0;
pub const EMSCRIPTEN_RESULT_DEFERRED: raw::c_int = 1;
pub const EMSCRIPTEN_RESULT_NOT_SUPPORTED: raw::c_int = -1;
pub const EMSCRIPTEN_RESULT_FAILED_NOT_DEFERRED: raw::c_int = -2;
pub const EMSCRIPTEN_RESULT_INVALID_TARGET: raw::c_int = -3;
pub const EMSCRIPTEN_RESULT_UNKNOWN_TARGET: raw::c_int = -4;
pub const EMSCRIPTEN_RESULT_INVALID_PARAM: raw::c_int = -5;
pub const EMSCRIPTEN_RESULT_FAILED: raw::c_int = -6;
pub const EMSCRIPTEN_RESULT_NO_DATA: raw::c_int = -7;

extern "C" {
    pub fn emscripten_webgl_init_context_attributes(
        attributes: *mut EmscriptenWebGLContextAttributes,
    );
    pub fn emscripten_webgl_create_context(
        target: *const raw::c_char,
        attributes: *const EmscriptenWebGLContextAttributes,
    ) -> EMSCRIPTEN_WEBGL_CONTEXT_HANDLE;

    pub fn emscripten_webgl_make_context_current(
        context: EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
    ) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_webgl_get_current_context() -> EMSCRIPTEN_WEBGL_CONTEXT_HANDLE;

    pub fn emscripten_webgl_destroy_context(
        context: EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
    ) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_webgl_enable_extension(
        context: EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
        extension: *const raw::c_char,
    ) -> EM_BOOL;

    pub fn emscripten_set_webglcontextlost_callback(
        target: *const raw::c_char,
        userData: *mut raw::c_void,
        useCapture: EM_BOOL,
        callback: em_webgl_context_callback,
    ) -> EMSCRIPTEN_RESULT;
    pub fn emscripten_set_webglcontextrestored_callback(
        target: *const raw::c_char,
        userData: *mut raw::c_void,
        useCapture: EM_BOOL,
        callback: em_webgl_context_callback,
    ) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_is_webgl_context_lost(target: *const raw::c_char) -> EM_BOOL;

    // note: this function is not documented but is used by the ports of glfw,
    // SDL and EGL
    pub fn emscripten_GetProcAddress(name: *const raw::c_char) -> *const raw::c_void;

    pub fn emscripten_request_fullscreen(
        target: *const raw::c_char,
        deferUntilInEventHandler: EM_BOOL,
    ) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_exit_fullscreen() -> EMSCRIPTEN_RESULT;

    pub fn emscripten_set_element_css_size(
        target: *const raw::c_char,
        width: raw::c_double,
        height: raw::c_double,
    ) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_get_element_css_size(
        target: *const raw::c_char,
        width: *mut raw::c_double,
        height: *mut raw::c_double,
    ) -> EMSCRIPTEN_RESULT;

    pub fn emscripten_sleep(delay: raw::c_uint);

    pub fn emscripten_set_main_loop(
        func: em_callback_func,
        fps: raw::c_int,
        simulate_infinite_loop: raw::c_int,
    );
}
