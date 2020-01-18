mod support;

use glutin::config::ConfigsFinder;
use glutin::context::ContextBuilder;
use glutin::surface::Surface;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    env_logger::init();
    let el = EventLoop::new();
    let wb = WindowBuilder::new().with_title("A fantastic window!");

    let conf = ConfigsFinder::new().find(&*el).unwrap();
    let conf = &conf[0];
    println!(
        "Configeration chosen: {:?}",
        conf,
    );

    let ctx = ContextBuilder::new().build(conf).unwrap();
    let (win, surf) = unsafe { Surface::new_window(conf, &*el, wb).unwrap() };

    unsafe { ctx.make_current(&surf).unwrap() }

    let gl = support::load(|s| ctx.get_proc_address(s).unwrap());

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::MainEventsCleared => {
                win.request_redraw();
            }
            Event::RedrawRequested(_) => {
                gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
                surf.swap_buffers().unwrap();
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(size) => {
                    let dpi_factor = win.scale_factor();
                    ctx.update_after_resize();
                    surf.update_after_resize(size.clone());
                    unsafe { gl.gl.Viewport(0, 0, size.width as _, size.height as _); }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            _ => (),
        }
    });
}
