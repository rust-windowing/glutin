//! Support module for the glutin examples.
#![allow(dead_code)]
#![allow(unused_variables)]

use std::num::NonZeroU32;

use raw_window_handle::{HasRawWindowHandle, RawDisplayHandle, RawWindowHandle};

use winit::event_loop::EventLoop;
#[cfg(glx_backend)]
use winit::platform::unix;
use winit::window::{Window, WindowBuilder};

use glutin::config::{Config, ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
use glutin::display::{Display, DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributes, SurfaceAttributesBuilder, WindowSurface};

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
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
    ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true)
        .compatible_with_native_window(raw_window_handle)
        .with_surface_type(ConfigSurfaceTypes::WINDOW)
        .build()
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
