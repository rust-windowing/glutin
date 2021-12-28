use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
#[path = "../../../examples/support/mod.rs"]
mod support;

fn main() {
    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("A transparent window!")
        .with_decorations(false)
        .with_transparent(true);

    let windowed_context = ContextBuilder::new().build_windowed(wb, &el).unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    println!("Pixel format of the window's GL context: {:?}", windowed_context.get_pixel_format());
    let gl = support::load(&windowed_context.context());
    let mut inc: f32 = 0.0;

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(physical_size) => windowed_context.resize(physical_size),
                WindowEvent::Touch(_touch) => {
                    const INCREMENTER: f32 = 0.05;
                    inc += INCREMENTER;
                    gl.draw_frame([
                        inc % 1.0,
                        (inc + INCREMENTER) % 1.0,
                        (inc + INCREMENTER) % 1.0,
                        0.0,
                    ]);
                    windowed_context.swap_buffers().unwrap();
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::RedrawRequested(_) => {
                gl.draw_frame([0.0; 4]);
                windowed_context.swap_buffers().unwrap();
            }
            _ => (),
        }
    });
}

#[no_mangle]
pub extern "C" fn run_app() {
    main();
}
