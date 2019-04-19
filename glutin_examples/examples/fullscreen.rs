mod support;

use std::io::{self, Write};

fn main() {
    let el = glutin::event_loop::EventLoop::new();

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

    let wb = glutin::window::WindowBuilder::new()
        .with_title("Hello world!")
        .with_fullscreen(Some(monitor));
    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(wb, &el)
        .unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let gl = support::load(&windowed_context.context());

    let mut fullscreen = true;
    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        match event {
            glutin::event::Event::LoopDestroyed => return,
            glutin::event::Event::WindowEvent { ref event, .. } => {
                match event {
                    glutin::event::WindowEvent::Resized(logical_size) => {
                        let dpi_factor =
                            windowed_context.window().get_hidpi_factor();
                        windowed_context
                            .resize(logical_size.to_physical(dpi_factor));
                    }
                    glutin::event::WindowEvent::KeyboardInput {
                        input, ..
                    } => match input.virtual_keycode {
                        Some(glutin::event::VirtualKeyCode::F)
                            if input.state
                                == glutin::event::ElementState::Pressed =>
                        {
                            let monitor = if fullscreen {
                                None
                            } else {
                                Some(
                                    windowed_context
                                        .window()
                                        .get_current_monitor(),
                                )
                            };
                            windowed_context.window().set_fullscreen(monitor);
                            fullscreen = !fullscreen;
                        }
                        _ => (),
                    },
                    glutin::event::WindowEvent::RedrawRequested => {
                        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
                        windowed_context.swap_buffers().unwrap();
                    }
                    _ => (),
                }
            }
            _ => (),
        }

        match event {
            glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            _ => *control_flow = winit::event_loop::ControlFlow::Wait,
        }
    });
}
