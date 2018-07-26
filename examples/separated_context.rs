extern crate glutin;

mod support;

use glutin::GlContext;

fn main() {
    let mut evlp = glutin::EventsLoop::new();
    let win = glutin::WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&evlp)
        .unwrap();

    let ctx = glutin::ContextBuilder::new();
    let ctx = glutin::GlSeparatedContext::new(&win, ctx, &evlp).unwrap();

    let _ = unsafe { ctx.make_current() };

    println!("Pixel format of the window's GL context: {:?}", ctx.get_pixel_format());

    let gl = support::load(&ctx.context());

    let mut running = true;
    while running {
        evlp.poll_events(|event| {
            println!("Evlp {:?}", event);
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::Resized(logical_size) => {
                        let dpi_factor = win.get_hidpi_factor();
                        ctx.resize(logical_size.to_physical(dpi_factor));
                    },
                    _ => (),
                },
                _ => ()
            }
        });

        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
        let _ = ctx.swap_buffers();
    }
}
