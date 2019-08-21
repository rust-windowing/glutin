mod support;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
use glutin::Rect;

struct Color {
    red: f32,
    green: f32,
    blue: f32,
}

impl Color {
    fn new() -> Color {
        Color {
            red: 1.0,
            green: 0.5,
            blue: 0.0,
        }
    }
    fn next(&self) -> Color {
        Color {
            red: if self.red >= 1.0 { 0.0 } else { self.red + 0.01 },
            green: if self.green >= 1.0 { 0.0 } else { self.green + 0.01 },
            blue: if self.blue >= 1.0 { 0.0 } else { self.blue + 0.01 },
        }
    }
}

fn main() {
    let el = EventLoop::new();
    let wb = WindowBuilder::new().with_title("A fantastic window!");

    let windowed_context =
        ContextBuilder::new().build_windowed(wb, &el).unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    println!(
        "Pixel format of the window's GL context: {:?}",
        windowed_context.get_pixel_format()
    );

    let gl = support::load(&windowed_context.context());

    let mut color = Color::new();

    gl.draw_frame([color.red, color.green, color.blue, 1.0]);
    windowed_context.swap_buffers().unwrap();

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(logical_size) => {
                    let dpi_factor =
                        windowed_context.window().hidpi_factor();
                    windowed_context
                        .resize(logical_size.to_physical(dpi_factor));
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::CursorMoved{ .. } => {
                    // Select a new color to render, draw and swap buffers.
                    //
                    // Note that damage is *intentionally* being misreported
                    // here to display the effect of damage. All changes must
                    // be covered by the reported damage, as the compositor is
                    // free to read more from the buffer than damage was
                    // reported, such as when windows unhide.
                    //
                    // However, here we only damage the lower left corner to
                    // show that it is (usually) only the damage that gets
                    // composited to screen.
                    //
                    // Panics if damage is not supported due to the unwrap.
                    color = color.next();
                    gl.draw_frame([color.red, color.green, color.blue, 1.0]);
                    windowed_context.swap_buffers_with_damage(&[Rect{
                        x: 0,
                        y: 0,
                        height: 100,
                        width: 100,
                    }]).unwrap();
                }
                _ => (),
            },
            _ => (),
        }
    });
}
