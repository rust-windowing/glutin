mod support;

use glutin::ContextTrait;
use std::io::{self, Write};

fn main() {
    let mut el = glutin::EventsLoop::new();

    // enumerating monitors
    let monitor = {
        for (num, monitor) in el.get_available_monitors().enumerate() {
            println!("Monitor #{}: {:?}", num, monitor.get_name());
        }

        print!("Please write the number of the monitor to use: ");
        io::stdout().flush().unwrap();

        let mut num = String::new();
        io::stdin().read_line(&mut num).unwrap();
        let num = num.trim().parse().ok().expect("Please enter a number");
        let monitor = el
            .get_available_monitors()
            .nth(num)
            .expect("Please enter a valid ID");

        println!("Using {:?}", monitor.get_name());

        monitor
    };

    let wb = glutin::WindowBuilder::new()
        .with_title("Hello world!")
        .with_fullscreen(Some(monitor));
    let combined_context = glutin::ContextBuilder::new()
        .build_combined(wb, &el)
        .unwrap();

    unsafe { combined_context.make_current().unwrap() }

    let gl = support::load(&combined_context.context());

    let mut fullscreen = true;
    let mut running = true;
    while running {
        el.poll_events(|event| {
            println!("{:?}", event);
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::Resized(logical_size) => {
                        let dpi_factor = combined_context.get_hidpi_factor();
                        combined_context
                            .resize(logical_size.to_physical(dpi_factor));
                    }
                    glutin::WindowEvent::KeyboardInput { input, .. } => {
                        match input.virtual_keycode {
                            Some(glutin::VirtualKeyCode::Escape) => {
                                running = false
                            }
                            Some(glutin::VirtualKeyCode::F)
                                if input.state
                                    == glutin::ElementState::Pressed =>
                            {
                                let monitor = if fullscreen {
                                    None
                                } else {
                                    Some(combined_context.get_current_monitor())
                                };
                                combined_context.set_fullscreen(monitor);
                                fullscreen = !fullscreen;
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        });

        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
        combined_context.swap_buffers().unwrap();
    }
}
