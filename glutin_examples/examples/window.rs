use std::ffi::CString;
use std::num::NonZeroU32;

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use glutin::context::ContextAttributesBuilder;
use glutin::prelude::*;
use glutin::surface::SwapInterval;

mod support;

use support::*;

fn main() {
    let event_loop = EventLoop::new();

    let raw_display = event_loop.raw_display_handle();

    // We create a window before the display to accomodate for WGL, since it
    // requires creating HDC for properly loading the WGL and it should be taken
    // from the window you'll be rendering into.
    let window = WindowBuilder::new().with_transparent(true).build(&event_loop).unwrap();
    let raw_window_handle = window.raw_window_handle();

    // Create the GL display. This will create display automatically for the
    // underlying GL platform. See support module on how it's being done.
    let gl_display = create_display(raw_display, raw_window_handle);

    // Create the config we'll be used for window. We'll use the native window
    // raw-window-handle for it to get the right visual and use proper hdc. Note
    // that you can likely use it for other windows using the same config.
    let template = config_template(window.raw_window_handle());
    let config = unsafe { gl_display.find_configs(template).unwrap().next().unwrap() };

    // Create a wrapper for GL window and surface.
    let gl_window = GlWindow::from_existing(&gl_display, window, &config);

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));
    let gl_context = unsafe { gl_display.create_context(&config, &context_attributes).unwrap() };

    // Make it current and load symbols.
    let gl_context = gl_context.make_current(&gl_window.surface).unwrap();

    gl::load_with(|symbol| {
        let symbol = CString::new(symbol).unwrap();
        gl_context.get_proc_address(symbol.as_c_str()) as *const _
    });

    // Try setting vsync.
    if let Err(res) = gl_window
        .surface
        .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
    {
        eprintln!("Error setting vsync: {:?}", res);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    if size.width != 0 && size.height != 0 {
                        // Some platforms like EGL require resizing GL surface to update the size
                        // Notable platforms here are Wayland and macOS, other don't require it
                        // and the function is no-op, but it's wise to resize it for portability
                        // reasons.
                        gl_window.surface.resize(
                            &gl_context,
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        );
                    }
                },
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                },
                _ => (),
            },
            Event::RedrawEventsCleared => {
                unsafe {
                    gl::ClearColor(0., 0.3, 0.3, 0.8);
                    gl::Clear(gl::COLOR_BUFFER_BIT);
                    gl_window.window.request_redraw();
                }

                gl_window.surface.swap_buffers(&gl_context).unwrap();
            },
            _ => (),
        }
    });
}
