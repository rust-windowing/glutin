use std::error::Error;
use std::num::NonZeroU32;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::error::{Error as GlutinError, ErrorKind};
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};
use glutin_examples::gl::types::GLfloat;
use glutin_examples::{gl_config_picker, Renderer};
use glutin_winit::{self, DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::Window;

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<PlatformThreadEvent>::with_user_event().build().unwrap();

    // `EventLoopProxy` allows you to dispatch custom events to the main Winit event
    // loop from any thread.
    let event_loop_proxy = event_loop.create_proxy();

    let mut app = App::new(event_loop_proxy);

    event_loop.run_app(&mut app)?;

    app.exit_state
}

struct App {
    event_loop_proxy: EventLoopProxy<PlatformThreadEvent>,
    exit_state: Result<(), Box<dyn Error>>,
    state: Option<AppState>,
}

impl App {
    fn new(event_loop_proxy: EventLoopProxy<PlatformThreadEvent>) -> Self {
        Self { event_loop_proxy, exit_state: Ok(()), state: None }
    }
}

impl ApplicationHandler<PlatformThreadEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let (window, render_context) = match create_window_with_render_context(event_loop) {
            Ok(ok) => ok,
            Err(e) => {
                self.exit_state = Err(e);
                event_loop.exit();
                return;
            },
        };
        let render_context = Arc::new(Mutex::new(render_context));

        let (_render_threads, render_thread_senders) =
            spawn_render_threads(render_context, self.event_loop_proxy.clone());

        let state = AppState {
            _window: window,
            render_thread_senders,
            render_thread_index: 0,
            thread_switch_in_progress: false,
        };
        state.send_event_to_current_render_thread(RenderThreadEvent::MakeCurrent);
        assert!(self.state.replace(state).is_none());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                self.state.as_ref().unwrap().send_event_to_current_render_thread(
                    RenderThreadEvent::Resize(PhysicalSize {
                        width: NonZeroU32::new(size.width).unwrap(),
                        height: NonZeroU32::new(size.height).unwrap(),
                    }),
                );
            },
            WindowEvent::RedrawRequested => {
                self.state
                    .as_ref()
                    .unwrap()
                    .send_event_to_current_render_thread(RenderThreadEvent::Draw);
            },
            WindowEvent::MouseInput { state: ElementState::Pressed, .. } => {
                self.state.as_mut().unwrap().start_render_thread_switch();
            },
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: PlatformThreadEvent) {
        self.state.as_mut().unwrap().complete_render_thread_switch();
    }
}

struct AppState {
    render_thread_senders: Vec<Sender<RenderThreadEvent>>,
    render_thread_index: usize,
    thread_switch_in_progress: bool,
    _window: Window,
}

impl AppState {
    fn send_event_to_current_render_thread(&self, event: RenderThreadEvent) {
        if self.thread_switch_in_progress {
            return;
        }

        if let Some(tx) = self.render_thread_senders.get(self.render_thread_index) {
            tx.send(event).expect("sending event to current render thread failed");
        }
    }

    fn start_render_thread_switch(&mut self) {
        self.send_event_to_current_render_thread(RenderThreadEvent::MakeNotCurrent);

        self.thread_switch_in_progress = true;
    }

    fn complete_render_thread_switch(&mut self) {
        self.thread_switch_in_progress = false;

        self.render_thread_index += 1;
        if self.render_thread_index == self.render_thread_senders.len() {
            self.render_thread_index = 0;
        }

        self.send_event_to_current_render_thread(RenderThreadEvent::MakeCurrent);
        self.send_event_to_current_render_thread(RenderThreadEvent::Draw);
    }
}

/// A rendering context that can be shared between tasks.
struct RenderContext {
    context: Option<PossiblyCurrentContext>,
    surface: Surface<WindowSurface>,
    renderer: Renderer,
}

unsafe impl Send for RenderContext {}

impl RenderContext {
    fn new(
        context: PossiblyCurrentContext,
        surface: Surface<WindowSurface>,
        renderer: Renderer,
    ) -> Self {
        Self { context: Some(context), surface, renderer }
    }

    fn make_current(&mut self) -> Result<(), impl Error> {
        let ctx =
            self.context.take().ok_or_else(|| GlutinError::from(ErrorKind::BadContextState))?;
        let result = ctx.make_current(&self.surface);
        self.context = Some(ctx);
        result
    }

    fn make_not_current(&mut self) -> Result<(), impl Error> {
        let ctx =
            self.context.take().ok_or_else(|| GlutinError::from(ErrorKind::BadContextState))?;
        let not_current_ctx = ctx.make_not_current()?;
        self.context = Some(not_current_ctx.treat_as_possibly_current());
        Ok::<(), GlutinError>(())
    }

    fn swap_buffers(&mut self) -> Result<(), impl Error> {
        let ctx =
            self.context.take().ok_or_else(|| GlutinError::from(ErrorKind::BadContextState))?;
        let result = self.surface.swap_buffers(&ctx);
        self.context = Some(ctx);
        result
    }

    fn draw_with_clear_color(&self, red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
        self.renderer.draw_with_clear_color(red, green, blue, alpha)
    }

    fn resize(&mut self, size: PhysicalSize<NonZeroU32>) {
        let Some(ctx) = self.context.take() else {
            return;
        };
        self.surface.resize(&ctx, size.width, size.height);
        self.context = Some(ctx);

        self.renderer.resize(size.width.get() as i32, size.height.get() as i32);
    }
}

fn create_window_with_render_context(
    event_loop: &ActiveEventLoop,
) -> Result<(Window, RenderContext), Box<dyn Error>> {
    let window_attributes = Window::default_attributes().with_transparent(true);

    let template = ConfigTemplateBuilder::new().with_alpha_size(8);

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

    let (mut window, gl_config) = display_builder.build(event_loop, template, gl_config_picker)?;

    println!("Picked a config with {} samples", gl_config.num_samples());

    let raw_window_handle = window
        .as_ref()
        .and_then(|window| window.window_handle().map(|handle| handle.as_raw()).ok());

    let window = window.take().unwrap();

    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

    let not_current_gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .expect("failed to create context")
    };

    let attrs = window
        .build_surface_attributes(<_>::default())
        .expect("Failed to build surface attributes");
    let gl_surface =
        unsafe { gl_config.display().create_window_surface(&gl_config, &attrs).unwrap() };

    // Make it current.
    let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

    // The context needs to be current for the Renderer to set up shaders and
    // buffers. It also performs function loading, which needs a current context on
    // WGL.
    let renderer = Renderer::new(&gl_display);

    let gl_context = gl_context.make_not_current().unwrap().treat_as_possibly_current();

    Ok((window, RenderContext::new(gl_context, gl_surface, renderer)))
}

fn spawn_render_threads(
    render_context: Arc<Mutex<RenderContext>>,
    event_loop_proxy: EventLoopProxy<PlatformThreadEvent>,
) -> (Vec<RenderThread>, Vec<Sender<RenderThreadEvent>>) {
    let mut senders = Vec::new();
    let mut render_threads = Vec::new();

    for id in 0..3 {
        let render_thread = RenderThread::new(id, render_context.clone());
        let tx = render_thread.spawn(event_loop_proxy.clone());

        render_threads.push(render_thread);
        senders.push(tx);
    }

    (render_threads, senders)
}

#[derive(Debug, Clone, Copy, Default)]
struct Color {
    r: GLfloat,
    g: GLfloat,
    b: GLfloat,
    a: GLfloat,
}

impl Color {
    fn new(r: GLfloat, g: GLfloat, b: GLfloat, a: GLfloat) -> Self {
        Self { r, g, b, a }
    }

    fn new_from_index(color_index: i32) -> Self {
        match color_index {
            0 => Color::new(1.0, 0.0, 0.0, 0.9),
            1 => Color::new(0.0, 1.0, 0.0, 0.9),
            2 => Color::new(0.0, 0.0, 1.0, 0.9),
            _ => Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum RenderThreadEvent {
    Draw,
    MakeCurrent,
    MakeNotCurrent,
    Resize(PhysicalSize<NonZeroU32>),
}

#[derive(Debug, Clone, Copy)]
enum PlatformThreadEvent {
    ContextNotCurrent,
}

struct RenderThread {
    id: i32,
    color: Color,
    render_context: Arc<Mutex<RenderContext>>,
}

impl RenderThread {
    fn new(id: i32, render_context: Arc<Mutex<RenderContext>>) -> Self {
        let color = Color::new_from_index(id);
        Self { id, color, render_context }
    }

    fn spawn(
        &self,
        event_loop_proxy: EventLoopProxy<PlatformThreadEvent>,
    ) -> Sender<RenderThreadEvent> {
        let (tx, rx) = mpsc::channel();

        let (id, color, render_context) = (self.id, self.color, self.render_context.clone());

        thread::spawn(move || {
            for event in rx {
                let mut render_context_guard = render_context.lock().unwrap();

                match event {
                    RenderThreadEvent::Draw => {
                        println!("thread {}: drawing", id);
                        render_context_guard
                            .draw_with_clear_color(color.r, color.g, color.b, color.a);
                        render_context_guard.swap_buffers().expect("swap buffers failed");
                    },
                    RenderThreadEvent::MakeCurrent => {
                        println!("thread {}: make current", id);
                        render_context_guard.make_current().expect("make current failed");
                    },
                    RenderThreadEvent::MakeNotCurrent => {
                        println!("thread {}: make not current", id);
                        render_context_guard.make_not_current().expect("make not current failed");
                        event_loop_proxy
                            .send_event(PlatformThreadEvent::ContextNotCurrent)
                            .expect("sending context-not-current event failed");
                    },
                    RenderThreadEvent::Resize(size) => {
                        render_context_guard.resize(size);
                    },
                }
            }
        });

        tx
    }
}
