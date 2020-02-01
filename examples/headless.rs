mod support;

use support::HeadlessBackend;

use winit::event_loop::EventLoop;
use winit_types::dpi::PhysicalSize;

use std::path::Path;

fn main() {
    simple_logger::init().unwrap();
    let size = PhysicalSize::new(512, 512);
    let el = EventLoop::new();

    let (backend, size, _) = unsafe { HeadlessBackend::new(&el, size, false).unwrap() };
    let gl = backend.load_symbols().unwrap();

    let mut fb = None;
    let mut render_buf = None;
    match backend {
        HeadlessBackend::Surfaceless(_) => {
            // Surfaceless doesn't come with a surface, as the name implies, so
            // you must make your own fb.
            render_buf = Some(gl.make_renderbuf(size));
            fb = Some(gl.make_framebuffer(render_buf.unwrap()));
        }
        _ => (),
    }

    unsafe {
        gl.gl.Viewport(0, 0, size.width as _, size.height as _);
    }
    gl.draw_frame([1.0, 0.5, 0.7, 1.0]);

    gl.export_to_file(size, &Path::new("headless.png"));

    match backend {
        HeadlessBackend::Surfaceless(_) => unsafe {
            gl.gl.DeleteFramebuffers(1, &fb.unwrap());
            gl.gl.DeleteRenderbuffers(1, &render_buf.unwrap());
        },
        _ => (),
    }
}
