mod support;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
use support::{ContextCurrentWrapper, ContextTracker, ContextWrapper};

fn main() {
    let el = EventLoop::new();
    let mut ct = ContextTracker::default();

    let mut windows = std::collections::HashMap::new();
    for index in 0..3 {
        let title = format!("Charming Window #{}", index + 1);
        let wb = WindowBuilder::new().with_title(title);
        let windowed_context = ContextBuilder::new().build_windowed(wb, &el).unwrap();
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        let gl = support::load(&windowed_context.context());
        let window_id = windowed_context.window().id();
        let context_id = ct.insert(ContextCurrentWrapper::PossiblyCurrent(
            ContextWrapper::Windowed(windowed_context),
        ));
        windows.insert(window_id, (context_id, gl, index));
    }

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, window_id } => match event {
                WindowEvent::Resized(physical_size) => {
                    let windowed_context = ct.get_current(windows[&window_id].0).unwrap();
                    let windowed_context = windowed_context.windowed();
                    windowed_context.resize(physical_size);
                }
                WindowEvent::CloseRequested => {
                    if let Some((cid, _, _)) = windows.remove(&window_id) {
                        ct.remove(cid);
                        println!("Window with ID {:?} has been closed", window_id);
                    }
                }
                _ => (),
            },
            Event::RedrawRequested(window_id) => {
                let window = &windows[&window_id];

                let mut color = [1.0, 0.5, 0.7, 1.0];
                color.swap(0, window.2 % 3);

                let windowed_context = ct.get_current(window.0).unwrap();

                window.1.draw_frame(color);
                windowed_context.windowed().swap_buffers().unwrap();
            }
            _ => (),
        }

        if windows.is_empty() {
            *control_flow = ControlFlow::Exit
        } else {
            *control_flow = ControlFlow::Wait
        }
    });
}
