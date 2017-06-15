extern crate glutin;

mod support;

fn main() {
    let events_loop = glutin::winit::EventsLoop::new();

    struct Window {
        _window: glutin::winit::Window,
        context: glutin::Context,
        gl: support::Gl,
    }

    let mut windows = std::collections::HashMap::new();
    for _ in 0..3 {
        let window = glutin::winit::WindowBuilder::new().build(&events_loop).unwrap();
        let context = glutin::ContextBuilder::new().build(&window).unwrap();
        let _ = unsafe { context.make_current() };
        let gl = support::load(&context);
        let window_id = window.id();
        let window = Window { _window: window, context: context, gl: gl };
        windows.insert(window_id, window);
    }

    events_loop.run_forever(|event| {
        println!("{:?}", event);
        match event {
            glutin::winit::Event::WindowEvent { event, window_id } => match event {
                glutin::winit::WindowEvent::Resized(w, h) => {
                    windows[&window_id].context.resize(w, h)
                },
                glutin::winit::WindowEvent::Closed => {
                    if windows.remove(&window_id).is_some() {
                        println!("Window with ID {:?} has been closed", window_id);
                        if windows.is_empty() {
                            events_loop.interrupt();
                        }
                    }
                },
                _ => (),
            },
            _ => (),
        }

        for (i, window) in windows.values().enumerate() {
            let mut color = [0.0, 0.0, 0.0, 1.0];
            color[i] = 1.0; // Color each of the three windows a different color.
            let _ = unsafe { window.context.make_current() };
            window.gl.draw_frame(color);
            let _ = window.context.swap_buffers();
        }
    });
}
