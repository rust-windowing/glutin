mod support;

use glutin::config::{ConfigsFinder, SwapInterval};
use glutin::context::ContextBuilder;
use glutin::surface::Surface;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use std::io::{stdin, stdout, Write};

fn prompt_vsync() -> SwapInterval {
    print!("Please write the swap interval to use: ");
    stdout().flush().unwrap();

    let mut num = String::new();
    stdin().read_line(&mut num).unwrap();
    let num: u32 = num.trim().parse().ok().expect("Please enter a number");

    match num {
        0 => SwapInterval::DontWait,
        _ => {
            print!("Adaptive swap [y/n]? ");
            stdout().flush().unwrap();

            let mut adaptive = String::new();
            stdin().read_line(&mut adaptive).unwrap();

            match &adaptive.trim().to_lowercase() as _ {
                "y" | "yes" | "true" => SwapInterval::AdaptiveWait(num),
                "n" | "no" | "false" => SwapInterval::Wait(num),
                _ => panic!("Please provide a valid responce."),
            }
        }
    }
}

fn main() {
    env_logger::init();
    let swap_interval = prompt_vsync();
    println!("Using {:?}", swap_interval);

    let el = EventLoop::new();
    let wb = WindowBuilder::new().with_title("A fantastic window!");

    let confs = ConfigsFinder::new()
        .with_desired_swap_interval_ranges(vec![swap_interval.into()])
        .find(&*el)
        .unwrap();
    let conf = &confs[0];
    println!("Configeration chosen: {:?}", conf);

    let ctx = ContextBuilder::new().build(conf).unwrap();
    let (win, surf) = unsafe { Surface::new_window(conf, &*el, wb).unwrap() };

    unsafe { ctx.make_current(&surf).unwrap() }
    let gl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());
    surf.modify_swap_interval(swap_interval).unwrap();

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::MainEventsCleared => {
                win.request_redraw();
            }
            Event::RedrawRequested(_) => {
                gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
                surf.swap_buffers().unwrap();
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(size) => {
                    ctx.update_after_resize();
                    surf.update_after_resize(size);
                    unsafe {
                        gl.gl.Viewport(0, 0, size.width as _, size.height as _);
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            _ => (),
        }
    });
}
