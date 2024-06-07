//! Manual implementation of EGL bindings.
//!
//! This is necessary since `gl_generator` is unmaintaned and incapable of
//! generating bindings for some of the newer extensions.

use std::ffi::c_uint;

pub type khronos_utime_nanoseconds_t = super::khronos_utime_nanoseconds_t;
pub type khronos_uint64_t = super::khronos_uint64_t;
pub type khronos_ssize_t = super::khronos_ssize_t;
pub type EGLNativeDisplayType = super::EGLNativeDisplayType;
pub type EGLNativePixmapType = super::EGLNativePixmapType;
pub type EGLNativeWindowType = super::EGLNativeWindowType;
pub type EGLint = super::EGLint;
pub type NativeDisplayType = super::EGLNativeDisplayType;
pub type NativePixmapType = super::EGLNativePixmapType;
pub type NativeWindowType = super::EGLNativeWindowType;

include!(concat!(env!("OUT_DIR"), "/egl_bindings.rs"));

// EGL_EXT_platform_xcb
pub const PLATFORM_XCB_EXT: super::EGLenum = 0x31DC;
pub const PLATFORM_XCB_SCREEN_EXT: super::EGLenum = 0x31DE;
// EGL_EXT_device_query_name
pub const RENDERER_EXT: super::EGLenum = 0x335F;
// EGL_ANGLE_platform_angle - https://chromium.googlesource.com/angle/angle/+/HEAD/extensions/EGL_ANGLE_platform_angle.txt
pub const PLATFORM_ANGLE_ANGLE: super::EGLenum = 0x3202;
pub const PLATFORM_ANGLE_TYPE_ANGLE: super::EGLenum = 0x3203;
pub const PLATFORM_ANGLE_TYPE_VULKAN_ANGLE: super::EGLenum = 0x3450;
pub const PLATFORM_ANGLE_MAX_VERSION_MAJOR_ANGLE: super::EGLenum = 0x3204;
pub const PLATFORM_ANGLE_MAX_VERSION_MINOR_ANGLE: super::EGLenum = 0x3205;
pub const PLATFORM_ANGLE_DEBUG_LAYERS_ENABLED: super::EGLenum = 0x3451;
pub const PLATFORM_ANGLE_NATIVE_PLATFORM_TYPE_ANGLE: super::EGLenum = 0x348F;
pub const PLATFORM_ANGLE_TYPE_DEFAULT_ANGLE: super::EGLenum = 0x3206;
pub const PLATFORM_ANGLE_DEVICE_TYPE_HARDWARE_ANGLE: super::EGLenum = 0x320A;
pub const PLATFORM_ANGLE_DEVICE_TYPE_NULL_ANGLE: super::EGLenum = 0x345E;

mod wayland_storage {
    use super::FnPtr;
    use super::__gl_imports::raw;

    // EGL_WL_create_wayland_buffer_from_image
    pub static mut CREATE_WAYLAND_BUFFER_FROM_IMAGE_WL: FnPtr =
        FnPtr { f: super::missing_fn_panic as *const raw::c_void, is_loaded: false };

    // EGL_WL_bind_wayland_display
    pub static mut BIND_WAYLAND_DISPLAY_WL: FnPtr =
        FnPtr { f: super::missing_fn_panic as *const raw::c_void, is_loaded: false };
    pub static mut UNBIND_WAYLAND_DISPLAY_WL: FnPtr =
        FnPtr { f: super::missing_fn_panic as *const raw::c_void, is_loaded: false };
    pub static mut QUERY_WAYLAND_BUFFER_WL: FnPtr =
        FnPtr { f: super::missing_fn_panic as *const raw::c_void, is_loaded: false };
}

impl Egl {
    #[allow(non_snake_case, unused_variables, dead_code)]
    #[inline]
    pub unsafe fn CreateWaylandBufferFromImageWL(
        &self,
        dpy: types::EGLDisplay,
        image: types::EGLImageKHR,
    ) -> *mut std::ffi::c_void {
        __gl_imports::mem::transmute::<
            _,
            extern "system" fn(types::EGLDisplay, types::EGLImageKHR) -> *mut std::ffi::c_void,
        >(wayland_storage::CREATE_WAYLAND_BUFFER_FROM_IMAGE_WL.f)(dpy, image)
    }

    #[allow(non_snake_case, unused_variables, dead_code)]
    #[inline]
    pub unsafe fn BindWaylandDisplayWL(
        &self,
        dpy: types::EGLDisplay,
        display: *mut __gl_imports::raw::c_void,
    ) -> types::EGLBoolean {
        __gl_imports::mem::transmute::<
            _,
            extern "system" fn(
                types::EGLDisplay,
                *mut __gl_imports::raw::c_void,
            ) -> types::EGLBoolean,
        >(wayland_storage::BIND_WAYLAND_DISPLAY_WL.f)(dpy, display)
    }

    #[allow(non_snake_case, unused_variables, dead_code)]
    #[inline]
    pub unsafe fn UnbindWaylandDisplayWL(
        &self,
        dpy: types::EGLDisplay,
        display: *mut __gl_imports::raw::c_void,
    ) -> types::EGLBoolean {
        __gl_imports::mem::transmute::<
            _,
            extern "system" fn(
                types::EGLDisplay,
                *mut __gl_imports::raw::c_void,
            ) -> types::EGLBoolean,
        >(wayland_storage::UNBIND_WAYLAND_DISPLAY_WL.f)(dpy, display)
    }

    #[allow(non_snake_case, unused_variables, dead_code)]
    #[inline]
    pub unsafe fn QueryWaylandBufferWL(
        &self,
        dpy: types::EGLDisplay,
        buffer: *mut __gl_imports::raw::c_void,
        attribute: types::EGLint,
        value: *mut types::EGLint,
    ) -> types::EGLBoolean {
        __gl_imports::mem::transmute::<
            _,
            extern "system" fn(
                types::EGLDisplay,
                *mut __gl_imports::raw::c_void,
                types::EGLint,
                *mut types::EGLint,
            ) -> types::EGLBoolean,
        >(wayland_storage::QUERY_WAYLAND_BUFFER_WL.f)(dpy, buffer, attribute, value)
    }
}

// Extension: EGL_WL_create_wayland_buffer_from_image
//

#[allow(non_snake_case)]
pub mod CreateWaylandBufferFromImageWL {
    use super::__gl_imports::raw;
    use super::{metaloadfn, wayland_storage, FnPtr};

    #[inline]
    #[allow(dead_code)]
    pub fn is_loaded() -> bool {
        unsafe { wayland_storage::CREATE_WAYLAND_BUFFER_FROM_IMAGE_WL.is_loaded }
    }

    #[allow(dead_code)]
    pub fn load_with<F>(mut loadfn: F)
    where
        F: FnMut(&'static str) -> *const raw::c_void,
    {
        unsafe {
            wayland_storage::CREATE_WAYLAND_BUFFER_FROM_IMAGE_WL =
                FnPtr::new(metaloadfn(&mut loadfn, "eglCreateWaylandBufferFromImageWL", &[]))
        }
    }
}

// Extension: EGL_WL_bind_wayland_display
//

// Accepted as <target> in eglCreateImageKHR.
pub const WAYLAND_BUFFER_WL: c_uint = 0x31D5;
// Accepted in the <attrib_list> parameter of eglCreateImageKHR.
pub const WAYLAND_PLANE_WL: c_uint = 0x31D6;
// Possible values for EGL_TEXTURE_FORMAT.
pub const TEXTURE_Y_U_V_WL: i32 = 0x31D7;
pub const TEXTURE_Y_UV_WL: i32 = 0x31D8;
pub const TEXTURE_Y_XUXV_WL: i32 = 0x31D9;
pub const TEXTURE_EXTERNAL_WL: i32 = 0x31DA;
pub const WAYLAND_Y_INVERTED_WL: i32 = 0x31DB;

#[allow(non_snake_case)]
pub mod BindWaylandDisplayWL {
    use super::__gl_imports::raw;
    use super::{metaloadfn, wayland_storage, FnPtr};

    #[inline]
    #[allow(dead_code)]
    pub fn is_loaded() -> bool {
        unsafe { wayland_storage::BIND_WAYLAND_DISPLAY_WL.is_loaded }
    }

    #[allow(dead_code)]
    pub fn load_with<F>(mut loadfn: F)
    where
        F: FnMut(&'static str) -> *const raw::c_void,
    {
        unsafe {
            wayland_storage::BIND_WAYLAND_DISPLAY_WL =
                FnPtr::new(metaloadfn(&mut loadfn, "eglBindWaylandDisplayWL", &[]))
        }
    }
}

#[allow(non_snake_case)]
pub mod UnbindWaylandDisplayWL {
    use super::__gl_imports::raw;
    use super::{metaloadfn, wayland_storage, FnPtr};

    #[inline]
    #[allow(dead_code)]
    pub fn is_loaded() -> bool {
        unsafe { wayland_storage::UNBIND_WAYLAND_DISPLAY_WL.is_loaded }
    }

    #[allow(dead_code)]
    pub fn load_with<F>(mut loadfn: F)
    where
        F: FnMut(&'static str) -> *const raw::c_void,
    {
        unsafe {
            wayland_storage::UNBIND_WAYLAND_DISPLAY_WL =
                FnPtr::new(metaloadfn(&mut loadfn, "eglUnbindWaylandDisplayWL", &[]))
        }
    }
}

#[allow(non_snake_case)]
pub mod QueryWaylandBufferWL {
    use super::__gl_imports::raw;
    use super::{metaloadfn, wayland_storage, FnPtr};

    #[inline]
    #[allow(dead_code)]
    pub fn is_loaded() -> bool {
        unsafe { wayland_storage::QUERY_WAYLAND_BUFFER_WL.is_loaded }
    }

    #[allow(dead_code)]
    pub fn load_with<F>(mut loadfn: F)
    where
        F: FnMut(&'static str) -> *const raw::c_void,
    {
        unsafe {
            wayland_storage::QUERY_WAYLAND_BUFFER_WL =
                FnPtr::new(metaloadfn(&mut loadfn, "eglQueryWaylandBufferWL", &[]))
        }
    }
}

/// OpenGL function loader.
///
/// This is based on the loader generated by `gl_generator`.
#[inline(never)]
fn metaloadfn(
    loadfn: &mut dyn FnMut(&'static str) -> *const __gl_imports::raw::c_void,
    symbol: &'static str,
    fallbacks: &[&'static str],
) -> *const __gl_imports::raw::c_void {
    let mut ptr = loadfn(symbol);
    if ptr.is_null() {
        for &sym in fallbacks {
            ptr = loadfn(sym);
            if !ptr.is_null() {
                break;
            }
        }
    }
    ptr
}
