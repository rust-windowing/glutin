extern crate glutin;

use std::io::{self, Write};

mod support;

fn main() {
    // enumerating monitors
    let monitor = {
        for (num, monitor) in glutin::get_available_monitors().enumerate() {
            println!("Monitor #{}: {:?}", num, monitor.get_name());
        }

        print!("Please write the number of the monitor to use: ");
        io::stdout().flush().unwrap();

        let mut num = String::new();
        io::stdin().read_line(&mut num).unwrap();
        let num = num.trim().parse().ok().expect("Please enter a number");
        let monitor = glutin::get_available_monitors().nth(num).expect("Please enter a valid ID");

        println!("Using {:?}", monitor.get_name());

        monitor
    };

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("Hello world!")
        .with_fullscreen(monitor)
        .build(&events_loop)
        .unwrap();

    let _ = unsafe { window.make_current() };
    
    let context = support::load(&window);

    events_loop.run_forever(|event| {
        println!("{:?}", event);

        context.draw_frame((0.0, 1.0, 0.0, 1.0));
        let _ = window.swap_buffers();

        match event {
            glutin::Event::WindowEvent { event: glutin::WindowEvent::Closed, .. } |
            glutin::Event::WindowEvent { event: glutin::WindowEvent::KeyboardInput { input: glutin::KeyboardInput { virtual_keycode: Some(glutin::VirtualKeyCode::Escape), .. }, .. }, .. } => {
                glutin::ControlFlow::Break
            },
            _ => glutin::ControlFlow::Continue
        }
    });
}
