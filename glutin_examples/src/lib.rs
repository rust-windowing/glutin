//! Support module for the glutin examples.
#![allow(dead_code)]
#![allow(unused_variables)]

use std::ffi::{CStr, CString};
use std::num::NonZeroU32;

use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
#[cfg(glx_backend)]
use winit::platform::unix;
use winit::window::{Window, WindowBuilder};

use glutin::config::{Config, ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder};
use glutin::display::{Display, DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{
    Surface, SurfaceAttributes, SurfaceAttributesBuilder, SwapInterval, WindowSurface,
};

pub fn main() {
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

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}

/// Structure to hold winit window and gl surface.
pub struct GlWindow {
    pub surface: Surface<WindowSurface>,
    pub window: Window,
}

impl GlWindow {
    pub fn new<T>(event_loop: &EventLoop<T>, display: &Display, config: &Config) -> Self {
        let window = WindowBuilder::new().with_transparent(true).build(event_loop).unwrap();
        let attrs = surface_attributes(&window);
        let surface = unsafe { display.create_window_surface(config, &attrs).unwrap() };
        Self { window, surface }
    }

    pub fn from_existing(display: &Display, window: Window, config: &Config) -> Self {
        let attrs = surface_attributes(&window);
        let surface = unsafe { display.create_window_surface(config, &attrs).unwrap() };
        Self { window, surface }
    }
}

/// Create template to find OpenGL config.
pub fn config_template(raw_window_handle: RawWindowHandle) -> ConfigTemplate {
    let builder = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .compatible_with_native_window(raw_window_handle)
        .with_surface_type(ConfigSurfaceTypes::WINDOW);

    #[cfg(cgl_backend)]
    let builder = builder.with_transparency(true).with_multisampling(8);

    builder.build()
}

/// Create surface attributes for window surface.
pub fn surface_attributes(window: &Window) -> SurfaceAttributes<WindowSurface> {
    let (width, height): (u32, u32) = window.inner_size().into();
    let raw_window_handle = window.raw_window_handle();
    SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    )
}

/// Create the display.
pub fn create_display(
    raw_display: RawDisplayHandle,
    raw_window_handle: RawWindowHandle,
) -> Display {
    #[cfg(egl_backend)]
    let preference = DisplayApiPreference::Egl;

    #[cfg(glx_backend)]
    let preference = DisplayApiPreference::Glx(Box::new(unix::register_xlib_error_hook));

    #[cfg(cgl_backend)]
    let preference = DisplayApiPreference::Cgl;

    #[cfg(wgl_backend)]
    let preference = DisplayApiPreference::Wgl(Some(raw_window_handle));

    #[cfg(all(egl_backend, wgl_backend))]
    let preference = DisplayApiPreference::WglThenEgl(Some(raw_window_handle));

    #[cfg(all(egl_backend, glx_backend))]
    let preference = DisplayApiPreference::GlxThenEgl(Box::new(unix::register_xlib_error_hook));

    // Create connection to underlying OpenGL client Api.
    unsafe { Display::from_raw(raw_display, preference).unwrap() }
}

pub struct Renderer {
    program: gl::types::GLuint,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
    gl: gl::Gl,
}

impl Renderer {
    pub fn new(gl_display: &Display) -> Self {
        unsafe {
            let gl = gl::Gl::load_with(|symbol| {
                let symbol = CString::new(symbol).unwrap();
                gl_display.get_proc_address(symbol.as_c_str()).cast()
            });

            if let Some(renderer) = get_gl_string(&gl, gl::RENDERER) {
                println!("Running on {}", renderer.to_string_lossy());
            }
            if let Some(version) = get_gl_string(&gl, gl::VERSION) {
                println!("OpenGL Version {}", version.to_string_lossy());
            }

            if let Some(shaders_version) = get_gl_string(&gl, gl::SHADING_LANGUAGE_VERSION) {
                println!("Shaders version on {}", shaders_version.to_string_lossy());
            }

            let vertex_shader = create_shader(&gl, gl::VERTEX_SHADER, VERTEX_SHADER_SOURCE);
            let fragment_shader = create_shader(&gl, gl::FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE);

            let program = gl.CreateProgram();

            gl.AttachShader(program, vertex_shader);
            gl.AttachShader(program, fragment_shader);

            gl.LinkProgram(program);

            gl.UseProgram(program);

            gl.DeleteShader(vertex_shader);
            gl.DeleteShader(fragment_shader);

            let mut vao = std::mem::zeroed();
            gl.GenVertexArrays(1, &mut vao);
            gl.BindVertexArray(vao);

            let mut vbo = std::mem::zeroed();
            gl.GenBuffers(1, &mut vbo);
            gl.BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl.BufferData(
                gl::ARRAY_BUFFER,
                (VERTEX_DATA.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                VERTEX_DATA.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            let pos_attrib = gl.GetAttribLocation(program, b"position\0".as_ptr() as *const _);
            let color_attrib = gl.GetAttribLocation(program, b"color\0".as_ptr() as *const _);
            gl.VertexAttribPointer(
                pos_attrib as gl::types::GLuint,
                2,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                std::ptr::null(),
            );
            gl.VertexAttribPointer(
                color_attrib as gl::types::GLuint,
                3,
                gl::FLOAT,
                0,
                5 * std::mem::size_of::<f32>() as gl::types::GLsizei,
                (2 * std::mem::size_of::<f32>()) as *const () as *const _,
            );
            gl.EnableVertexAttribArray(pos_attrib as gl::types::GLuint);
            gl.EnableVertexAttribArray(color_attrib as gl::types::GLuint);

            Self { program, vao, vbo, gl }
        }
    }

    pub fn draw(&self) {
        unsafe {
            self.gl.UseProgram(self.program);

            self.gl.BindVertexArray(self.vao);
            self.gl.BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            self.gl.ClearColor(0.1, 0.1, 0.1, 0.9);
            self.gl.Clear(gl::COLOR_BUFFER_BIT);
            self.gl.DrawArrays(gl::TRIANGLES, 0, 3);
        }
    }

    pub fn resize(&self, width: i32, height: i32) {
        unsafe {
            self.gl.Viewport(0, 0, width, height);
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteProgram(self.program);
            self.gl.DeleteBuffers(1, &self.vbo);
            self.gl.DeleteVertexArrays(1, &self.vao);
        }
    }
}

unsafe fn create_shader(
    gl: &gl::Gl,
    shader: gl::types::GLenum,
    source: &[u8],
) -> gl::types::GLuint {
    let shader = gl.CreateShader(shader);
    gl.ShaderSource(shader, 1, [source.as_ptr().cast()].as_ptr(), std::ptr::null());
    gl.CompileShader(shader);
    shader
}

fn get_gl_string(gl: &gl::Gl, variant: gl::types::GLenum) -> Option<&'static CStr> {
    unsafe {
        let s = gl.GetString(variant);
        (!s.is_null()).then(|| CStr::from_ptr(s.cast()))
    }
}

#[rustfmt::skip]
static VERTEX_DATA: [f32; 15] = [
    -0.5, -0.5,  1.0,  0.0,  0.0,
     0.0,  0.5,  0.0,  1.0,  0.0,
     0.5, -0.5,  0.0,  0.0,  1.0,
];

const VERTEX_SHADER_SOURCE: &[u8] = b"
#version 100
precision mediump float;

attribute vec2 position;
attribute vec3 color;

varying vec3 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_color = color;
}
\0";

const FRAGMENT_SHADER_SOURCE: &[u8] = b"
#version 100
precision mediump float;

varying vec3 v_color;

void main() {
    gl_FragColor = vec4(v_color, 1.0);
}
\0";
