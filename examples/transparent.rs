extern crate glutin;

mod support;

fn main() {
    let events_loop = glutin::winit::EventsLoop::new();
    let window = glutin::winit::WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_decorations(false)
        .with_transparency(true)
        .build(&events_loop)
        .unwrap();
    let context = glutin::ContextBuilder::new()
        .with_transparency(true)
        .build(&window)
        .unwrap();

    let _ = unsafe { context.make_current() };

    println!("Pixel format of the window's GL context: {:?}", context.get_pixel_format());

    let gl = support::load(&context);

    events_loop.run_forever(|event| {
        println!("{:?}", event);
        match event {
            glutin::winit::Event::WindowEvent { event, .. } => match event {
                glutin::winit::WindowEvent::Closed => events_loop.interrupt(),
                glutin::winit::WindowEvent::Resized(w, h) => context.resize(w, h),
                _ => (),
            },
        }

        gl.draw_frame([0.0, 0.0, 0.0, 0.0]);
        let _ = context.swap_buffers();
    });
}
