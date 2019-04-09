mod support;

use support::{ContextCurrentWrapper, ContextTracker, ContextWrapper};

fn main() {
    let mut el = glutin::EventsLoop::new();
    let mut ct = ContextTracker::default();

    let mut windows = std::collections::HashMap::new();
    for index in 0..3 {
        let title = format!("Charming Window #{}", index + 1);
        let wb = glutin::WindowBuilder::new().with_title(title);
        let windowed_context = glutin::ContextBuilder::new()
            .build_windowed(wb, &el)
            .unwrap();
        let windowed_context =
            unsafe { windowed_context.make_current().unwrap() };
        let gl = support::load(&windowed_context.context());
        let window_id = windowed_context.window().id();
        let context_id = ct.insert(ContextCurrentWrapper::PossiblyCurrent(
            ContextWrapper::Windowed(windowed_context),
        ));
        windows.insert(window_id, (context_id, gl));
    }

    while !windows.is_empty() {
        el.poll_events(|event| {
            println!("{:?}", event);
            match event {
                glutin::Event::WindowEvent { event, window_id } => {
                    match event {
                        glutin::WindowEvent::Resized(logical_size) => {
                            let windowed_context =
                                ct.get_current(windows[&window_id].0).unwrap();
                            let windowed_context = windowed_context.windowed();
                            let dpi_factor =
                                windowed_context.window().get_hidpi_factor();
                            windowed_context
                                .resize(logical_size.to_physical(dpi_factor));
                        }
                        glutin::WindowEvent::CloseRequested => {
                            if let Some((cid, _)) = windows.remove(&window_id) {
                                ct.remove(cid);
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
            let windowed_context = ct.get_current(window.0).unwrap();
            window.1.draw_frame(color);
            windowed_context.windowed().swap_buffers().unwrap();
        }
    }
}
