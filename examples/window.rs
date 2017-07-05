extern crate glutin;

mod support;

fn main() {
    let mut events_loop = glutin::winit::EventsLoop::new();
    let window_builder = glutin::winit::WindowBuilder::new()
        .with_title("A fantastic window!");
    let (_window, context) = glutin::ContextBuilder::new()
        .build(window_builder, &events_loop)
        .unwrap();

    let _ = unsafe { context.make_current() };

    println!("Pixel format of the window's GL context: {:?}", context.get_pixel_format());

    let gl = support::load(&context);

    events_loop.run_forever(|event| {
        println!("{:?}", event);
        match event {
            glutin::winit::Event::WindowEvent { event, .. } => match event {
                glutin::winit::WindowEvent::Closed => return glutin::winit::ControlFlow::Break,
                glutin::winit::WindowEvent::Resized(w, h) => context.resize(w, h),
                _ => (),
            },
            _ => ()
        }

        gl.draw_frame([0.0, 1.0, 0.0, 1.0]);
        let _ = context.swap_buffers();
        glutin::winit::ControlFlow::Continue
    });
}
