mod support;

use glutin::config::ConfigsFinder;
use glutin::context::ContextBuilder;
use glutin::surface::Surface;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::monitor::{MonitorHandle, VideoMode};
use winit::window::{Fullscreen, WindowBuilder};

use std::io::{stdin, stdout, Write};

fn main() {
    simple_logger::init().unwrap();
    let el = EventLoop::new();

    print!("Please choose the fullscreen mode: (1) exclusive, (2) borderless: ");
    stdout().flush().unwrap();

    let mut num = String::new();
    stdin().read_line(&mut num).unwrap();
    let num = num.trim().parse().ok().expect("Please enter a number");

    let fullscreen = Some(match num {
        1 => Fullscreen::Exclusive(prompt_for_video_mode(&prompt_for_monitor(&el))),
        2 => Fullscreen::Borderless(prompt_for_monitor(&el)),
        _ => panic!("[winit] Please enter a valid number"),
    });

    let mut is_maximized = false;
    let mut decorations = true;

    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_fullscreen(fullscreen.clone());

    let confs = unsafe { ConfigsFinder::new().find(&*el).unwrap() };
    let conf = &confs[0];
    println!("Configeration chosen: {:?}", conf);

    let ctx = unsafe { ContextBuilder::new().build(conf).unwrap() };
    let (win, surf) = unsafe { Surface::new_window(conf, &*el, wb).unwrap() };

    unsafe { ctx.make_current(&surf).unwrap() }

    let gl = support::Gl::load(|s| ctx.get_proc_address(s).unwrap());

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::MainEventsCleared => {
                win.request_redraw();
            }
            Event::RedrawRequested(_) => {
                gl.draw_frame([1.0, 0.5, 0.7, 1.0]);
                surf.swap_buffers().unwrap();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    ctx.update_after_resize();
                    surf.update_after_resize(size);
                    unsafe {
                        gl.gl.Viewport(0, 0, size.width as _, size.height as _);
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state,
                            ..
                        },
                    ..
                } => match (virtual_code, state) {
                    (VirtualKeyCode::Escape, _) => *control_flow = ControlFlow::Exit,
                    (VirtualKeyCode::F, ElementState::Pressed) => {
                        if win.fullscreen().is_some() {
                            win.set_fullscreen(None).unwrap();
                        } else {
                            win.set_fullscreen(fullscreen.clone()).unwrap();
                        }
                    }
                    (VirtualKeyCode::S, ElementState::Pressed) => {
                        println!("win.fullscreen {:?}", win.fullscreen());
                    }
                    (VirtualKeyCode::M, ElementState::Pressed) => {
                        is_maximized = !is_maximized;
                        win.set_maximized(is_maximized);
                    }
                    (VirtualKeyCode::D, ElementState::Pressed) => {
                        decorations = !decorations;
                        win.set_decorations(decorations);
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
