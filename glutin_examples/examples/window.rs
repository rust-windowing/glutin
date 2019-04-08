mod support;

fn main() {
    let mut el = glutin::EventsLoop::new();
    let wb = glutin::WindowBuilder::new().with_title("A fantastic window!");

    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(wb, &el)
        .unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

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
                        let dpi_factor =
                            windowed_context.window().get_hidpi_factor();
                        windowed_context
                            .resize(logical_size.to_physical(dpi_factor));
                    }
                    _ => (),
                },
                _ => (),
            }
        });

        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
        windowed_context.swap_buffers().unwrap();
    }
}
