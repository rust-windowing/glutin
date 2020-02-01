mod support;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use glutin::platform::unix::ConfigPlatformAttributesExt;

use glutin::config::ConfigsFinder;
use glutin::context::ContextBuilder;
use glutin::surface::Surface;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    simple_logger::init().unwrap();
    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("A transparent window!")
        .with_decorations(false)
        .with_transparent(true);

    let conf_finder = ConfigsFinder::new();
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    let conf_finder = conf_finder.with_x11_transparency(Some(true));

    let confs = unsafe { conf_finder.find(&*el).unwrap() };
    let conf = &confs[0];
    println!("Configeration chosen: {:?}", conf);

    let ctx = unsafe { ContextBuilder::new().build(conf).unwrap() };
    let (win, surf) = unsafe { Surface::new_window(conf, &*el, wb).unwrap() };

    unsafe { ctx.make_current(&surf).unwrap() }

    let gl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::MainEventsCleared => {
                win.request_redraw();
            }
            Event::RedrawRequested(_) => {
                gl.draw_frame([0.0; 4]);
                surf.swap_buffers().unwrap();
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(size) => {
                    ctx.update_after_resize();
                    surf.update_after_resize(*size);
                    unsafe {
                        gl.gl.Viewport(0, 0, size.width as _, size.height as _);
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            _ => (),
        }
    });
}
