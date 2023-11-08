use std::error::Error;
use std::ffi::{CStr, CString};
use std::num::NonZeroU32;
use std::ops::Deref;

use raw_window_handle::HasRawWindowHandle;
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

use glutin::config::{ColorBufferType, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay as _;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, SwapInterval};

use glutin_winit::{self, DisplayBuilder, GlWindow};

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}

pub fn main(event_loop: winit::event_loop::EventLoop<()>) -> Result<(), Box<dyn Error>> {
    // Only Windows requires the window to be present before creating the display.
    // Other platforms don't really need one.
    //
    // XXX if you don't care about running on Android or so you can safely remove
    // this condition and always pass the window builder.
    let window_builder = cfg!(wgl_backend).then(|| {
        WindowBuilder::new()
            .with_transparent(true)
            .with_title("Glutin triangle gradient example (press Escape to exit)")
    });

    // The template will match only the configurations supporting rendering
    // to windows.
    //
    // XXX We force transparency only on macOS, given that EGL on X11 doesn't
    // have it, but we still want to show window. The macOS situation is like
    // that, because we can query only one config at a time on it, but all
    // normal platforms will return multiple configs, so we can find the config
    // with transparency ourselves inside the `reduce`.
    let template = ConfigTemplateBuilder::new()
        // The default is 8, but then we miss out on R10G10B10A2 formats.  Requesting at least 2
        // bits for alpha allows us to find it. Note that the default ConfigTemplate still requests
        // at least 8 bits for RGB.
        .with_alpha_size(2)
        // .with_float_pixels(false)
        .with_transparency(cfg!(cgl_backend));

    let display_builder = DisplayBuilder::new().with_window_builder(window_builder);

    let (mut window, gl_config) = display_builder.build(&event_loop, template, |configs| {
        // Find the config with the maximum number of samples, so our triangle will
        // be smooth.
        configs
            .reduce(|accum, config| {
                let mut higher_bit_depth = false;
                if let Some(ColorBufferType::Rgb { r_size, g_size, b_size }) =
                    accum.color_buffer_type()
                {
                    if let Some(ColorBufferType::Rgb { r_size: r, g_size: g, b_size: b }) =
                        config.color_buffer_type()
                    {
                        higher_bit_depth = r > r_size || g > g_size || b > b_size;
                    }
                }
                let transparency_check = config.supports_transparency().unwrap_or(false)
                    & !accum.supports_transparency().unwrap_or(false);

                // TODO: Note that there's no preference order here. We accept the new config
                // based on any of these changes
                if transparency_check
                    || config.num_samples() > accum.num_samples()
                    || higher_bit_depth
                {
                    config
                } else {
                    accum
                }
            })
            .unwrap()
    })?;

    println!(
        "Picked a config with {} samples, transparency {:?}, pixel layout {:?}, alpha {}",
        gl_config.num_samples(),
        gl_config.supports_transparency(),
        gl_config.color_buffer_type(),
        gl_config.alpha_size(),
    );

    let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());

    // XXX The display could be obtained from any object created by it, so we can
    // query it from the config.
    let gl_display = gl_config.display();

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(raw_window_handle);

    // There are also some old devices that support neither modern OpenGL nor GLES.
    // To support these we can try and create a 2.1 context.
    let legacy_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
        .build(raw_window_handle);

    let mut not_current_gl_context = Some(unsafe {
        gl_display.create_context(&gl_config, &context_attributes).unwrap_or_else(|_| {
            gl_display.create_context(&gl_config, &fallback_context_attributes).unwrap_or_else(
                |_| {
                    gl_display
                        .create_context(&gl_config, &legacy_context_attributes)
                        .expect("failed to create context")
                },
            )
        })
    });

    let mut state = None;
    let mut renderer = None;
    event_loop.run(move |event, window_target| {
        match event {
            Event::Resumed => {
                #[cfg(android_platform)]
                println!("Android window available");

                let window = window.take().unwrap_or_else(|| {
                    let window_builder = WindowBuilder::new()
                        .with_transparent(true)
                        .with_title("Glutin triangle gradient example (press Escape to exit)");
                    glutin_winit::finalize_window(window_target, window_builder, &gl_config)
                        .unwrap()
                });

                // sRGB sufaces should serve no purpose unless we call
                // gl.Enable(gl::FRAMEBUFFER_SRGB);
                let surface_attribs = SurfaceAttributesBuilder::new(); //.with_srgb(Some(false));
                let attrs = window.build_surface_attributes(surface_attribs);

                #[cfg(egl_backend)]
                let gl_surface = if let glutin::config::Config::Egl(egl_config) = &gl_config {
                    use glutin::api::egl::surface::{ColorSpace, EglSurfaceAttributes};
                    use glutin::display::GetDisplayExtensions;

                    let egl_display = egl_config.display();
                    let exts = egl_display.extensions();

                    // Test color space availability. Note that this creates "ugly" output as our
                    // shader still writes linear color values
                    let s = [
                        Some(ColorSpace::Bt2020Hlg),
                        Some(ColorSpace::Bt2020Pq),
                        Some(ColorSpace::Bt2020Linear),
                        Some(ColorSpace::DisplayP3),
                        Some(ColorSpace::DisplayP3Linear),
                        Some(ColorSpace::Linear),
                        None,
                    ]
                    .into_iter()
                    .find_map(|color_space| {
                        // Skip color spaces for which the extension is not available, to prevent
                        // receiving EGL_BAD_ATTRIBUTE
                        if let Some(color_space) = color_space {
                            if !exts.contains(color_space.egl_extension_name()) {
                                return None;
                            }
                        }
                        let egl_attrs =
                            EglSurfaceAttributes { attributes: attrs.clone(), color_space };
                        // TODO: Careful here that it uses the EGL function, or deref will pass
                        // non-EGL surface attributes to the non-EGL trait function!
                        // Should we use a different name and/or remove deref?
                        match unsafe { egl_display.create_window_surface(egl_config, &egl_attrs) } {
                            Err(e) => {
                                eprintln!("Color space {color_space:?} not supported: {e:?}");
                                None
                            },
                            Ok(s) => Some(s),
                        }
                    })
                    .expect("Could not create surface");
                    println!("Picked surface with color space {:?}", s.color_space());
                    glutin::surface::Surface::Egl(s)
                } else {
                    unsafe { gl_display.create_window_surface(&gl_config, &attrs) }.unwrap()
                };

                #[cfg(not(egl_backend))]
                let gl_surface =
                    unsafe { gl_display.create_window_surface(&gl_config, &attrs) }.unwrap();

                // Make it current.
                let gl_context =
                    not_current_gl_context.take().unwrap().make_current(&gl_surface).unwrap();

                // The context needs to be current for the Renderer to set up shaders and
                // buffers. It also performs function loading, which needs a current context on
                // WGL.
                renderer.get_or_insert_with(|| Renderer::new(&gl_display));

                // Try setting vsync.
                if let Err(res) = gl_surface
                    .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
                {
                    eprintln!("Error setting vsync: {res:?}");
                }

                assert!(state.replace((gl_context, gl_surface, window)).is_none());
            },
            Event::Suspended => {
                // This event is only raised on Android, where the backing NativeWindow for a GL
                // Surface can appear and disappear at any moment.
                println!("Android window removed");

                // Destroy the GL Surface and un-current the GL Context before ndk-glue releases
                // the window back to the system.
                let (gl_context, ..) = state.take().unwrap();
                assert!(not_current_gl_context
                    .replace(gl_context.make_not_current().unwrap())
                    .is_none());
            },
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    if size.width != 0 && size.height != 0 {
                        // Some platforms like EGL require resizing GL surface to update the size
                        // Notable platforms here are Wayland and macOS, other don't require it
                        // and the function is no-op, but it's wise to resize it for portability
                        // reasons.
                        if let Some((gl_context, gl_surface, _)) = &state {
                            gl_surface.resize(
                                gl_context,
                                NonZeroU32::new(size.width).unwrap(),
                                NonZeroU32::new(size.height).unwrap(),
                            );
                            let renderer = renderer.as_ref().unwrap();
                            renderer.resize(size.width as i32, size.height as i32);
                        }
                    }
                },
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event: KeyEvent { logical_key: Key::Named(NamedKey::Escape), .. },
                    ..
                } => window_target.exit(),
                _ => (),
            },
            Event::AboutToWait => {
                if let Some((gl_context, gl_surface, window)) = &state {
                    let renderer = renderer.as_ref().unwrap();
                    renderer.draw();
                    window.request_redraw();

                    gl_surface.swap_buffers(gl_context).unwrap();
                }
            },
            _ => (),
        }
    })?;

    Ok(())
}

pub struct Renderer {
    program: gl::types::GLuint,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
    gl: gl::Gl,
}

impl Renderer {
    pub fn new<D: GlDisplay>(gl_display: &D) -> Self {
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

            self.gl.ClearColor(0.1, 0.1, 0.1, 0.7);
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

impl Deref for Renderer {
    type Target = gl::Gl;

    fn deref(&self) -> &Self::Target {
        &self.gl
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
    shader_type: gl::types::GLenum,
    source: &[u8],
) -> gl::types::GLuint {
    let shader = gl.CreateShader(shader_type);
    gl.ShaderSource(shader, 1, [source.as_ptr().cast()].as_ptr(), std::ptr::null());
    gl.CompileShader(shader);
    let mut len = 0;
    gl.GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
    if len > 0 {
        let mut log = Vec::<u8>::with_capacity(len as usize);
        gl.GetShaderInfoLog(shader, len, &mut len, log.as_mut_ptr().cast());
        log.set_len(len as usize);
        log.push(0);
        let log = CString::from_vec_with_nul(log).unwrap();
        eprintln!("Shader {shader_type:?} log");
        eprintln!("{}", log.to_string_lossy());
    }
    let mut status = 0;
    gl.GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    assert_eq!(status, 1, "Shader {shader_type:?} compilation failed");
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
