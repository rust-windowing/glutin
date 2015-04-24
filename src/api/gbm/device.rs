use super::libgbm;
use libc;

pub struct GbmDevice {
    lib: libgbm::LibGbm,
    device: *mut libc::c_void,
    surface: *mut libc::c_void,
}

impl GbmDevice {
    pub fn open() -> GbmDevice {
        let lib = libgbm::LibGbm::open().unwrap();

        let device = unsafe { 
            let fd = libc::open(b"/dev/dri/card0\0".as_ptr() as *const _, libc::O_RDWR, libc::S_IRUSR | libc::S_IWUSR | libc::S_IRGRP | libc::S_IROTH);

            if fd < 0 {
                panic!("Failed to open /dev/dri/card0");
            }

            let device = lib.create_device(fd);
            if device.is_null() {
                panic!("gbm_create_device failed");
            }
            device
        };

        let surface = unsafe {
            let surface = lib.surface_create(device,
                            256, 256,
                            ('X' as u32 | ('R' as u32) << 8 | 2 << 16 | 4 << 24) /* GBM_FORMAT_XRGB8888 */,
                            1 << 2/* GBM_BO_USE_RENDERING */);
            if surface.is_null() {
                panic!("gbm_surface_create failed");
            }
            surface
        };

        GbmDevice {
            lib: lib,
            device: device,
            surface: surface,
        }
    }

    pub fn get_device(&self) -> *mut libc::c_void {
        self.device
    }

    pub fn get_surface(&self) -> *mut libc::c_void {
        self.surface
    }
}

impl Drop for GbmDevice {
    fn drop(&mut self) {
        unsafe {
            self.lib.surface_destroy(self.surface);
            self.lib.device_destroy(self.device);
        };
    }
}
