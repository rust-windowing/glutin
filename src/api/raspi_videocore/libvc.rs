use libc;
use std::ffi::CStr;
use std::mem;
use api::dlopen;

pub struct LibVc {
    libbcm: *mut libc::c_void,

    bcm_host_init: unsafe extern fn(),
    graphics_get_display_size: unsafe extern fn(libc::uint16_t, *mut libc::uint32_t,
                                                *mut libc::uint32_t) -> libc::int32_t,
    vc_dispmanx_display_open: unsafe extern fn(libc::uint32_t) -> libc::uint32_t,
    vc_dispmanx_display_close: unsafe extern fn(libc::uint32_t) -> libc::c_int,
    vc_dispmanx_update_start: unsafe extern fn(libc::int32_t) -> libc::uint32_t,
    vc_dispmanx_element_add: unsafe extern fn(libc::uint32_t, libc::uint32_t, libc::int32_t,
                                              *const VC_RECT_T, libc::uint32_t, *const VC_RECT_T,
                                              libc::uint32_t, *const VC_DISPMANX_ALPHA_T,
                                              *const DISPMANX_CLAMP_T, libc::uint32_t)
                                              -> libc::uint32_t,
    vc_dispmanx_update_submit_sync: unsafe extern fn(libc::uint32_t) -> libc::c_int,
}

#[repr(C)]
pub struct EGL_DISPMANX_WINDOW_T {
    pub element: libc::uint32_t,
    pub width: libc::c_int,
    pub height: libc::c_int,
}

#[repr(C)]
pub struct VC_RECT_T {
    pub x: libc::int32_t,
    pub y: libc::int32_t,
    pub width: libc::int32_t,
    pub height: libc::int32_t,
}

#[repr(C)]
pub struct VC_DISPMANX_ALPHA_T {
    pub flags: libc::uint32_t,
    pub opacity: libc::uint32_t,
    pub mask: libc::uint32_t,
}

#[repr(C)]
pub struct DISPMANX_CLAMP_T {
    pub mode: libc::uint32_t,
    pub key_mask: libc::uint32_t,
    pub key_value: DISPMANX_CLAMP_KEYS_T,
    pub replace_value: libc::uint32_t,
}

#[repr(C)]
pub struct DISPMANX_CLAMP_KEYS_T {
    pub red_upper: libc::uint8_t,
    pub red_lower: libc::uint8_t,
    pub blue_upper: libc::uint8_t,
    pub blue_lower: libc::uint8_t,
    pub green_upper: libc::uint8_t,
    pub green_lower: libc::uint8_t,
}

pub const DISPMANX_FLAGS_ALPHA_FROM_SOURCE: libc::uint32_t = 0;
pub const DISPMANX_FLAGS_ALPHA_FIXED_ALL_PIXELS: libc::uint32_t = 1;
pub const DISPMANX_FLAGS_ALPHA_FIXED_NON_ZERO: libc::uint32_t = 2;
pub const DISPMANX_FLAGS_ALPHA_FIXED_EXCEED_0X07: libc::uint32_t = 3;
pub const DISPMANX_FLAGS_ALPHA_PREMULT: libc::uint32_t = 1 << 16;
pub const DISPMANX_FLAGS_ALPHA_MIX: libc::uint32_t = 1 << 1;

pub const DISPMANX_NO_ROTATE: libc::uint32_t = 0;
pub const DISPMANX_ROTATE_90: libc::uint32_t = 1;
pub const DISPMANX_ROTATE_180: libc::uint32_t = 2;
pub const DISPMANX_ROTATE_270: libc::uint32_t = 3;
pub const DISPMANX_FLIP_HRIZ: libc::uint32_t = 1 << 16;
pub const DISPMANX_FLIP_VERT: libc::uint32_t = 1 << 17;
pub const DISPMANX_STEREOSCOPIC_INVERT: libc::uint32_t = 1 << 19;
pub const DISPMANX_STEREOSCOPIC_NONE: libc::uint32_t = 0 << 20;
pub const DISPMANX_STEREOSCOPIC_MONO: libc::uint32_t = 1 << 20;
pub const DISPMANX_STEREOSCOPIC_SBS: libc::uint32_t = 2 << 20;
pub const DISPMANX_STEREOSCOPIC_TB: libc::uint32_t = 3 << 20;
pub const DISPMANX_STEREOSCOPIC_MASK: libc::uint32_t = 15 << 20;
pub const DISPMANX_SNAPSHOT_NO_YUV: libc::uint32_t = 1 << 24;
pub const DISPMANX_SNAPSHOT_NO_RGB: libc::uint32_t = 1 << 25;
pub const DISPMANX_SNAPSHOT_FILL: libc::uint32_t = 1 << 26;
pub const DISPMANX_SNAPSHOT_SWAP_RED_BLUE: libc::uint32_t = 1 << 27;
pub const DISPMANX_SNAPSHOT_PACK: libc::uint32_t = 1 << 2;

pub const DISPMANX_FLAGS_CLAMP_NONE: libc::uint32_t = 0;
pub const DISPMANX_FLAGS_CLAMP_LUMA_TRANSPARENT: libc::uint32_t = 1;
pub const DISPMANX_FLAGS_CLAMP_TRANSPARENT: libc::uint32_t = 2;
pub const DISPMANX_FLAGS_CLAMP_REPLACE: libc::uint32_t = 3;

pub const DISPMANX_FLAGS_KEYMASK_OVERRIDE: libc::uint32_t = 1;
pub const DISPMANX_FLAGS_KEYMASK_SMOOTH: libc::uint32_t = 1 << 1;
pub const DISPMANX_FLAGS_KEYMASK_CR_INV: libc::uint32_t = 1 << 2;
pub const DISPMANX_FLAGS_KEYMASK_CB_INV: libc::uint32_t = 1 << 3;
pub const DISPMANX_FLAGS_KEYMASK_YY_INV: libc::uint32_t = 1 << 4;

pub const DISPMANX_PROTECTION_MAX: libc::uint32_t = 0x0f;
pub const DISPMANX_PROTECTION_NONE: libc::uint32_t = 0;
pub const DISPMANX_PROTECTION_HDCP: libc::uint32_t = 11;

#[derive(Debug)]
pub struct OpenError {
    reason: String
}

impl LibVc {
    pub fn open() -> Result<LibVc, OpenError> {
        let libbcm = unsafe { dlopen::dlopen(b"/opt/vc/lib/libbcm_host.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };

        if libbcm.is_null() {
            let cstr = unsafe { CStr::from_ptr(dlopen::dlerror()) };
            let reason = String::from_utf8(cstr.to_bytes().to_vec()).unwrap();
            return Err(OpenError { reason: reason });
        }

        let bcm_host_init = unsafe { dlopen::dlsym(libbcm, b"bcm_host_init\0".as_ptr() as *const _) };
        if bcm_host_init.is_null() {
            return Err(OpenError { reason: "Could not find symbol bcm_host_init in libbcm_host".to_string() });
        }

        let graphics_get_display_size = unsafe { dlopen::dlsym(libbcm, b"graphics_get_display_size\0".as_ptr() as *const _) };
        if graphics_get_display_size.is_null() {
            return Err(OpenError { reason: "Could not find symbol graphics_get_display_size in libbcm_host".to_string() });
        }

        let vc_dispmanx_display_open = unsafe { dlopen::dlsym(libbcm, b"vc_dispmanx_display_open\0".as_ptr() as *const _) };
        if vc_dispmanx_display_open.is_null() {
            return Err(OpenError { reason: "Could not find symbol vc_dispmanx_display_open in libbcm_host".to_string() });
        }

        let vc_dispmanx_display_close = unsafe { dlopen::dlsym(libbcm, b"vc_dispmanx_display_close\0".as_ptr() as *const _) };
        if vc_dispmanx_display_close.is_null() {
            return Err(OpenError { reason: "Could not find symbol vc_dispmanx_display_close in libbcm_host".to_string() });
        }

        let vc_dispmanx_update_start = unsafe { dlopen::dlsym(libbcm, b"vc_dispmanx_update_start\0".as_ptr() as *const _) };
        if vc_dispmanx_update_start.is_null() {
            return Err(OpenError { reason: "Could not find symbol vc_dispmanx_update_start in libbcm_host".to_string() });
        }

        let vc_dispmanx_element_add = unsafe { dlopen::dlsym(libbcm, b"vc_dispmanx_element_add\0".as_ptr() as *const _) };
        if vc_dispmanx_element_add.is_null() {
            return Err(OpenError { reason: "Could not find symbol vc_dispmanx_element_add in libbcm_host".to_string() });
        }

        let vc_dispmanx_update_submit_sync = unsafe { dlopen::dlsym(libbcm, b"vc_dispmanx_update_submit_sync\0".as_ptr() as *const _) };
        if vc_dispmanx_update_submit_sync.is_null() {
            return Err(OpenError { reason: "Could not find symbol vc_dispmanx_update_submit_sync in libbcm_host".to_string() });
        }

        let vcm = LibVc {
            libbcm: libbcm,

            bcm_host_init: unsafe { mem::transmute(bcm_host_init) },
            graphics_get_display_size: unsafe { mem::transmute(graphics_get_display_size) },
            vc_dispmanx_display_open: unsafe { mem::transmute(vc_dispmanx_display_open) },
            vc_dispmanx_display_close: unsafe { mem::transmute(vc_dispmanx_display_close) },
            vc_dispmanx_update_start: unsafe { mem::transmute(vc_dispmanx_update_start) },
            vc_dispmanx_element_add: unsafe { mem::transmute(vc_dispmanx_element_add) },
            vc_dispmanx_update_submit_sync: unsafe { mem::transmute(vc_dispmanx_update_submit_sync) },
        };

        Ok(vcm)
    }

    pub unsafe fn bcm_host_init(&self) {
        (self.bcm_host_init)()
    }

    pub unsafe fn graphics_get_display_size(&self, display_number: libc::uint16_t,
                                            width: *mut libc::uint32_t,
                                            height: *mut libc::uint32_t) -> libc::int32_t
    {
        (self.graphics_get_display_size)(display_number, width, height)
    }

    pub unsafe fn vc_dispmanx_display_open(&self, device: libc::uint32_t) -> libc::uint32_t {
        (self.vc_dispmanx_display_open)(device)
    }

    pub unsafe fn vc_dispmanx_display_close(&self, display: libc::uint32_t) -> libc::c_int {
        (self.vc_dispmanx_display_close)(display)
    }

    pub unsafe fn vc_dispmanx_update_start(&self, priority: libc::int32_t) -> libc::uint32_t {
        (self.vc_dispmanx_update_start)(priority)
    }

    pub unsafe fn vc_dispmanx_element_add(&self, update: libc::uint32_t, display: libc::uint32_t,
                                          layer: libc::int32_t, dest_rect: *const VC_RECT_T,
                                          src: libc::uint32_t, src_rect: *const VC_RECT_T,
                                          protection: libc::uint32_t,
                                          alpha: *const VC_DISPMANX_ALPHA_T,
                                          clamp: *const DISPMANX_CLAMP_T,
                                          transform: libc::uint32_t) -> libc::uint32_t
    {
        (self.vc_dispmanx_element_add)(update, display, layer, dest_rect, src, src_rect,
                                       protection, alpha, clamp, transform)
    }

    pub unsafe fn vc_dispmanx_update_submit_sync(&self, update: libc::uint32_t) -> libc::c_int {
        (self.vc_dispmanx_update_submit_sync)(update)
    }
}

impl Drop for LibVc {
    fn drop(&mut self) {
        unsafe { dlopen::dlclose(self.libbcm); }
    }
}
