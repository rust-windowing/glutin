extern crate glutin;

mod support;

use glutin::GlContext;

fn main() {
    let mut el = glutin::EventsLoop::new();
    let win = glutin::WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&el)
        .unwrap();

    let separated_context = glutin::ContextBuilder::new()
        .build_separated(&win, &el)
        .unwrap();

    let _ = unsafe { separated_context.make_current() };

    println!("Pixel format of the window's GL context: {:?}", separated_context.get_pixel_format());

    let gl = support::load(&separated_context.context());

    let mut running = true;
    while running {
        el.poll_events(|event| {
            println!("el {:?}", event);
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
                        separated_context.resize(logical_size.to_physical(dpi_factor));
                    },
                    _ => (),
                },
                _ => ()
            }
        });

        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
        let _ = separated_context.swap_buffers();
    }
}
