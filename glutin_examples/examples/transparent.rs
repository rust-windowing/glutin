mod support;

use glutin::ContextTrait;

fn main() {
    let mut el = glutin::EventsLoop::new();
    let wb = glutin::WindowBuilder::new()
        .with_title("A transparent window!")
        .with_decorations(false)
        .with_transparency(true);
    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(wb, &el)
        .unwrap();

    unsafe { windowed_context.make_current().unwrap() }

    println!(
        "Pixel format of the window's GL context: {:?}",
        windowed_context.get_pixel_format()
    );

    let gl = support::load(&windowed_context.context());

    let mut running = true;
    while running {
        el.poll_events(|event| {
            println!("{:?}", event);
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::Resized(logical_size) => {
                        let dpi_factor = windowed_context.get_hidpi_factor();
                        windowed_context
                            .resize(logical_size.to_physical(dpi_factor));
                    }
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode:
                                    Some(glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => running = false,
                    _ => (),
                },
                _ => (),
            }
        });

        gl.draw_frame([0.0; 4]);
        windowed_context.swap_buffers().unwrap();
    }
}
