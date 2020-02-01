mod support;

use glutin::config::ConfigsFinder;
use glutin::context::ContextBuilder;
use glutin::surface::Surface;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_types::dpi::{PhysicalPosition, PhysicalSize, Rect};

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
            red: if self.red >= 1.0 {
                0.0
            } else {
                self.red + 0.01
            },
            green: if self.green >= 1.0 {
                0.0
            } else {
                self.green + 0.01
            },
            blue: if self.blue >= 1.0 {
                0.0
            } else {
                self.blue + 0.01
            },
        }
    }
}

fn main() {
    simple_logger::init().unwrap();
    let el = EventLoop::new();
    let wb = WindowBuilder::new().with_title("A fantastic window!");

    let confs = unsafe { ConfigsFinder::new().find(&*el).unwrap() };
    let conf = &confs[0];
    println!("Configeration chosen: {:?}", conf);

    let ctx = unsafe { ContextBuilder::new().build(conf).unwrap() };
    let (win, surf) = unsafe { Surface::new_window(conf, &*el, wb).unwrap() };

    unsafe { ctx.make_current(&surf).unwrap() }

    let gl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());

    let mut color = Color::new();
    gl.draw_frame([color.red, color.green, color.blue, 1.0]);
    surf.swap_buffers().unwrap();

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::RedrawRequested(_) => {
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
                surf.swap_buffers_with_damage(&[Rect {
                    pos: PhysicalPosition::new(0, 0),
                    size: PhysicalSize::new(100, 100),
                }])
                .unwrap();
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(size) => {
                    ctx.update_after_resize();
                    surf.update_after_resize(*size);
                    unsafe {
                        gl.gl.Viewport(0, 0, size.width as _, size.height as _);
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::CursorMoved { .. } => {
                    win.request_redraw();
                }
                _ => (),
            },
            _ => (),
        }
    });
}
