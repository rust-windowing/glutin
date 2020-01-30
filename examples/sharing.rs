mod support;

use support::{gl, HeadlessBackend};

use glutin::context::ContextBuilder;
use glutin::surface::Surface;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_types::dpi::PhysicalSize;

fn main() {
    simple_logger::init().unwrap();
    let size = PhysicalSize::new(512, 512);
    let el = EventLoop::new();

    let (backend, mut size, conf) = unsafe { HeadlessBackend::new(&el, &size, true).unwrap() };
    let conf = conf.unwrap();
    let hgl = backend.load_symbols().unwrap();

    let render_buf = hgl.make_renderbuf(size);
    let hfb = hgl.make_framebuffer(render_buf);

    unsafe {
        hgl.gl.Viewport(0, 0, size.width as _, size.height as _);
    }

    let wb = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(size);
    let ctx = unsafe {
        ContextBuilder::new()
            .with_sharing(Some(backend.context()))
            .build(&conf)
            .unwrap()
    };
    let (win, surf) = unsafe { Surface::new_window(&conf, &*el, wb).unwrap() };

    unsafe { ctx.make_current(&surf).unwrap() }
    let wgl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());
    let wfb = wgl.make_framebuffer(render_buf);
    unsafe {
        wgl.gl.Viewport(0, 0, size.width as _, size.height as _);
    }

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => unsafe {
                backend.make_current().unwrap();
                hgl.gl.DeleteFramebuffers(1, &hfb);
                hgl.gl.DeleteRenderbuffers(1, &render_buf);

                ctx.make_current(&surf).unwrap();
                wgl.gl.DeleteFramebuffers(1, &wfb);
                return;
            },
            Event::MainEventsCleared => {
                win.request_redraw();
            }
            Event::RedrawRequested(_) => unsafe {
                backend.make_current().unwrap();
                hgl.gl.BindFramebuffer(gl::FRAMEBUFFER, hfb);
                hgl.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
                hgl.draw_frame([1.0, 0.5, 0.7, 1.0]);

                ctx.make_current(&surf).unwrap();
                wgl.gl.BindFramebuffer(gl::FRAMEBUFFER, wfb);
                wgl.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
                wgl.gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);

                wgl.gl.BlitFramebuffer(
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
                surf.swap_buffers().unwrap();
            },
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(nsize) => unsafe {
                    size = *nsize;
                    ctx.make_current(&surf).unwrap();
                    surf.update_after_resize(&size);
                    wgl.gl.Viewport(0, 0, size.width as _, size.height as _);
                    wgl.gl.BindRenderbuffer(gl::RENDERBUFFER, render_buf);
                    wgl.gl.RenderbufferStorage(
                        gl::RENDERBUFFER,
                        gl::RGB8,
                        size.width as _,
                        size.height as _,
                    );

                    backend.make_current().unwrap();
                    hgl.gl.Viewport(0, 0, size.width as _, size.height as _);
                },
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            _ => (),
        }
    });
}
