mod support;

use glutin::event::{
    ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent,
};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::monitor::{MonitorHandle, VideoMode};
use glutin::window::{Fullscreen, WindowBuilder};
use std::io::{stdin, stdout, Write};

fn main() {
    let el = EventLoop::new();

    print!(
        "Please choose the fullscreen mode: (1) exclusive, (2) borderless: "
    );
    stdout().flush().unwrap();

    let mut num = String::new();
    stdin().read_line(&mut num).unwrap();
    let num = num.trim().parse().ok().expect("Please enter a number");

    let fullscreen = Some(match num {
        1 => Fullscreen::Exclusive(prompt_for_video_mode(&prompt_for_monitor(
            &el,
        ))),
        2 => Fullscreen::Borderless(prompt_for_monitor(&el)),
        _ => panic!("Please enter a valid number"),
    });

    println!("Press (F) to toggle fullscreen, (D) to toggle window decorations, and (M) to toggle maximized/minimized.");

    let mut is_maximized = false;
    let mut decorations = true;

    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_fullscreen(fullscreen.clone());
    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(wb, &el)
        .unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let gl = support::load(&windowed_context.context());

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::Resized(physical_size) => {
                    windowed_context.resize(physical_size);
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
                        if windowed_context.window().fullscreen().is_some() {
                            windowed_context.window().set_fullscreen(None);
                        } else {
                            windowed_context
                                .window()
                                .set_fullscreen(fullscreen.clone());
                        }
                    }
                    (VirtualKeyCode::S, ElementState::Pressed) => {
                        println!(
                            "window.fullscreen {:?}",
                            windowed_context.window().fullscreen()
                        );
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
            Event::RedrawRequested(_) => {
                gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
                windowed_context.swap_buffers().unwrap();
            }
            _ => {}
        }
    });
}

// Enumerate monitors and prompt user to choose one
fn prompt_for_monitor(el: &EventLoop<()>) -> MonitorHandle {
    for (num, monitor) in el.available_monitors().enumerate() {
        println!("Monitor #{}: {:?}", num, monitor.name());
    }

    print!("Please write the number of the monitor to use: ");
    stdout().flush().unwrap();

    let mut num = String::new();
    stdin().read_line(&mut num).unwrap();
    let num = num.trim().parse().ok().expect("Please enter a number");
    let monitor = el
        .available_monitors()
        .nth(num)
        .expect("Please enter a valid ID");

    println!("Using {:?}", monitor.name());

    monitor
}

fn prompt_for_video_mode(monitor: &MonitorHandle) -> VideoMode {
    for (i, video_mode) in monitor.video_modes().enumerate() {
        println!("Video mode #{}: {}", i, video_mode);
    }

    print!("Please write the number of the video mode to use: ");
    stdout().flush().unwrap();

    let mut num = String::new();
    stdin().read_line(&mut num).unwrap();
    let num = num.trim().parse().ok().expect("Please enter a number");
    let video_mode = monitor
        .video_modes()
        .nth(num)
        .expect("Please enter a valid ID");

    println!("Using {}", video_mode);

    video_mode
}
