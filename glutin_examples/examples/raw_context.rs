#[cfg(any(target_os = "linux", target_os = "windows"))]
mod support;

fn main() {
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    unimplemented!();
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    this_example::main();
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
mod this_example {
    use super::support;
    use glutin::event::{Event, WindowEvent};
    use glutin::event_loop::{ControlFlow, EventLoop};
    use glutin::window::WindowBuilder;
    use glutin::ContextBuilder;
    use std::io::Write;
    use takeable_option::Takeable;

    pub fn main() {
        print!("Do you want transparency? (true/false) (default: true): ");
        std::io::stdout().flush().unwrap();

        let mut transparency = String::new();
        std::io::stdin().read_line(&mut transparency).unwrap();
        let transparency = transparency.trim().parse().unwrap_or_else(|_| {
            println!("Unknown input, assumming true.");
            true
        });

        let (raw_context, el) = {
            let el = EventLoop::new();
            let mut wb = WindowBuilder::new().with_title("A fantastic window!");

            if transparency {
                wb = wb.with_decorations(false).with_transparent(true);
            }

            #[cfg(target_os = "linux")]
            unsafe {
                use glutin::platform::unix::{
                    EventLoopWindowTargetExtUnix, RawContextExt, WindowExtUnix,
                };

                if el.is_wayland() {
                    let win = wb.build(&el).unwrap();
                    let size = win.inner_size();
                    let (width, height): (u32, u32) = size.into();

                    let display_ptr = win.wayland_display().unwrap() as *const _;
                    let surface = win.wayland_surface().unwrap();

                    let raw_context = ContextBuilder::new()
                        .build_raw_wayland_context(display_ptr, surface, width, height)
                        .unwrap();

                    (raw_context, el)
                } else {
                    if transparency {
                        unimplemented!(
                            r#"
Users should make sure that the window gets built with an x11 visual that
supports transparency. Winit does not currently do this by default for x11
because it is not provided with enough details to make a good choice. Normally
glutin decides this for winit, but this is not the case for raw contexts.

Depending on the default order of the x11 visuals, transparency may by sheer
luck work for you.

Such a task of selecting the appropriate x11 visual is outside the limited
scope of the glutin examples. Implementing it would likely require a lot of
platform specific egl/glx/x11 calls or exposing a lot of glutin's internals.
File a PR if you are interested in implementing the latter.
                        "#
                        )
                    }

                    let win = wb.build(&el).unwrap();
                    let xconn = el.xlib_xconnection().unwrap();
                    let xwindow = win.xlib_window().unwrap();
                    let raw_context =
                        ContextBuilder::new().build_raw_x11_context(xconn, xwindow).unwrap();

                    (raw_context, el)
                }
            }

            #[cfg(target_os = "windows")]
            unsafe {
                let win = wb.build(&el).unwrap();
                use glutin::platform::windows::{RawContextExt, WindowExtWindows};

                let hwnd = win.hwnd();
                let raw_context = ContextBuilder::new().build_raw_context(hwnd).unwrap();

                (raw_context, el)
            }
        };

        let raw_context = unsafe { raw_context.make_current().unwrap() };

        println!("Pixel format of the window's GL context: {:?}", raw_context.get_pixel_format());

        let gl = support::load(&*raw_context);

        let mut raw_context = Takeable::new(raw_context);
        el.run(move |event, _, control_flow| {
            println!("el {:?}", event);
            *control_flow = ControlFlow::Wait;

            match event {
                Event::LoopDestroyed => {
                    Takeable::take(&mut raw_context); // Make sure it drops first
                    return;
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(physical_size) => raw_context.resize(physical_size),
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => (),
                },
                Event::RedrawRequested(_) => {
                    gl.draw_frame(if transparency { [0.0; 4] } else { [1.0, 0.5, 0.7, 1.0] });
                    raw_context.swap_buffers().unwrap();
                }
                _ => (),
            }
        });
    }
}
