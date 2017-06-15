extern crate glutin;

use glutin::winit;

mod support;

fn main() {
    let events_loop = winit::EventsLoop::new();
    let window = winit::WindowBuilder::new()
        .with_title("glutin - Cursor grabbing test")
        .build(&events_loop)
        .unwrap();
    let context = glutin::ContextBuilder::new()
        .build(&window)
        .unwrap();

    let _ = unsafe { context.make_current() };

    let gl = support::load(&context);
    let mut grabbed = false;

    events_loop.run_forever(|event| {
        match event {
            winit::Event::WindowEvent { event, .. } => match event {

                winit::WindowEvent::KeyboardInput(winit::ElementState::Pressed, _, _, _) => {
                    if grabbed {
                        grabbed = false;
                        window.set_cursor_state(winit::CursorState::Normal)
                              .ok().expect("could not ungrab mouse cursor");
                    } else {
                        grabbed = true;
                        window.set_cursor_state(winit::CursorState::Grab)
                              .ok().expect("could not grab mouse cursor");
                    }
                },

                winit::WindowEvent::Closed => events_loop.interrupt(),
                winit::WindowEvent::Resized(w, h) => context.resize(w, h),
                a @ winit::WindowEvent::MouseMoved(_, _) => {
                    println!("{:?}", a);
                },
                _ => (),
            },
        }

        gl.draw_frame([0.0, 1.0, 0.0, 1.0]);
        let _ = context.swap_buffers();
    });
}
