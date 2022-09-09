use std::num::NonZeroU32;

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use glutin::context::{ContextApi, ContextAttributesBuilder};
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
    let config = unsafe {
        gl_display
            .find_configs(template)
            .unwrap()
            .reduce(|accum, config| {
                // Find the config with the maximum number of samples.
                //
                // In general if you're not sure what you want in template you can request or
                // don't want to require multisampling for example, you can search for a
                // specific option you want afterwards.
                //
                // XXX however on macOS you can request only one config, so you should do
                // a search with the help of `find_configs` and adjusting your template.
                if config.num_samples() > accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
            .unwrap()
    };

    println!("Picked a config with {} samples", config.num_samples());

    // Create a wrapper for GL window and surface.
    let gl_window = GlWindow::from_existing(&gl_display, window, &config);

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(Some(raw_window_handle));
    let gl_context = unsafe {
        gl_display.create_context(&config, &context_attributes).unwrap_or_else(|_| {
            gl_display
                .create_context(&config, &fallback_context_attributes)
                .expect("failed to create context")
        })
    };

    // Make it current and load symbols.
    let gl_context = gl_context.make_current(&gl_window.surface).unwrap();

    // WGL requires current context on the calling thread to load symbols properly,
    // so the call here is for portability reasons. In case you don't target WGL
    // you can call it right after display creation.
    //
    // The symbol loading is done by the renderer.
    let renderer = Renderer::new(&gl_display);

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
                        renderer.resize(size.width as i32, size.height as i32);
                    }
                },
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                },
                _ => (),
            },
            Event::RedrawEventsCleared => {
                renderer.draw();
                gl_window.window.request_redraw();

                gl_window.surface.swap_buffers(&gl_context).unwrap();
            },
            _ => (),
        }
    });
}
