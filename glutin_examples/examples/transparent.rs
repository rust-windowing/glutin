mod support;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::{ContextBuilder, ContextSupports, WindowSurface};

fn main() {
    env_logger::init();
    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("A transparent window!")
        .with_decorations(false)
        .with_transparent(true);

    let ctx = ContextBuilder::new()
        .build(&el, ContextSupports::WINDOW_SURFACES)
        .unwrap();
    let (win, surface) = WindowSurface::new(&el, &ctx, wb).unwrap();

    unsafe { ctx.make_current_surface(&surface).unwrap() }

    println!(
        "Pixel format of the window's GL context: {:?}",
        ctx.get_pixel_format()
    );

    let gl = support::load(|s| ctx.get_proc_address(s));

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(logical_size) => {
                    let dpi_factor = win.hidpi_factor();
                    ctx.update_after_resize();
                    surface.update_after_resize(
                        logical_size.to_physical(dpi_factor),
                    );
                }
                WindowEvent::RedrawRequested => {
                    gl.draw_frame([0.0; 4]);
                    surface.swap_buffers().unwrap();
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                _ => (),
            },
            _ => (),
        }
    });
}
