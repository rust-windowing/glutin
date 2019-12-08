#![cfg(target_os = "android")]

use crate::api::egl::{
    Context as EglContext, NativeDisplay, SurfaceType as EglSurfaceType,
};
use crate::CreationError;
use crate::{
    Api, ContextError, GlAttributes, PixelFormat, PixelFormatRequirements, Rect,
};

use crate::platform::android::EventLoopWindowTargetExtAndroid;
use android_ndk::android_app::AndroidApp;
use glutin_egl_sys as ffi;
use parking_lot::Mutex;
use winit;
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

use std::sync::Arc;

#[derive(Debug)]
struct AndroidContext {
    egl_context: EglContext,
    stopped: Option<Mutex<bool>>,
}

#[derive(Debug)]
pub struct Context(Arc<AndroidContext>);

impl Context {
    #[inline]
    pub fn new_windowed<T>(
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<(winit::window::Window, Self), CreationError> {
        let win = wb.build(el)?;
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0.egl_context);
        let native_display = NativeDisplay::Android;
        let egl_context = EglContext::new(
            pf_reqs,
            &gl_attr,
            native_display,
            EglSurfaceType::Window,
            |c, _| Ok(c[0]),
        )
        .and_then(|p| p.finish())?;
        let ctx = Arc::new(AndroidContext {
            egl_context,
            stopped: Some(Mutex::new(false)),
        });

        let context = Context(ctx.clone());

        el.set_suspend_callback(Some(Box::new(move |suspended| {
            let mut stopped = ctx.stopped.as_ref().unwrap().lock();
            *stopped = suspended;
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
                    let android_app =
                        AndroidApp::from_ptr(android_glue::get_android_app());
                    let native_window = android_app.native_window().unwrap();
                    let ptr =
                        native_window.ptr().as_ref() as *const _ as *const _;
                    ctx.egl_context.on_surface_created(ptr);
                }
            }
        })));

        let android_app =
            unsafe { AndroidApp::from_ptr(android_glue::get_android_app()) };
        if let Some(native_window) = android_app.native_window() {
            unsafe {
                let ptr = native_window.ptr().as_ref() as *const _ as *const _;
                context.0.egl_context.on_surface_created(ptr);
            }
        }

        Ok((win, context))
    }

    #[inline]
    pub fn new_headless<T>(
        _el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0.egl_context);
        let context = EglContext::new(
            pf_reqs,
            &gl_attr,
            NativeDisplay::Android,
            EglSurfaceType::PBuffer,
            |c, _| Ok(c[0]),
        )?;
        let egl_context = context.finish_pbuffer(size)?;
        let ctx = Arc::new(AndroidContext {
            egl_context,
            stopped: None,
        });
        Ok(Context(ctx))
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            let stopped = stopped.lock();
            if *stopped {
                return Err(ContextError::ContextLost);
            }
        }

        self.0.egl_context.make_current()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            let stopped = stopped.lock();
            if *stopped {
                return Err(ContextError::ContextLost);
            }
        }

        self.0.egl_context.make_not_current()
    }

    #[inline]
    pub fn resize(&self, _: u32, _: u32) {}

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.egl_context.is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        self.0.egl_context.get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            let stopped = stopped.lock();
            if *stopped {
                return Err(ContextError::ContextLost);
            }
        }
        self.0.egl_context.swap_buffers()
    }

    #[inline]
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            let stopped = stopped.lock();
            if *stopped {
                return Err(ContextError::ContextLost);
            }
        }
        self.0.egl_context.swap_buffers_with_damage(rects)
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        self.0.egl_context.swap_buffers_with_damage_supported()
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
