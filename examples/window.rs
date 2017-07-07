extern crate glutin;

mod support;

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&events_loop)
        .unwrap();

    let _ = unsafe { window.make_current() };

    println!("Pixel format of the window: {:?}", window.get_pixel_format());

    let context = support::load(&window);

    events_loop.run_forever(|event| {
        println!("{:?}", event);

        context.draw_frame((0.0, 1.0, 0.0, 1.0));
        let _ = window.swap_buffers();

        match event {
            glutin::Event::WindowEvent { event: glutin::WindowEvent::Closed, .. } => {
                glutin::ControlFlow::Break
            }
            _ => glutin::ControlFlow::Continue,
        }
    });
}
