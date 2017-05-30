extern crate glutin;

mod support;

fn main() {
    let events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("glutin - Cursor grabbing test")
        .build(&events_loop)
        .unwrap();

    let _ = unsafe { window.make_current() };

    let context = support::load(&window);
    let mut grabbed = false;

    events_loop.run_forever(|event| {
        use glutin::{CursorState, WindowEvent, ElementState};
        match event {
            glutin::Event::WindowEvent { event, .. } => match event {

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

                WindowEvent::Closed => events_loop.interrupt(),

                a @ WindowEvent::MouseMoved { .. } => {
                    println!("{:?}", a);
                },

                _ => (),
            },
            _ => (),
        }

        context.draw_frame((0.0, 1.0, 0.0, 1.0));
        let _ = window.swap_buffers();
    });
}
