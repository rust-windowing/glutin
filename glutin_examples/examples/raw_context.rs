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

    pub fn main() {
        let (raw_context, mut el, win) = {
            let el = glutin::EventsLoop::new();
            let win = glutin::WindowBuilder::new()
                .with_title("A fantastic window!")
                .build(&el)
                .unwrap();

            #[cfg(target_os = "linux")]
            unsafe {
                use glutin::os::unix::RawContextExt;
                use winit::os::unix::{EventsLoopExt, WindowExt};

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
                use glutin::os::windows::RawContextExt;
                use winit::os::windows::WindowExt;

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

        let mut running = true;
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
                        | glutin::WindowEvent::CloseRequested => {
                            running = false
                        }
                        glutin::WindowEvent::Resized(logical_size) => {
                            let dpi_factor = win.get_hidpi_factor();
                            raw_context
                                .resize(logical_size.to_physical(dpi_factor));
                        }
                        _ => (),
                    },
                    _ => (),
                }
            });

            gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
            raw_context.swap_buffers().unwrap();
        }

        std::mem::drop(raw_context) // Make sure it drops first
    }
}
