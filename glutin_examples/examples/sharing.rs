mod support;

use glutin::dpi::PhysicalSize;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
use support::{gl, ContextCurrentWrapper, ContextTracker, ContextWrapper};

fn make_renderbuf(gl: &support::Gl, size: PhysicalSize<u32>) -> gl::types::GLuint {
    let mut render_buf = 0;
    unsafe {
        gl.gl.GenRenderbuffers(1, &mut render_buf);
        gl.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
        gl.gl.RenderbufferStorage(gl::RENDERBUFFER, gl::RGB8, size.width as _, size.height as _);
    }

    render_buf
}

fn main() {
    let el = EventLoop::new();
    let size = PhysicalSize::new(768, 480);

    let mut ct = ContextTracker::default();

    let headless_context =
        ContextBuilder::new().build_headless(&el, PhysicalSize::new(1, 1)).unwrap();

    let wb = WindowBuilder::new().with_title("A fantastic window!").with_inner_size(size);
    let windowed_context =
        ContextBuilder::new().with_shared_lists(&headless_context).build_windowed(wb, &el).unwrap();

    let headless_id =
        ct.insert(ContextCurrentWrapper::NotCurrent(ContextWrapper::Headless(headless_context)));
    let windowed_id =
        ct.insert(ContextCurrentWrapper::NotCurrent(ContextWrapper::Windowed(windowed_context)));

    let windowed_context = ct.get_current(windowed_id).unwrap();
    println!(
        "Pixel format of the window's GL context: {:?}",
        windowed_context.windowed().get_pixel_format()
    );
    let glw = support::load(&windowed_context.windowed().context());

    let render_buf = make_renderbuf(&glw, size);

    let mut window_fb = 0;
    unsafe {
        glw.gl.GenFramebuffers(1, &mut window_fb);
        // Both `GL_DRAW_FRAMEBUFFER` and `GL_READ_FRAMEBUFFER` need to be
        // non-zero for `glFramebufferRenderbuffer`. We can change
        // `GL_DRAW_FRAMEBUFFER` after.
        glw.gl.BindFramebuffer(gl::FRAMEBUFFER, window_fb);
        glw.gl.FramebufferRenderbuffer(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::RENDERBUFFER,
            render_buf,
        );
        glw.gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
        glw.gl.Viewport(0, 0, size.width as _, size.height as _);
    }
    std::mem::drop(windowed_context);

    let headless_context = ct.get_current(headless_id).unwrap();
    let glc = support::load(&headless_context.headless());

    let mut context_fb = 0;
    unsafe {
        // Using the fb backing a pbuffer is very much a bad idea. Fails on
        // many platforms, and is deprecated. Better just make your own fb.
        glc.gl.GenFramebuffers(1, &mut context_fb);
        glc.gl.BindFramebuffer(gl::FRAMEBUFFER, context_fb);
        glc.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
        glc.gl.FramebufferRenderbuffer(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::RENDERBUFFER,
            render_buf,
        );
        glc.gl.Viewport(0, 0, size.width as _, size.height as _);
    }
    std::mem::drop(headless_context);

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => {
                unsafe {
                    let windowed_context = ct.get_current(windowed_id).unwrap();
                    glw.gl.DeleteFramebuffers(1, &window_fb);
                    glw.gl.DeleteRenderbuffers(1, &render_buf);
                    std::mem::drop(windowed_context);
                    let _ = ct.get_current(headless_id).unwrap();
                    glc.gl.DeleteFramebuffers(1, &context_fb);
                }
                return;
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    let windowed_context = ct.get_current(windowed_id).unwrap();
                    windowed_context.windowed().resize(physical_size);

                    unsafe {
                        windowed_context.windowed().swap_buffers().unwrap();
                        glw.gl.RenderbufferStorage(
                            gl::RENDERBUFFER,
                            gl::RGB8,
                            size.width as _,
                            size.height as _,
                        );
                        glw.gl.Viewport(0, 0, size.width as _, size.height as _);
                        std::mem::drop(windowed_context);

                        let _ = ct.get_current(headless_id).unwrap();
                        glc.gl.Viewport(0, 0, size.width as _, size.height as _);
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::RedrawRequested(_) => {
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
            _ => (),
        }
    });
}
