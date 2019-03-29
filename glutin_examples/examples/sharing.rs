//! Requires OpenGL 4.2 minimium.

mod support;

use support::{gl, ContextCurrentWrapper, ContextTracker, ContextWrapper};

fn main() {
    let mut el = glutin::EventsLoop::new();
    let mut size = glutin::dpi::PhysicalSize::new(768., 480.);

    let mut ct = ContextTracker::default();

    let headless_context = glutin::ContextBuilder::new()
        .build_headless(&el, size)
        .unwrap();

    let wb = glutin::WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_dimensions(glutin::dpi::LogicalSize::from_physical(size, 1.0));
    let windowed_context = glutin::ContextBuilder::new()
        .with_shared_lists(&headless_context)
        .build_windowed(wb, &el)
        .unwrap();

    let headless_id = ct.insert(ContextCurrentWrapper::NotCurrent(
        ContextWrapper::Headless(headless_context),
    ));
    let windowed_id = ct.insert(ContextCurrentWrapper::NotCurrent(
        ContextWrapper::Windowed(windowed_context),
    ));

    let windowed_context = ct.get_current(windowed_id).unwrap();
    println!(
        "Pixel format of the window's GL context: {:?}",
        windowed_context.windowed().get_pixel_format()
    );
    let glw = support::load(&windowed_context.windowed().context());

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
    std::mem::drop(windowed_context);

    let headless_context = ct.get_current(headless_id).unwrap();
    let glc = support::load(&headless_context.headless());

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
    std::mem::drop(headless_context);

    let mut running = true;
    while running {
        el.poll_events(|event| {
            println!("{:?}", event);
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::Resized(logical_size) => {
                        let windowed_context =
                            ct.get_current(windowed_id).unwrap();
                        let dpi_factor = windowed_context
                            .windowed()
                            .window()
                            .get_hidpi_factor();
                        size = logical_size.to_physical(dpi_factor);
                        windowed_context.windowed().resize(size);

                        unsafe {
                            windowed_context.windowed().swap_buffers().unwrap();
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
                            std::mem::drop(windowed_context);

                            let _ = ct.get_current(headless_id).unwrap();
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

        let headless_context = ct.get_current(headless_id).unwrap();
        glc.draw_frame([1.0, 0.5, 0.7, 1.0]);
        std::mem::drop(headless_context);

        let windowed_context = ct.get_current(windowed_id).unwrap();
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
        windowed_context.windowed().swap_buffers().unwrap();
    }

    unsafe {
        let windowed_context = ct.get_current(windowed_id).unwrap();
        glw.gl.DeleteTextures(1, &render_tex);
        glw.gl.DeleteFramebuffers(1, &window_fb);
        std::mem::drop(windowed_context);
        let _ = ct.get_current(headless_id).unwrap();
        glc.gl.DeleteFramebuffers(1, &context_fb);
    }
}
