mod support;

use glutin::config::ConfigsFinder;
use glutin::context::ContextBuilder;
use glutin::surface::Surface;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    simple_logger::init().unwrap();
    let el = EventLoop::new();

    let confs = unsafe { ConfigsFinder::new().find(&*el).unwrap() };
    let conf = &confs[0];
    println!("Configeration chosen: {:?}", conf);

    let ctx = unsafe { ContextBuilder::new().build(conf).unwrap() };

    let mut wins = std::collections::HashMap::new();
    for index in 0..3 {
        let title = format!("Charming Window #{}", index + 1);
        let wb = WindowBuilder::new().with_title(title);

        let (win, surf) = unsafe { Surface::new_window(conf, &*el, wb).unwrap() };

        let win_id = win.id();
        let size = win.inner_size();
        wins.insert(win_id, (index, win, surf, size));
    }

    let mut cur_surf = wins.keys().next().unwrap().clone();
    unsafe { ctx.make_current(&wins[&cur_surf].2).unwrap() }
    let gl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        match event {
            Event::LoopDestroyed => return,
            Event::RedrawRequested(win_id) => {
                let (index, _, surf, size) = &wins[&win_id];

                let mut color = [1.0, 0.5, 0.7, 1.0];
                color.swap(0, (index % 3) as _);

                if cur_surf != win_id {
                    unsafe { ctx.make_current(&surf).unwrap() };
                    cur_surf = win_id;
                }

                unsafe {
                    gl.gl.Viewport(0, 0, size.width as _, size.height as _);
                }

                gl.draw_frame(color);
                surf.swap_buffers().unwrap();
            }
            Event::WindowEvent { event, window_id } => match event {
                WindowEvent::Resized(size) => {
                    let (_, _, surf, wsize) = &mut wins.get_mut(&window_id).unwrap();

                    if cur_surf != window_id {
                        unsafe { ctx.make_current(&surf).unwrap() };
                        cur_surf = window_id;
                    } else {
                        // Only required if make_current was not called.
                        ctx.update_after_resize();
                    }

                    surf.update_after_resize(size);
                    *wsize = size;
                }
                WindowEvent::CloseRequested => {
                    if let Some(_) = wins.remove(&window_id) {
                        println!("Window with ID {:?} has been closed", window_id);
                    }
                }
                _ => (),
            },
            _ => (),
        }

        if wins.is_empty() {
            *control_flow = ControlFlow::Exit
        } else {
            *control_flow = ControlFlow::Wait
        }
    });
}
