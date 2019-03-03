mod support;

use glutin::ContextTrait;
use std::sync::Arc;

fn main() {
    let (separated_context, mut el, win) = {
        let el = glutin::EventsLoop::new();
        let win = glutin::WindowBuilder::new()
            .with_title("A fantastic window!")
            .build(&el)
            .unwrap();
        let win = Arc::new(win);

        let separated_context = glutin::ContextBuilder::new()
            .build_separated(Arc::clone(&win), &el)
            .unwrap();
        (separated_context, el, win)
    };

    unsafe { separated_context.make_current().unwrap() }

    println!(
        "Pixel format of the window's GL context: {:?}",
        separated_context.get_pixel_format()
    );

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
                                virtual_keycode:
                                    Some(glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::Resized(logical_size) => {
                        let dpi_factor = win.get_hidpi_factor();
                        separated_context
                            .resize(logical_size.to_physical(dpi_factor));
                    }
                    _ => (),
                },
                _ => (),
            }
        });

        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
        separated_context.swap_buffers().unwrap();
    }
}
