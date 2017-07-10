extern crate glutin;

mod support;

use glutin::GlContext;

fn main() {
    let mut events_loop = glutin::EventsLoop::new();

    let mut windows = std::collections::HashMap::new();
    for _ in 0..3 {
        let window = glutin::WindowBuilder::new();
        let context = glutin::ContextBuilder::new();
        let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();
        let _ = unsafe { gl_window.make_current() };
        let gl = support::load(&gl_window);
        let window_id = gl_window.id();
        windows.insert(window_id, (gl_window, gl));
    }

    events_loop.run_forever(|event| {
        println!("{:?}", event);
        match event {
            glutin::Event::WindowEvent { event, window_id } => match event {
                glutin::WindowEvent::Resized(w, h) => {
                    windows[&window_id].0.resize(w, h)
                },
                glutin::WindowEvent::Closed => {
                    if windows.remove(&window_id).is_some() {
                        println!("Window with ID {:?} has been closed", window_id);
                        if windows.is_empty() {
                            return glutin::ControlFlow::Break;
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
            let _ = unsafe { window.0.make_current() };
            window.1.draw_frame(color);
            let _ = window.0.swap_buffers();
        }

        glutin::ControlFlow::Continue
    });
}
