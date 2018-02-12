#![cfg(target_os = "android")]

extern crate android_glue;

use libc;

use CreationError::{self, OsError};

use winit;

use Api;
use ContextError;
use GlAttributes;
use PixelFormat;
use PixelFormatRequirements;

use api::egl;
use api::egl::Context as EglContext;
use std::cell::Cell;
use std::sync::Arc;
use winit::os::android::EventsLoopExt;

mod ffi;

struct AndroidContext {
    egl_context: EglContext,
    stopped: Cell<bool>,
}

pub struct Context(Arc<AndroidContext>);

struct AndroidSyncEventHandler(Arc<AndroidContext>);

impl android_glue::SyncEventHandler for AndroidSyncEventHandler {
    fn handle(&mut self, event: &android_glue::Event) {
        match *event {
            // 'on_surface_destroyed' Android event can arrive with some delay because multithreading communication.
            // Because of that, swap_buffers can be called before processing 'on_surface_destroyed' event, with the
            // native window surface already destroyed. EGL generates a BAD_SURFACE error in this situation.
            // Set stop to true to prevent swap_buffer call race conditions.
            android_glue::Event::TermWindow => {
                self.0.stopped.set(true);
            },
            _ => { return; }
        };
    }
}

impl Context {
    pub fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<(winit::Window, Self), CreationError>
    {
        let window = try!(window_builder.build(events_loop));
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0.egl_context);
        let native_window = unsafe { android_glue::get_native_window() };
        if native_window.is_null() {
            return Err(OsError(format!("Android's native window is null")));
        }
        let egl = egl::ffi::egl::Egl;
        let native_display = egl::NativeDisplay::Android;
        let context = try!(EglContext::new(egl, pf_reqs, &gl_attr, native_display)
            .and_then(|p| p.finish(native_window as *const _)));
        let ctx = Arc::new(AndroidContext {
            egl_context: context,
            stopped: Cell::new(false),
        });

        let handler = Box::new(AndroidSyncEventHandler(ctx.clone()));
        android_glue::add_sync_event_handler(handler);
        let context = Context(ctx.clone());

        events_loop.set_suspend_callback(Some(Box::new(move |suspended| {
            ctx.stopped.set(suspended);
            if suspended {
                // Android has stopped the activity or sent it to background.
                // Release the EGL surface and stop the animation loop.
                unsafe {
                    ctx.egl_context.on_surface_destroyed();
                }
            } else {
                // Android has started the activity or sent it to foreground.
                // Restore the EGL surface and animation loop.
                unsafe {
                    let native_window = android_glue::get_native_window();
                    ctx.egl_context.on_surface_created(native_window as *const _);
                }
            }
        })));

        Ok((window, context))
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        if !self.0.stopped.get() {
            return self.0.egl_context.make_current();
        }
        Err(ContextError::ContextLost)
    }

    #[inline]
    pub fn resize(&self, _: u32, _: u32) {
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.egl_context.is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.0.egl_context.get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        if !self.0.stopped.get() {
            return self.0.egl_context.swap_buffers();
        }
        Err(ContextError::ContextLost)
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.0.egl_context.get_api()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.0.egl_context.get_pixel_format()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> egl::ffi::EGLContext {
        self.0.egl_context.raw_handle()
    }
}

#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

pub struct HeadlessContext(EglContext);

unsafe impl Send for HeadlessContext {}
unsafe impl Sync for HeadlessContext {}

impl HeadlessContext {
    /// See the docs in the crate root file.
    pub fn new(
        dimensions: (u32, u32),
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&HeadlessContext>,
        _: &PlatformSpecificHeadlessBuilderAttributes,
    ) -> Result<Self, CreationError>
    {
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0);
        let context = try!(EglContext::new(egl::ffi::egl::Egl,
                                           pf_reqs,
                                           &gl_attr,
                                           egl::NativeDisplay::Android));
        let context = try!(context.finish_pbuffer(dimensions));     // TODO:
        Ok(HeadlessContext(context))
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.0.make_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.0.get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.0.swap_buffers()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.0.get_api()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.0.get_pixel_format()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> egl::ffi::EGLContext {
        self.0.raw_handle()
    }
}
