#[cfg(target_os = "linux")]
mod support;

fn main() {
    #[cfg(not(target_os = "linux"))]
    unimplemented!();
    #[cfg(target_os = "linux")]
    this_example::main();
}

#[cfg(target_os = "linux")]
mod this_example {
    use super::support;
    use glutin::ContextTrait;
    use std::path::Path;
    use support::gl;

    pub fn main() {
        use glutin::os::unix::OsMesaContextExt;

        let cb = glutin::ContextBuilder::new()
            .with_gl_profile(glutin::GlProfile::Core)
            .with_gl(glutin::GlRequest::Latest);
        let dims = glutin::dpi::PhysicalSize::new(840., 640.);
        let os_mesa = glutin::Context::new_osmesa(cb, dims).unwrap();

        unsafe { os_mesa.make_current().unwrap() }

        let gl = support::load(&os_mesa);
        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);

        // [x, y, width, height]
        let mut ss: [gl::types::GLint; 4] = [0; 4];
        unsafe {
            gl.gl.GetIntegerv(gl::VIEWPORT, ss.as_mut_ptr() as *mut _);
        }

        let mut pixels: Vec<gl::types::GLubyte> = vec![];
        pixels.resize(3 * ss[2] as usize * ss[3] as usize, 0);
        unsafe {
            gl.gl.ReadPixels(
                0,
                0,
                ss[2],
                ss[3],
                gl::RGB,
                gl::UNSIGNED_BYTE,
                pixels.as_mut_ptr() as *mut _,
            );
        }

        let mut pixels_flipped: Vec<gl::types::GLubyte> = vec![];
        for v in (0..ss[3]).rev()  {
            let s = 3 * v as usize * ss[2] as usize;
            let o = 3 * ss[2] as usize;
            pixels_flipped.extend_from_slice(&pixels[s..(s + o)]);
        }

        image::save_buffer(
            &Path::new("os_mesa.png"),
            &pixels_flipped,
            ss[2] as u32,
            ss[3] as u32,
            image::RGB(8),
        )
        .unwrap();
    }
}
