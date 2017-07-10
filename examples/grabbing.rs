extern crate glutin;

mod support;

use glutin::GlContext;

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("glutin - Cursor grabbing test");
    let context = glutin::ContextBuilder::new();
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    let _ = unsafe { gl_window.make_current() };

    let gl = support::load(&gl_window);
    let mut grabbed = false;

    events_loop.run_forever(|event| {
        use glutin::{CursorState, ControlFlow, Event, WindowEvent, ElementState};
        match event {
            Event::WindowEvent { event, .. } => match event {

                WindowEvent::KeyboardInput { input, .. } if ElementState::Pressed == input.state => {
                    if grabbed {
                        grabbed = false;
                        gl_window.set_cursor_state(CursorState::Normal)
                                 .ok().expect("could not ungrab mouse cursor");
                    } else {
                        grabbed = true;
                        gl_window.set_cursor_state(CursorState::Grab)
                                 .ok().expect("could not grab mouse cursor");
                    }
                },

                WindowEvent::Closed => return ControlFlow::Break,
                WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                a @ WindowEvent::MouseMoved { .. } => {
                    println!("{:?}", a);
                },
                _ => (),
            },
            _ => (),
        }

        gl.draw_frame([0.0, 1.0, 0.0, 1.0]);
        let _ = gl_window.swap_buffers();
        ControlFlow::Continue
    });
}
