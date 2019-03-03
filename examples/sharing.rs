//! Requires OpenGL 4.2 minimium.

mod support;

use glutin::ContextTrait;
use support::gl;

fn main() {
    let mut el = glutin::EventsLoop::new();
    let mut size = glutin::dpi::PhysicalSize::new(768., 480.);

    let headless_context =
        glutin::ContextBuilder::new().build_headless(&el).unwrap();

    let wb = glutin::WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_dimensions(glutin::dpi::LogicalSize::from_physical(size, 1.0));
    let combined_context = glutin::ContextBuilder::new()
        .with_shared_lists(&headless_context)
        .build_combined(wb, &el)
        .unwrap();

    unsafe { combined_context.make_current().unwrap() }
    println!(
        "Pixel format of the window's GL context: {:?}",
        combined_context.get_pixel_format()
    );
    let glw = support::load(&combined_context.context());

    let mut render_tex = 0;
    unsafe {
        glw.gl.GenTextures(1, &mut render_tex);
        glw.gl.BindTexture(gl::TEXTURE_2D, render_tex);
        glw.gl.TexStorage2D(
            gl::TEXTURE_2D,
            1,
            gl::SRGB8_ALPHA8,
            size.width as _,
            size.height as _,
        );
    }

    let mut window_fb = 0;
    unsafe {
        glw.gl.GenFramebuffers(1, &mut window_fb);
        glw.gl.BindFramebuffer(gl::READ_FRAMEBUFFER, window_fb);
        glw.gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
        glw.gl.FramebufferTexture2D(
            gl::READ_FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            render_tex,
            0,
        );
    }

    unsafe { headless_context.make_current().unwrap() }
    let glc = support::load(&headless_context);

    let mut context_fb = 0;
    unsafe {
        glc.gl.GenFramebuffers(1, &mut context_fb);
        glc.gl.BindFramebuffer(gl::FRAMEBUFFER, context_fb);
        glc.gl.FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            render_tex,
            0,
        );
        glc.gl.Viewport(0, 0, size.width as _, size.height as _);
    }

    let mut running = true;
    while running {
        el.poll_events(|event| {
            println!("{:?}", event);
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::Resized(logical_size) => {
                        unsafe { combined_context.make_current().unwrap() }
                        let dpi_factor = combined_context.get_hidpi_factor();
                        size = logical_size.to_physical(dpi_factor);
                        combined_context.resize(size);

                        unsafe {
                            combined_context.swap_buffers().unwrap();
                            glw.gl.DeleteTextures(1, &render_tex);
                            glw.gl.DeleteFramebuffers(1, &window_fb);

                            glw.gl.GenTextures(1, &mut render_tex);
                            glw.gl.BindTexture(gl::TEXTURE_2D, render_tex);
                            glw.gl.TexStorage2D(
                                gl::TEXTURE_2D,
                                1,
                                gl::SRGB8_ALPHA8,
                                size.width as _,
                                size.height as _,
                            );

                            glw.gl.GenFramebuffers(1, &mut window_fb);
                            glw.gl.BindFramebuffer(
                                gl::READ_FRAMEBUFFER,
                                window_fb,
                            );
                            glw.gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
                            glw.gl.FramebufferTexture2D(
                                gl::READ_FRAMEBUFFER,
                                gl::COLOR_ATTACHMENT0,
                                gl::TEXTURE_2D,
                                render_tex,
                                0,
                            );

                            let _ = headless_context.make_current();
                            glc.gl.DeleteFramebuffers(1, &context_fb);

                            glc.gl.GenFramebuffers(1, &mut context_fb);
                            glc.gl.BindFramebuffer(gl::FRAMEBUFFER, context_fb);
                            glc.gl.FramebufferTexture2D(
                                gl::FRAMEBUFFER,
                                gl::COLOR_ATTACHMENT0,
                                gl::TEXTURE_2D,
                                render_tex,
                                0,
                            );

                            glc.gl.Viewport(
                                0,
                                0,
                                size.width as _,
                                size.height as _,
                            );
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        });

        unsafe { headless_context.make_current().unwrap() }
        glc.draw_frame([1.0, 0.5, 0.7, 1.0]);

        unsafe { combined_context.make_current().unwrap() }
        unsafe {
            glw.gl.BlitFramebuffer(
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
        }
        let _ = combined_context.swap_buffers();
    }

    unsafe {
        let _ = combined_context.make_current();
        glw.gl.DeleteTextures(1, &render_tex);
        glw.gl.DeleteFramebuffers(1, &window_fb);
        let _ = headless_context.make_current();
        glc.gl.DeleteFramebuffers(1, &context_fb);
    }
}
