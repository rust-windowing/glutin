mod support;

use support::{ContextCurrentWrapper, ContextTracker, ContextWrapper};

fn main() {
    let el = glutin::event_loop::EventLoop::new();
    let mut ct = ContextTracker::default();

    let mut windows = std::collections::HashMap::new();
    for index in 0..3 {
        let title = format!("Charming Window #{}", index + 1);
        let wb = glutin::window::WindowBuilder::new().with_title(title);
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

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        match event {
            glutin::event::Event::LoopDestroyed => return,
            glutin::event::Event::WindowEvent { event, window_id } => {
                match event {
                    glutin::event::WindowEvent::Resized(logical_size) => {
                        let windowed_context =
                            ct.get_current(windows[&window_id].0).unwrap();
                        let windowed_context = windowed_context.windowed();
                        let dpi_factor =
                            windowed_context.window().get_hidpi_factor();
                        windowed_context
                            .resize(logical_size.to_physical(dpi_factor));
                    }
                    glutin::event::WindowEvent::CloseRequested => {
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

        for (index, window) in windows.values().enumerate() {
            let mut color = [1.0, 0.5, 0.7, 1.0];
            color.swap(0, index % 3);
            let windowed_context = ct.get_current(window.0).unwrap();
            window.1.draw_frame(color);
            windowed_context.windowed().swap_buffers().unwrap();
        }

        if windows.is_empty() {
            *control_flow = winit::event_loop::ControlFlow::Exit
        } else {
            *control_flow = winit::event_loop::ControlFlow::Wait
        }
    });
}
