extern crate glutin;

mod support;

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_title("glutin - Cursor grabbing test");
    let (window, context) = glutin::ContextBuilder::new()
        .build(window_builder, &events_loop)
        .unwrap();

    let _ = unsafe { context.make_current() };

    let gl = support::load(&context);
    let mut grabbed = false;

    events_loop.run_forever(|event| {
        use glutin::{CursorState, ControlFlow, Event, WindowEvent, ElementState};
        match event {
            Event::WindowEvent { event, .. } => match event {

                WindowEvent::KeyboardInput { input, .. } if ElementState::Pressed == input.state => {
                    if grabbed {
                        grabbed = false;
                        window.set_cursor_state(CursorState::Normal)
                              .ok().expect("could not ungrab mouse cursor");
                    } else {
                        grabbed = true;
                        window.set_cursor_state(CursorState::Grab)
                              .ok().expect("could not grab mouse cursor");
                    }
                },

                WindowEvent::Closed => return ControlFlow::Break,
                WindowEvent::Resized(w, h) => context.resize(w, h),
                a @ WindowEvent::MouseMoved { .. } => {
                    println!("{:?}", a);
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
