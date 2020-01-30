mod support;

use support::gl;

use glutin::config::{Api, ConfigsFinder, Version};
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

    let mut confs = unsafe {
        ConfigsFinder::new()
            .with_must_support_pbuffers(true)
            .with_must_support_windows(true)
            .with_gl((Api::OpenGl, Version(3, 3)))
            .find(&*el)
            .unwrap()
    };
    let conf = confs.drain(..1).next().unwrap();
    println!("Configeration chosen: {:?}", conf);

    let wb = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(size);
    let ctx = unsafe { ContextBuilder::new().build(&conf).unwrap() };
    let (win, wsurf) = unsafe { Surface::new_window(&conf, &*el, wb).unwrap() };
    let mut psurf = unsafe { Surface::new_pbuffer(&conf, &size, true).unwrap() };
    let mut size = psurf.size().unwrap();

    unsafe { ctx.make_current(&wsurf).unwrap() }
    let gl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());
    unsafe {
        gl.gl.Viewport(0, 0, size.width as _, size.height as _);
    }

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
                    psurf = Surface::new_pbuffer(&conf, &size, true).unwrap();
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
