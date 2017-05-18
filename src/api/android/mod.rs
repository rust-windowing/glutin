#![cfg(target_os = "android")]

extern crate android_glue;

use libc;

use CreationError::{self, OsError};

use winit;

use Api;
use ContextError;
use GlAttributes;
use GlContext;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;

use api::egl;
use api::egl::Context as EglContext;

mod ffi;

pub struct Window {
    context: EglContext,
    winit_window: winit::Window,
}

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;

#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

impl Window {
    pub fn new(events_loop: &winit::EventsLoop,
               _: &WindowAttributes,
               pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&Window>,
               _: &PlatformSpecificWindowBuilderAttributes,
               winit_builder: winit::WindowBuilder)
               -> Result<Window, CreationError> {
        let winit_window = winit_builder.build(events_loop).unwrap();
        let opengl = opengl.clone().map_sharing(|w| &w.context);
        let native_window = unsafe { android_glue::get_native_window() };
        if native_window.is_null() {
            return Err(OsError(format!("Android's native window is null")));
        }
        let context = try!(EglContext::new(egl::ffi::egl::Egl,
                                           pf_reqs,
                                           &opengl,
                                           egl::NativeDisplay::Android)
            .and_then(|p| p.finish(native_window as *const _)));
        Ok(Window {
            context: context,
            winit_window: winit_window,
        })
    }

    pub fn id(&self) -> winit::WindowId {
        self.winit_window.id()
    }

    pub fn set_title(&self, title: &str) {
        self.winit_window.set_title(title)
    }

    pub fn show(&self) {
        self.winit_window.show()
    }

    pub fn hide(&self) {
        self.winit_window.hide()
    }

    pub fn get_position(&self) -> Option<(i32, i32)> {
        self.winit_window.get_position()
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.winit_window.set_position(x, y)
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_points(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size()
    }

    pub fn get_inner_size_pixels(&self) -> Option<(u32, u32)> {
        self.winit_window.get_inner_size().map(|(x, y)| {
            let hidpi = self.hidpi_factor();
            ((x as f32 * hidpi) as u32, (y as f32 * hidpi) as u32)
        })
    }

    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        self.winit_window.get_outer_size()
    }

    pub fn set_inner_size(&self, x: u32, y: u32) {
        self.winit_window.set_inner_size(x, y)
    }

    pub unsafe fn platform_display(&self) -> *mut libc::c_void {
        self.winit_window.platform_display()
    }

    #[inline]
    pub fn as_winit_window(&self) -> &winit::Window {
        &self.winit_window
    }
 
    #[inline]
    pub fn as_winit_window_mut(&mut self) -> &mut winit::Window {
        &mut self.winit_window
    }

    pub unsafe fn platform_window(&self) -> *mut libc::c_void {
        self.winit_window.platform_window()
    }

    pub fn set_cursor(&self, cursor: winit::MouseCursor) {
        self.winit_window.set_cursor(cursor);
    }

    pub fn hidpi_factor(&self) -> f32 {
        self.winit_window.hidpi_factor()
    }

    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        self.winit_window.set_cursor_position(x, y)
    }

    pub fn set_cursor_state(&self, state: winit::CursorState) -> Result<(), String> {
        self.winit_window.set_cursor_state(state)
    }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    #[inline]
    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        self.context.swap_buffers()
    }

    #[inline]
    fn get_api(&self) -> Api {
        self.context.get_api()
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format()
    }
}

pub struct HeadlessContext(EglContext);

impl HeadlessContext {
    /// See the docs in the crate root file.
    pub fn new(dimensions: (u32, u32),
               pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&HeadlessContext>,
               _: &PlatformSpecificHeadlessBuilderAttributes)
               -> Result<HeadlessContext, CreationError> {
        let opengl = opengl.clone().map_sharing(|c| &c.0);
        let context = try!(EglContext::new(egl::ffi::egl::Egl,
                                           pf_reqs,
                                           &opengl,
                                           egl::NativeDisplay::Android));
        let context = try!(context.finish_pbuffer(dimensions));     // TODO:
        Ok(HeadlessContext(context))
    }
}

unsafe impl Send for HeadlessContext {}
unsafe impl Sync for HeadlessContext {}

impl GlContext for HeadlessContext {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.0.make_current()
    }

    #[inline]
    fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        self.0.get_proc_address(addr)
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        self.0.swap_buffers()
    }

    #[inline]
    fn get_api(&self) -> Api {
        self.0.get_api()
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.0.get_pixel_format()
    }
}
