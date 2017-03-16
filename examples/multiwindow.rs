extern crate glutin;

mod support;

fn main() {
    let events_loop = glutin::EventsLoop::new();

    let window1 = glutin::WindowBuilder::new().build(&events_loop).unwrap();
    let window2 = glutin::WindowBuilder::new().build(&events_loop).unwrap();
    let window3 = glutin::WindowBuilder::new().build(&events_loop).unwrap();

    let mut num_windows = 3;

    let _ = unsafe { window1.make_current() };
    let context1 = support::load(&window1);
    let _ = unsafe { window2.make_current() };
    let context2 = support::load(&window2);
    let _ = unsafe { window3.make_current() };
    let context3 = support::load(&window3);

    fn draw_to_window(window: &glutin::Window, context: &support::Context, color: (f32, f32, f32, f32)) {
        let _ = unsafe { window.make_current() };
        context.draw_frame(color);
        let _ = window.swap_buffers();
    }

    events_loop.run_forever(|event| {
        println!("{:?}", event);

        match event {
            glutin::Event::WindowEvent { event: glutin::WindowEvent::Closed, window_id } => {
                if window_id == window1.id() {
                    println!("Window 1 has been closed")
                } else if window_id == window2.id() {
                    println!("Window 2 has been closed")
                } else if window_id == window3.id() {
                    println!("Window 3 has been closed");
                } else {
                    unreachable!()
                }

                num_windows -= 1;
                if num_windows == 0 {
                    events_loop.interrupt();
                }
            },
            _ => (),
        }

        draw_to_window(&window1, &context1, (0.0, 1.0, 0.0, 1.0));
        draw_to_window(&window2, &context2, (0.0, 0.0, 1.0, 1.0));
        draw_to_window(&window3, &context3, (1.0, 0.0, 0.0, 1.0));
    });
}
