use std::error::Error;
use std::ffi::{CStr, CString};
use std::num::NonZeroU32;
use std::ops::Deref;

use gl::types::GLfloat;
use raw_window_handle::HasWindowHandle;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes};

use glutin::config::{Config, ConfigTemplateBuilder, GetGlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext, Version,
};
use glutin::display::{Display, GetGlDisplay};
use glutin::prelude::*;
use glutin::surface::{Surface, SwapInterval, WindowSurface};

use glutin_winit::{DisplayBuilder, GlWindow};

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}

pub fn main(event_loop: winit::event_loop::EventLoop<()>) -> Result<(), Box<dyn Error>> {
    // The template will match only the configurations supporting rendering
    // to windows.
    //
    // XXX We force transparency only on macOS, given that EGL on X11 doesn't
    // have it, but we still want to show window. The macOS situation is like
    // that, because we can query only one config at a time on it, but all
    // normal platforms will return multiple configs, so we can find the config
    // with transparency ourselves inside the `reduce`.
    let template =
        ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(cfg!(cgl_backend));

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes()));

    let mut app = App::new(template, display_builder);
    event_loop.run_app(&mut app)?;

    app.exit_state()
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.transition(event_loop, |state| match state {
            AppState::Uninitialized(state) => state.initialize(event_loop).map(AppState::Resumed),
            AppState::Resumed(state) => Ok(AppState::Resumed(state)),
            AppState::Suspended(state) => state.resume(event_loop).map(AppState::Resumed),
        });
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.transition(event_loop, |state| match state {
            AppState::Uninitialized { .. } => Err("invalid transition".into()),
            AppState::Resumed(state) => state.suspend().map(AppState::Suspended),
            AppState::Suspended(state) => Ok(AppState::Suspended(state)),
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    self.transition(event_loop, |state| {
                        match &state {
                            AppState::Uninitialized { .. } => {
                                return Err("invalid transition".into())
                            },
                            AppState::Resumed(state) => {
                                state.resize(width, height);
                            },
                            AppState::Suspended(AppStateSuspended { renderer, .. }) => {
                                // TODO: Should we call resize while suspended or should we not do
                                // anything here?
                                renderer.resize(size.width as i32, size.height as i32);
                            },
                        };
                        Ok(state)
                    });
                }
            },
            WindowEvent::RedrawRequested => {
                self.transition(event_loop, |state| {
                    match &state {
                        AppState::Uninitialized { .. } => return Err("invalid transition".into()),
                        AppState::Resumed(state) => {
                            state.redraw()?;
                        },
                        AppState::Suspended { .. } => {},
                    }
                    Ok(state)
                });
            },
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key: Key::Named(NamedKey::Escape), .. },
                ..
            } => event_loop.exit(),
            _ => (),
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.final_transition(|state| {
            // NOTE: The handling below is only needed due to nvidia on Wayland to not crash
            // on exit due to nvidia driver touching the Wayland display from on
            // `exit` hook.

            let _gl_display = match state {
                AppState::Uninitialized { .. } => return Err("invalid transition".into()),
                AppState::Resumed(AppStateResumed { gl_context, renderer, gl_surface, window }) => {
                    // Clear the window.
                    drop(gl_surface);
                    drop(window);
                    drop(renderer);
                    gl_context.display()
                },
                AppState::Suspended(AppStateSuspended { gl_context, renderer }) => {
                    drop(renderer);
                    gl_context.display()
                },
            };

            Ok(())
        });
    }
}

struct TerminateDisplayOnDrop<T: GetGlDisplay<Target = Display>>(T);

// impl TerminateDisplayOnDrop<NotCurrentContext> {
//     fn make_current(self, surface: ()) ->
// TerminateDisplayOnDrop<PossiblyCurrentContext> {         unsafe {
//             let old = std::mem::ManuallyDrop::new(self);
//             let context = std::ptr::read(std::ptr::from_ref(&old.0));
//             match
//             TerminateDisplayOnDrop(context.make_current(surface))
//         }
//     }
// }

impl<T: GetGlDisplay<Target = Display>> Drop for TerminateDisplayOnDrop<T> {
    fn drop(&mut self) {
        #[cfg(egl_backend)]
        #[allow(irrefutable_let_patterns)]
        if let glutin::display::Display::Egl(display) = self.0.display() {
            unsafe {
                display.terminate();
            }
        }
    }
}

fn create_gl_surface(
    window: &Window,
    gl_config: &Config,
) -> Result<Surface<WindowSurface>, Box<dyn Error>> {
    let attrs = window
        .build_surface_attributes(Default::default())
        .expect("Failed to build surface attributes");
    let gl_surface = unsafe { gl_config.display().create_window_surface(gl_config, &attrs)? };
    Ok(gl_surface)
}

fn create_gl_context(window: &Window, gl_config: &Config) -> NotCurrentContext {
    let raw_window_handle = window.window_handle().ok().map(|wh| wh.as_raw());

    // The context creation part.
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

    // Reuse the uncurrented context from a suspended() call if it exists, otherwise
    // this is the first time resumed() is called, where the context still
    // has to be created.
    let gl_display = gl_config.display();

    unsafe {
        gl_display.create_context(gl_config, &context_attributes).unwrap_or_else(|_| {
            gl_display.create_context(gl_config, &fallback_context_attributes).unwrap_or_else(
                |_| {
                    gl_display
                        .create_context(gl_config, &legacy_context_attributes)
                        .expect("failed to create context")
                },
            )
        })
    }
}

fn enable_vsync(gl_surface: &Surface<WindowSurface>, gl_context: &PossiblyCurrentContext) {
    // Try setting vsync.
    if let Err(res) =
        gl_surface.set_swap_interval(gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
    {
        eprintln!("Error setting vsync: {res:?}");
    }
}

fn window_attributes() -> WindowAttributes {
    Window::default_attributes()
        .with_transparent(true)
        .with_title("Glutin triangle gradient example (press Escape to exit)")
}

struct App {
    state: Option<AppState>,
    exit_state: Result<(), Box<dyn Error>>,
}

const INCONSISTENT: &str = "application was left in an inconsistent state";

impl App {
    fn new(template_builder: ConfigTemplateBuilder, display_builder: DisplayBuilder) -> Self {
        Self {
            state: Some(AppState::Uninitialized(AppStateUninitialized {
                template_builder,
                display_builder,
            })),
            exit_state: Ok(()),
        }
    }

    fn exit_state(self) -> Result<(), Box<dyn Error>> {
        debug_assert!(self.state.is_none());
        self.exit_state
    }

    fn transition<F: FnOnce(AppState) -> Result<AppState, Box<dyn Error>>>(
        &mut self,
        event_loop: &ActiveEventLoop,
        f: F,
    ) {
        match f(self.state.take().expect(INCONSISTENT)) {
            Ok(state) => self.state = Some(state),
            Err(error) => {
                event_loop.exit();
                self.exit_state = Err(error);
            },
        }
    }

    fn final_transition<F: FnOnce(AppState) -> Result<(), Box<dyn Error>>>(&mut self, f: F) {
        self.exit_state = f(self.state.take().expect(INCONSISTENT));
    }
}

enum AppState {
    Uninitialized(AppStateUninitialized),
    Resumed(AppStateResumed),
    Suspended(AppStateSuspended),
}

struct AppStateUninitialized {
    template_builder: ConfigTemplateBuilder,
    display_builder: DisplayBuilder,
}

impl AppStateUninitialized {
    fn initialize(self, event_loop: &ActiveEventLoop) -> Result<AppStateResumed, Box<dyn Error>> {
        let Self { template_builder, display_builder } = self;
        let (window, gl_config) =
            display_builder.build(event_loop, template_builder, gl_config_picker)?;
        let window = window.ok_or("failed to create window")?;
        println!("Picked a config with {} samples", gl_config.num_samples());
        let gl_context = create_gl_context(&window, &gl_config);
        let gl_surface = create_gl_surface(&window, &gl_config)?;
        let gl_context = gl_context.make_current(&gl_surface)?;
        enable_vsync(&gl_surface, &gl_context);
        let renderer = Renderer::new(&gl_config.display());
        Ok(AppStateResumed { window, gl_surface, gl_context, renderer })
    }
}

struct AppStateResumed {
    gl_context: PossiblyCurrentContext,
    renderer: Renderer,
    // NOTE: Window should be dropped after all resources created using its
    // raw-window-handle.
    gl_surface: Surface<WindowSurface>,
    window: Window,
}

impl AppStateResumed {
    fn suspend(self: AppStateResumed) -> Result<AppStateSuspended, Box<dyn Error>> {
        let AppStateResumed { gl_context, renderer, gl_surface, window } = self;
        println!("Android window removed");
        drop(gl_surface);
        drop(window);
        let gl_context = gl_context.make_not_current()?;
        Ok(AppStateSuspended { gl_context, renderer })
    }

    fn resize(&self, width: NonZeroU32, height: NonZeroU32) {
        // Some platforms like EGL require resizing GL surface to update the
        // size Notable platforms here are
        // Wayland and macOS, other don't require it
        // and the function is no-op, but it's wise to resize it for
        // portability reasons.
        self.gl_surface.resize(&self.gl_context, width, height);

        self.renderer.resize(width.get() as i32, height.get() as i32);
    }

    fn redraw(&self) -> Result<(), Box<dyn Error>> {
        self.window.pre_present_notify();
        self.renderer.draw();
        self.window.request_redraw(); // TODO: Document why we need to request a redraw.
        self.gl_surface.swap_buffers(&self.gl_context)?;
        Ok(())
    }
}

struct AppStateSuspended {
    gl_context: NotCurrentContext,
    renderer: Renderer,
}

impl AppStateSuspended {
    fn resume(self, event_loop: &ActiveEventLoop) -> Result<AppStateResumed, Box<dyn Error>> {
        let AppStateSuspended { gl_context, renderer } = self;
        println!("Recreating window in `resumed`");
        // Pick the config which we already use for the context.

        let gl_config = gl_context.config();
        let window = glutin_winit::finalize_window(event_loop, window_attributes(), &gl_config)?;
        let gl_surface = create_gl_surface(&window, &gl_config)?;
        let gl_context = gl_context.make_current(&gl_surface)?;
        enable_vsync(&gl_surface, &gl_context);

        Ok(AppStateResumed { gl_context, renderer, gl_surface, window })
    }
}

// Find the config with the maximum number of samples, so our triangle will be
// smooth.
pub fn gl_config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs
        .reduce(|accum, config| {
            let transparency_check = config.supports_transparency().unwrap_or(false)
                & !accum.supports_transparency().unwrap_or(false);

            if transparency_check || config.num_samples() > accum.num_samples() {
                config
            } else {
                accum
            }
        })
        .unwrap()
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
        self.draw_with_clear_color(0.1, 0.1, 0.1, 0.9)
    }

    pub fn draw_with_clear_color(
        &self,
        red: GLfloat,
        green: GLfloat,
        blue: GLfloat,
        alpha: GLfloat,
    ) {
        unsafe {
            self.gl.UseProgram(self.program);

            self.gl.BindVertexArray(self.vao);
            self.gl.BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            self.gl.ClearColor(red, green, blue, alpha);
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
