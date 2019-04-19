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
    use takeable_option::Takeable;

    pub fn main() {
        let (raw_context, el, win) = {
            let el = glutin::event_loop::EventLoop::new();
            let win = glutin::window::WindowBuilder::new()
                .with_title("A fantastic window!")
                .build(&el)
                .unwrap();

            #[cfg(target_os = "linux")]
            unsafe {
                use glutin::platform::unix::{
                    EventLoopExtUnix, RawContextExt, WindowExtUnix,
                };

                let cb = glutin::ContextBuilder::new();
                let raw_context;

                if el.is_wayland() {
                    let dpi_factor = win.get_hidpi_factor();
                    let size =
                        win.get_inner_size().unwrap().to_physical(dpi_factor);
                    let (width, height): (u32, u32) = size.into();

                    let display_ptr =
                        win.get_wayland_display().unwrap() as *const _;
                    let surface = win.get_wayland_surface().unwrap();

                    raw_context = cb
                        .build_raw_wayland_context(
                            display_ptr,
                            surface,
                            width,
                            height,
                        )
                        .unwrap();
                } else {
                    let xconn = el.get_xlib_xconnection().unwrap();
                    let xwindow = win.get_xlib_window().unwrap();
                    raw_context =
                        cb.build_raw_x11_context(xconn, xwindow).unwrap();
                }

                (raw_context, el, win)
            }

            #[cfg(target_os = "windows")]
            unsafe {
                use glutin::platform::windows::{
                    RawContextExt, WindowExtWindows,
                };

                let hwnd = win.get_hwnd();
                let raw_context = glutin::ContextBuilder::new()
                    .build_raw_context(hwnd)
                    .unwrap();

                (raw_context, el, win)
            }
        };

        let raw_context = unsafe { raw_context.make_current().unwrap() };

        println!(
            "Pixel format of the window's GL context: {:?}",
            raw_context.get_pixel_format()
        );

        let gl = support::load(&*raw_context);

        let mut raw_context = Takeable::new(raw_context);
        el.run(move |event, _, control_flow| {
            println!("el {:?}", event);
            match event {
                glutin::event::Event::LoopDestroyed => {
                    Takeable::take(&mut raw_context); // Make sure it drops first
                    return;
                }
                glutin::event::Event::WindowEvent { ref event, .. } => {
                    match event {
                        glutin::event::WindowEvent::Resized(logical_size) => {
                            let dpi_factor = win.get_hidpi_factor();
                            raw_context
                                .resize(logical_size.to_physical(dpi_factor));
                        }
                        _ => (),
                    }
                }
                _ => (),
            }

            gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
            raw_context.swap_buffers().unwrap();

            match event {
                glutin::event::Event::WindowEvent {
                    event: glutin::event::WindowEvent::CloseRequested,
                    ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,
                _ => *control_flow = winit::event_loop::ControlFlow::Wait,
            }
        });
    }
}
