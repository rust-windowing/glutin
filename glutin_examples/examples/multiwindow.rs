mod support;

use glutin::ContextTrait;

fn main() {
    let mut el = glutin::EventsLoop::new();

    let mut windows = std::collections::HashMap::new();
    for index in 0..3 {
        let title = format!("Charming Window #{}", index + 1);
        let wb = glutin::WindowBuilder::new().with_title(title);
        let combined_context = glutin::ContextBuilder::new()
            .build_combined(wb, &el)
            .unwrap();
        unsafe { combined_context.make_current().unwrap() }
        let gl = support::load(&combined_context.context());
        let window_id = combined_context.id();
        windows.insert(window_id, (combined_context, gl));
    }

    while !windows.is_empty() {
        el.poll_events(|event| {
            println!("{:?}", event);
            match event {
                glutin::Event::WindowEvent { event, window_id } => {
                    match event {
                        glutin::WindowEvent::Resized(logical_size) => {
                            let combined_context = &windows[&window_id].0;
                            let dpi_factor =
                                combined_context.get_hidpi_factor();
                            combined_context
                                .resize(logical_size.to_physical(dpi_factor));
                        }
                        glutin::WindowEvent::CloseRequested => {
                            if windows.remove(&window_id).is_some() {
                                println!(
                                    "Window with ID {:?} has been closed",
                                    window_id
                                );
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        });

        for (index, window) in windows.values().enumerate() {
            let mut color = [1.0, 0.5, 0.7, 1.0];
            color.swap(0, index % 3);
            unsafe { window.0.make_current().unwrap() };
            window.1.draw_frame(color);
            window.0.swap_buffers().unwrap();
        }
    }
}
