use libc;
use std::ffi::CStr;
use std::mem;
use api::dlopen;

pub struct LibGbm {
    lib: *mut libc::c_void,

    create_device: unsafe extern fn(libc::c_int) -> *mut libc::c_void,
    device_destroy: unsafe extern fn(*mut libc::c_void),
    surface_create: unsafe extern fn(*mut libc::c_void, libc::uint32_t, libc::uint32_t,
                                     libc::uint32_t, libc::uint32_t) -> *mut libc::c_void,
    surface_destroy: unsafe extern fn(*mut libc::c_void),
}

#[derive(Debug)]
pub struct OpenError {
    reason: String
}

impl LibGbm {
    pub fn open() -> Result<LibGbm, OpenError> {
        let lib = unsafe { dlopen::dlopen(b"libgbm.so.1\0".as_ptr() as *const _, 2 /* RTLD_NOW */) };

        if lib.is_null() {
            let cstr = unsafe { CStr::from_ptr(dlopen::dlerror()) };
            let reason = String::from_utf8(cstr.to_bytes().to_vec()).unwrap();
            return Err(OpenError { reason: reason });
        }

        Ok(LibGbm {
            lib: lib,

            create_device: unsafe { mem::transmute(dlopen::dlsym(lib,
                                    b"gbm_create_device\0".as_ptr() as *const _)) },
            device_destroy: unsafe { mem::transmute(dlopen::dlsym(lib,
                                     b"gbm_device_destroy\0".as_ptr() as *const _)) },
            surface_create: unsafe { mem::transmute(dlopen::dlsym(lib,
                                     b"gbm_surface_create\0".as_ptr() as *const _)) },
            surface_destroy: unsafe { mem::transmute(dlopen::dlsym(lib,
                                      b"gbm_surface_destroy\0".as_ptr() as *const _)) },
        })
    }

    pub unsafe fn create_device(&self, fd: libc::c_int) -> *mut libc::c_void {
        (self.create_device)(fd)
    }

    pub unsafe fn device_destroy(&self, device: *mut libc::c_void) {
        (self.device_destroy)(device)
    }

    pub unsafe fn surface_create(&self, a: *mut libc::c_void, b: libc::uint32_t, c: libc::uint32_t,
                                 d: libc::uint32_t, e: libc::uint32_t) -> *mut libc::c_void {
        (self.surface_create)(a, b, c, d, e)
    }

    pub unsafe fn surface_destroy(&self, surface: *mut libc::c_void) {
        (self.surface_destroy)(surface)
    }
}

impl Drop for LibGbm {
    fn drop(&mut self) {
        unsafe { dlopen::dlclose(self.lib); }
    }
}
