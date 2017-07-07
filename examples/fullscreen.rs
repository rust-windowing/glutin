extern crate glutin;

use glutin::winit;

use std::io::{self, Write};

mod support;

fn main() {
    // enumerating monitors
    let monitor = {
        for (num, monitor) in winit::get_available_monitors().enumerate() {
            println!("Monitor #{}: {:?}", num, monitor.get_name());
        }

        print!("Please write the number of the monitor to use: ");
        io::stdout().flush().unwrap();

        let mut num = String::new();
        io::stdin().read_line(&mut num).unwrap();
        let num = num.trim().parse().ok().expect("Please enter a number");
        let monitor = winit::get_available_monitors().nth(num).expect("Please enter a valid ID");

        println!("Using {:?}", monitor.get_name());

        monitor
    };

    let mut events_loop = winit::EventsLoop::new();
    let window = winit::WindowBuilder::new()
        .with_title("Hello world!")
        .with_fullscreen(monitor)
        .build(&events_loop)
        .unwrap();
    let context = glutin::ContextBuilder::new()
        .build(&window)
        .unwrap();

    let _ = unsafe { context.make_current() };
    
    let gl = support::load(&context);

    events_loop.run_forever(|event| {
        use glutin::winit::{ControlFlow, Event, WindowEvent, VirtualKeyCode};
        println!("{:?}", event);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Closed => return ControlFlow::Break,
                WindowEvent::Resized(w, h) => context.resize(w, h),
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                        return ControlFlow::Break;
                    }
                },
                _ => (),
            },
            _ => (),
        }

        gl.draw_frame([0.0, 1.0, 0.0, 1.0]);
        let _ = context.swap_buffers();
        ControlFlow::Continue
    });
}
