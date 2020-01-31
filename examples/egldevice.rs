#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
mod support;

mod implementation {
    use super::support;

    use glutin::config::{Api, ConfigsFinder, Version};
    use glutin::context::ContextBuilder;
    use glutin::surface::Surface;
    use glutin_interface::{NativeDisplay, RawDisplay, Seal};
    use winit_types::dpi::PhysicalSize;

    use std::ffi::CStr;
    use std::path::Path;

    pub fn main() {
        let lib = libloading::Library::new("libEGL.so.1")
            .unwrap_or_else(|_| libloading::Library::new("libEGL.so").unwrap());
        let egl =
            glutin_egl_sys::egl::Egl::load_with(|f| unsafe { support::load_egl_sym(&lib, f) });

        let devices: Vec<glutin_egl_sys::egl::types::EGLDeviceEXT> = unsafe {
            let mut num_device = 0;
            egl.QueryDevicesEXT(0, std::ptr::null_mut(), &mut num_device);

            let mut devices = Vec::with_capacity(num_device as usize);
            devices.resize_with(num_device as usize, || std::mem::zeroed());

            let mut new_num_device = 0;
            egl.QueryDevicesEXT(num_device, devices.as_mut_ptr(), &mut new_num_device);
            assert_eq!(num_device, new_num_device);
            devices
        };

        println!("Got devices: {:?}.", &devices);

        struct DeviceDisplay(glutin_egl_sys::egl::types::EGLDeviceEXT);
        impl NativeDisplay for DeviceDisplay {
            fn raw_display(&self) -> RawDisplay {
                RawDisplay::EglExtDevice {
                    egl_device_ext: self.0 as *mut _,
                    _non_exhaustive_do_not_use: Seal,
                }
            }
        }

        for (i, device) in devices.iter().enumerate() {
            let exts = unsafe {
                CStr::from_ptr(
                    egl.QueryDeviceStringEXT(*device, glutin_egl_sys::egl::EXTENSIONS as _),
                )
            };
            println!("Device {:?} has these exts: {:?}", device, exts);

            let nd = DeviceDisplay(*device);

            let mut choosen_confs = vec![];
            let mut confs = unsafe {
                ConfigsFinder::new()
                    .with_must_support_pbuffers(true)
                    .with_must_support_windows(false)
                    .with_gl((Api::OpenGl, Version(3, 3)))
                    .find(&nd)
                    .unwrap()
            };
            let conf = confs.drain(..1).next().unwrap();
            choosen_confs.push(conf);

            let mut confs = unsafe {
                ConfigsFinder::new()
                    .with_must_support_surfaceless(true)
                    .with_must_support_windows(false)
                    .with_gl((Api::OpenGl, Version(3, 3)))
                    .find(&nd)
                    .unwrap()
            };
            let conf = confs.drain(..1).next().unwrap();
            choosen_confs.push(conf);

            println!("Chose following confs {:#?}", choosen_confs);

            let size = PhysicalSize::new(512, 512);
            for (j, conf) in choosen_confs.iter().enumerate() {
                let ctx = unsafe { ContextBuilder::new().build(&conf).unwrap() };
                let surf = match i {
                    0 => unsafe { Some(Surface::new_pbuffer(&conf, &size, true).unwrap()) },
                    1 => None,
                    _ => unreachable!(),
                };

                let size = match i {
                    0 => surf.as_ref().unwrap().size().unwrap(),
                    1 => size.clone(),
                    _ => unreachable!(),
                };

                unsafe {
                    match i {
                        0 => ctx.make_current(&surf.as_ref().unwrap()),
                        1 => ctx.make_current_surfaceless(),
                        _ => unreachable!(),
                    }
                    .unwrap();
                }

                let gl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());

                let mut fb = None;
                let mut render_buf = None;
                match i {
                    0 => (),
                    1 => {
                        // Surfaceless doesn't come with a surface, as the name implies, so
                        // you must make your own fb.
                        render_buf = Some(gl.make_renderbuf(size));
                        fb = Some(gl.make_framebuffer(render_buf.unwrap()));
                    }
                    _ => unreachable!(),
                }

                unsafe {
                    gl.gl.Viewport(0, 0, size.width as _, size.height as _);
                }
                gl.draw_frame([0.0, 0.0, 0.0, 1.0]);

                gl.export_to_file(
                    &size,
                    &Path::new(
                        &("headless".to_string() + &i.to_string() + "_" + &j.to_string() + ".png"),
                    ),
                );

                match i {
                    0 => (),
                    1 => unsafe {
                        gl.gl.DeleteFramebuffers(1, &fb.unwrap());
                        gl.gl.DeleteRenderbuffers(1, &render_buf.unwrap());
                    },
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
)))]
mod implementation {
    pub fn main() {
        panic!("This example is for linux only.")
    }
}

fn main() {
    implementation::main();
}
