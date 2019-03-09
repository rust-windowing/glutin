#![cfg(target_os = "android")]

use crate::api::egl::{Context as EglContext, NativeDisplay};
use crate::CreationError::{self, OsError};
use crate::{
    Api, ContextError, GlAttributes, PixelFormat, PixelFormatRequirements,
};

use glutin_egl_sys as ffi;
use libc;
use winit;
use winit::dpi;
use winit::os::android::EventsLoopExt;

use std::cell::Cell;
use std::sync::Arc;

struct AndroidContext {
    egl_context: EglContext,
    stopped: Option<Cell<bool>>,
}

pub struct Context(Arc<AndroidContext>);

struct AndroidSyncEventHandler(Arc<AndroidContext>);

impl android_glue::SyncEventHandler for AndroidSyncEventHandler {
    fn handle(&mut self, event: &android_glue::Event) {
        match *event {
            // 'on_surface_destroyed' Android event can arrive with some delay
            // because multithreading communication. Because of
            // that, swap_buffers can be called before processing
            // 'on_surface_destroyed' event, with the native window
            // surface already destroyed. EGL generates a BAD_SURFACE error in
            // this situation. Set stop to true to prevent
            // swap_buffer call race conditions.
            android_glue::Event::TermWindow => {
                self.0.stopped.as_ref().unwrap().set(true);
            }
            _ => {
                return;
            }
        };
    }
}

impl Context {
    #[inline]
    pub fn new_windowed(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<(winit::Window, Self), CreationError> {
        let win = wb.build(el)?;
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0.egl_context);
        let nwin = unsafe { android_glue::get_native_window() };
        if nwin.is_null() {
            return Err(OsError(format!("Android's native window is null")));
        }
        let native_display = NativeDisplay::Android;
        let egl_context = EglContext::new(pf_reqs, &gl_attr, native_display)
            .and_then(|p| p.finish(nwin as *const _))?;
        let ctx = Arc::new(AndroidContext {
            egl_context,
            stopped: Some(Cell::new(false)),
        });

        let handler = Box::new(AndroidSyncEventHandler(ctx.clone()));
        android_glue::add_sync_event_handler(handler);
        let context = Context(ctx.clone());

        el.set_suspend_callback(Some(Box::new(move |suspended| {
            ctx.stopped.as_ref().unwrap().set(suspended);
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
                    let nwin = android_glue::get_native_window();
                    ctx.egl_context.on_surface_created(nwin as *const _);
                }
            }
        })));

        Ok((win, context))
    }

    #[inline]
    pub fn new_headless(
        _el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        dims: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0.egl_context);
        let context =
            EglContext::new(pf_reqs, &gl_attr, NativeDisplay::Android)?;
        let egl_context = context.finish_pbuffer(dims)?;
        let ctx = Arc::new(AndroidContext {
            egl_context,
            stopped: None,
        });
        Ok(Context(ctx))
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            if stopped.get() {
                return Err(ContextError::ContextLost);
            }
        }

        self.0.egl_context.make_current()
    }

    #[inline]
    pub fn resize(&self, _: u32, _: u32) {}

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
        if let Some(ref stopped) = self.0.stopped {
            if stopped.get() {
                return Err(ContextError::ContextLost);
            }
        }
        self.0.egl_context.swap_buffers()
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
    pub unsafe fn raw_handle(&self) -> ffi::EGLContext {
        self.0.egl_context.raw_handle()
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> ffi::EGLDisplay {
        self.0.egl_context.get_egl_display()
    }
}
