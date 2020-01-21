mod support;

use support::{gl, HeadlessBackend};

use winit::event_loop::EventLoop;
use winit_types::dpi::PhysicalSize;

use std::path::Path;

fn main() {
    env_logger::init();
    let size = PhysicalSize::new(512, 512);
    let el = EventLoop::new();

    let (backend, _) = HeadlessBackend::new(&el, &size, false).unwrap();
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

    let mut pixels: Vec<gl::types::GLubyte> = vec![];
    pixels.resize(3 * size.width as usize * size.height as usize, 0);
    unsafe {
        gl.gl.ReadPixels(
            0,
            0,
            size.width as _,
            size.height as _,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            pixels.as_mut_ptr() as *mut _,
        );
    }

    let mut pixels_flipped: Vec<gl::types::GLubyte> = vec![];
    for v in (0..size.height as _).rev() {
        let s = 3 * v as usize * size.width as usize;
        let o = 3 * size.width as usize;
        pixels_flipped.extend_from_slice(&pixels[s..(s + o)]);
    }

    image::save_buffer(
        &Path::new("headless.png"),
        &pixels_flipped,
        size.width as u32,
        size.height as u32,
        image::RGB(8),
    )
    .unwrap();

    match backend {
        HeadlessBackend::Surfaceless(_) => unsafe {
            gl.gl.DeleteFramebuffers(1, &fb.unwrap());
            gl.gl.DeleteRenderbuffers(1, &render_buf.unwrap());
        },
        _ => (),
    }
}
