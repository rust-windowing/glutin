mod support;

fn main() {
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("A transparent window!")
        .with_decorations(false)
        .with_transparency(true);

    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(wb, &el)
        .unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    println!(
        "Pixel format of the window's GL context: {:?}",
        windowed_context.get_pixel_format()
    );

    let gl = support::load(&windowed_context.context());

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        match event {
            glutin::event::Event::LoopDestroyed => return,
            glutin::event::Event::WindowEvent { ref event, .. } => match event {
                glutin::event::WindowEvent::Resized(logical_size) => {
                    let dpi_factor =
                        windowed_context.window().get_hidpi_factor();
                    windowed_context
                        .resize(logical_size.to_physical(dpi_factor));
                }
                glutin::event::WindowEvent::RedrawRequested => {
                    gl.draw_frame([0.0; 4]);
                    windowed_context.swap_buffers().unwrap();
                }
                _ => (),
            },
            _ => (),
        }

        match event {
            glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            _ => *control_flow = winit::event_loop::ControlFlow::Wait,
        }
    });
}
