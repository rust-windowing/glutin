use std::fs::OpenOptions;
use std::os::fd::AsRawFd;

use glutin::api::egl;
use raw_window_handle::{DrmDisplayHandle, RawDisplayHandle};

fn main() {
    let devices = egl::device::Device::query_devices().expect("Query EGL devices");
    for egl_device in devices {
        dbg!(&egl_device);
        dbg!(egl_device.drm_render_device_node_path());
        let Some(drm) = dbg!(egl_device.drm_device_node_path()) else {
            continue;
        };
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open(drm)
            .expect("Open DRM device with Read/Write permissions");

        // https://registry.khronos.org/EGL/extensions/EXT/EGL_EXT_device_drm.txt:
        // Providing DRM_MASTER_FD is only to cover cases where EGL might fail to open
        // it itself.
        let rdh = RawDisplayHandle::Drm(DrmDisplayHandle::new(fd.as_raw_fd()));

        let egl_display = unsafe { egl::display::Display::with_device(&egl_device, Some(rdh)) }
            .expect("Create EGL Display");
        dbg!(&egl_display);
    }
}
