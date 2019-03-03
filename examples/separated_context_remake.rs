mod support;

use glutin::{ContextTrait, EventsLoop, Window, WindowedContext};
use std::mem::ManuallyDrop;
use std::sync::Arc;

fn make_context(
    el: &EventsLoop,
    win: &Arc<Window>,
) -> (ManuallyDrop<WindowedContext>, support::Gl) {
    let separated_context = glutin::ContextBuilder::new()
        //.with_hardware_acceleration(None)
        .build_separated(Arc::clone(win), el)
        .unwrap();

    unsafe { separated_context.make_current().unwrap() }

    println!(
        "Pixel format of the window's GL context: {:?}",
        separated_context.get_pixel_format()
    );

    let gl = support::load(&separated_context.context());

    (ManuallyDrop::new(separated_context), gl)
}

fn main() {
    let mut el = glutin::EventsLoop::new();
    let win = glutin::WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&el)
        .unwrap();
    let win = Arc::new(win);

    let (mut separated_context, mut gl) = make_context(&el, &win);

    let mut running = true;
    let mut remake = false;
    while running {
        el.poll_events(|event| {
            println!("el {:?}", event);
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode:
                                    Some(glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | glutin::WindowEvent::CloseRequested => running = false,
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode:
                                    Some(glutin::VirtualKeyCode::R),
                                ..
                            },
                        ..
                    } => remake = true,
                    glutin::WindowEvent::Resized(logical_size) => {
                        let dpi_factor = win.get_hidpi_factor();
                        separated_context
                            .resize(logical_size.to_physical(dpi_factor));
                    }
                    _ => (),
                },
                _ => (),
            }
        });

        gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
        separated_context.swap_buffers().unwrap();

        if remake {
            println!("Remaking context.");
            unsafe {
                ManuallyDrop::drop(&mut separated_context);
            }
            let (new_separated_context, new_gl) = make_context(&el, &win);
            separated_context = new_separated_context;
            gl = new_gl;
            remake = false;
        }
    }
}
