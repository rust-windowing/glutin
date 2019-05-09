mod support;

use glutin::event::{
    ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent,
};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::monitor::MonitorHandle;
use glutin::window::WindowBuilder;
use std::io::Write;

fn main() {
    let el = EventLoop::new();

    #[cfg(target_os = "macos")]
    let mut macos_use_simple_fullscreen = false;

    let monitor = {
        // On macOS there are two fullscreen modes "native" and "simple"
        #[cfg(target_os = "macos")]
        {
            print!(
                "Please choose the fullscreen mode: (1) native, (2) simple: "
            );
            std::io::stdout().flush().unwrap();

            let mut num = String::new();
            std::io::stdin().read_line(&mut num).unwrap();
            let num = num.trim().parse().ok().expect("Please enter a number");
            match num {
                2 => macos_use_simple_fullscreen = true,
                _ => {}
            }

            // Prompt for monitor when using native fullscreen
            if !macos_use_simple_fullscreen {
                Some(prompt_for_monitor(&el))
            } else {
                None
            }
        }

        #[cfg(not(target_os = "macos"))]
        Some(prompt_for_monitor(&el))
    };

    let mut is_fullscreen = monitor.is_some();
    let mut is_maximized = false;
    let mut decorations = true;

    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_fullscreen(monitor);
    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(wb, &el)
        .unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let gl = support::load(&windowed_context.context());

    el.run(move |event, _, control_flow| {
        println!("{:?}", event);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(logical_size) => {
                    let dpi_factor =
                        windowed_context.window().get_hidpi_factor();
                    windowed_context
                        .resize(logical_size.to_physical(dpi_factor));
                }
                WindowEvent::RedrawRequested => {
                    gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
                    windowed_context.swap_buffers().unwrap();
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state,
                            ..
                        },
                    ..
                } => match (virtual_code, state) {
                    (VirtualKeyCode::Escape, _) => {
                        *control_flow = ControlFlow::Exit
                    }
                    (VirtualKeyCode::F, ElementState::Pressed) => {
                        #[cfg(target_os = "macos")]
                        {
                            if macos_use_simple_fullscreen {
                                use glutin::platform::macos::WindowExtMacOS;
                                if WindowExtMacOS::set_simple_fullscreen(
                                    windowed_context.window(),
                                    !is_fullscreen,
                                ) {
                                    is_fullscreen = !is_fullscreen;
                                }
                                return;
                            }
                        }

                        is_fullscreen = !is_fullscreen;
                        if !is_fullscreen {
                            windowed_context.window().set_fullscreen(None);
                        } else {
                            windowed_context.window().set_fullscreen(Some(
                                windowed_context.window().get_current_monitor(),
                            ));
                        }
                    }
                    (VirtualKeyCode::M, ElementState::Pressed) => {
                        is_maximized = !is_maximized;
                        windowed_context.window().set_maximized(is_maximized);
                    }
                    (VirtualKeyCode::D, ElementState::Pressed) => {
                        decorations = !decorations;
                        windowed_context.window().set_decorations(decorations);
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => {}
        }
    });
}

// Enumerate monitors and prompt user to choose one
fn prompt_for_monitor(el: &EventLoop<()>) -> MonitorHandle {
    for (num, monitor) in el.get_available_monitors().enumerate() {
        println!("Monitor #{}: {:?}", num, monitor.get_name());
    }

    print!("Please write the number of the monitor to use: ");
    std::io::stdout().flush().unwrap();

    let mut num = String::new();
    std::io::stdin().read_line(&mut num).unwrap();
    let num = num.trim().parse().ok().expect("Please enter a number");
    let monitor = el
        .get_available_monitors()
        .nth(num)
        .expect("Please enter a valid ID");

    println!("Using {:?}", monitor.get_name());

    monitor
}
