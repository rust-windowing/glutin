// FIXME: windows impl

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "windows",
))]
pub mod support;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate glutin_x11_sym;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "windows",
))]
mod implementation {
    use super::support::{gl, Gl};

    mod ffi {
        pub use glutin_egl_sys::egl;
        pub use glutin_glx_sys::glx;
        pub use x11_dl::xlib::*;
    }

    use glutin::config::ConfigsFinder;
    use glutin::context::ContextBuilder;
    use glutin::surface::Surface;
    use glutin_interface::{NativePixmap, NativePixmapSource, RawPixmap, Seal, X11PixmapParts};
    use glutin_x11_sym::Display;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::platform::unix::{EventLoopExtUnix, EventLoopWindowTargetExtUnix};
    use winit::window::WindowBuilder;
    use winit_types::dpi::PhysicalSize;
    use winit_types::error::Error;

    use std::sync::Arc;

    pub fn main() {
        simple_logger::init().unwrap();
        let el = EventLoop::<()>::new_x11().unwrap();
        let mut size = PhysicalSize::new(720, 512);

        let mut confs = unsafe {
            ConfigsFinder::new()
                .with_must_support_pixmaps(true)
                .find(&*el)
                .unwrap()
        };
        let conf = confs.drain(..1).next().unwrap();
        println!("Configeration chosen: {:?}", conf);

        let ctx = unsafe { ContextBuilder::new().build(&conf).unwrap() };
        let wb = WindowBuilder::new()
            .with_title("A fantastic window!")
            .with_inner_size(size);
        let (win, wsurf) = unsafe { Surface::new_window(&conf, &*el, wb).unwrap() };

        unsafe { ctx.make_current(&wsurf).unwrap() }
        let gl = Gl::load(|s| ctx.get_proc_address(s).unwrap());

        let disp = Display::from_raw(el.xlib_display().unwrap());
        struct Pixmap(ffi::Pixmap);
        impl NativePixmap for Pixmap {
            fn raw_pixmap(&self) -> RawPixmap {
                RawPixmap::Xlib {
                    pixmap: self.0,
                    _non_exhaustive_do_not_use: Seal,
                }
            }
        }

        struct PixmapSource(Arc<Display>);
        impl NativePixmapSource for PixmapSource {
            type Pixmap = Pixmap;
            type PixmapBuilder = PhysicalSize<u32>;

            fn build_x11(
                &self,
                pb: Self::PixmapBuilder,
                xpp: X11PixmapParts,
            ) -> Result<Self::Pixmap, Error> {
                let xlib = syms!(XLIB);
                let pixmap = unsafe {
                    (xlib.XCreatePixmap)(
                        **self.0,
                        (xlib.XDefaultRootWindow)(**self.0),
                        pb.width,
                        pb.height,
                        xpp.depth as _,
                    )
                };
                self.0.check_errors()?;
                Ok(Pixmap(pixmap))
            }
        }
        let (mut pix, mut psurf) =
            unsafe { Surface::new_pixmap(&conf, &PixmapSource(Arc::clone(&disp)), size).unwrap() };

        el.run(move |event, _, control_flow| {
            println!("{:?}", event);
            *control_flow = ControlFlow::Wait;

            match event {
                Event::LoopDestroyed => return,
                Event::MainEventsCleared => {
                    win.request_redraw();
                }
                Event::RedrawRequested(_) => unsafe {
                    ctx.make_current(&psurf).unwrap();
                    gl.draw_frame([1.0, 0.5, 0.7, 1.0]);

                    ctx.make_current_rw(&psurf, &wsurf).unwrap();
                    gl.gl.BlitFramebuffer(
                        0,
                        0,
                        size.width as _,
                        size.height as _,
                        0,
                        0,
                        size.width as _,
                        size.height as _,
                        gl::COLOR_BUFFER_BIT,
                        gl::NEAREST,
                    );
                    wsurf.swap_buffers().unwrap();
                },
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(nsize) => unsafe {
                        size = *nsize;
                        let (npix, npsurf) =
                            Surface::new_pixmap(&conf, &PixmapSource(Arc::clone(&disp)), size)
                                .unwrap();
                        psurf = npsurf;
                        pix = npix;
                        ctx.make_current(&wsurf).unwrap();
                        wsurf.update_after_resize(&size);
                        gl.gl.Viewport(0, 0, size.width as _, size.height as _);
                    },
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => (),
                },
                _ => (),
            }
        });
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "windows",
),))]
mod implementation {
    pub fn main() {
        panic!("This example is for linux and windows only.")
    }
}

fn main() {
    implementation::main();
}
