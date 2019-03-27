use crate::api::egl::{Context as EglContext, NativeDisplay};
use crate::{
    ContextCurrentState, ContextError, CreationError, GlAttributes,
    NotCurrentContext, PixelFormat, PixelFormatRequirements,
    PossiblyCurrentContext,
};

use glutin_egl_sys as ffi;
use wayland_client::egl as wegl;
pub use wayland_client::sys::client::wl_display;
use winit;
use winit::os::unix::WindowExt;

use std::os::raw;
use std::sync::Arc;

#[derive(DebugStub)]
pub struct Context<T: ContextCurrentState> {
    #[debug_stub = "Arc<wegl::WlEglSurface>"]
    egl_surface: Arc<wegl::WlEglSurface>,
    context: EglContext<T>,
}

impl<T: ContextCurrentState> Context<T> {
    #[inline]
    pub fn new(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context<T>>,
    ) -> Result<(winit::Window, Context<NotCurrentContext>), CreationError>
    {
        let win = wb.build(el)?;

        let dpi_factor = win.get_hidpi_factor();
        let size = win.get_inner_size().unwrap().to_physical(dpi_factor);
        let (width, height): (u32, u32) = size.into();

        let display_ptr = win.get_wayland_display().unwrap() as *const _;
        let surface = win.get_wayland_surface();
        let surface = match surface {
            Some(s) => s,
            None => {
                return Err(CreationError::NotSupported("Wayland not found"));
            }
        };

        let context = Self::new_raw_context(
            display_ptr,
            surface,
            width,
            height,
            pf_reqs,
            gl_attr,
        )?;
        Ok((win, context))
    }

    #[inline]
    pub fn new_raw_context(
        display_ptr: *const wl_display,
        surface: *mut raw::c_void,
        width: u32,
        height: u32,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context<T>>,
    ) -> Result<Context<NotCurrentContext>, CreationError> {
        let egl_surface = unsafe {
            wegl::WlEglSurface::new_from_raw(
                surface as *mut _,
                width as i32,
                height as i32,
            )
        };
        let context = {
            let gl_attr = gl_attr.clone().map_sharing(|c| &c.context);
            let native_display =
                NativeDisplay::Wayland(Some(display_ptr as *const _));
            EglContext::new(pf_reqs, &gl_attr, native_display)
                .and_then(|p| p.finish(egl_surface.ptr() as *const _))?
        };
        let context = Context {
            egl_surface: Arc::new(egl_surface),
            context,
        };
        Ok(context)
    }

    #[inline]
    pub unsafe fn make_current(
        self,
    ) -> Result<Context<PossiblyCurrentContext>, (Self, ContextError)> {
        let egl_surface = self.egl_surface;
        match self.context.make_current() {
            Ok(context) => Ok(Context {
                context,
                egl_surface,
            }),
            Err((context, err)) => Err((
                Context {
                    context,
                    egl_surface,
                },
                err,
            )),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(
        self,
    ) -> Result<Context<NotCurrentContext>, (Self, ContextError)> {
        let egl_surface = self.egl_surface;
        match self.context.make_not_current() {
            Ok(context) => Ok(Context {
                context,
                egl_surface,
            }),
            Err((context, err)) => Err((
                Context {
                    context,
                    egl_surface,
                },
                err,
            )),
        }
    }

    #[inline]
    pub unsafe fn treat_as_not_current(self) -> Context<NotCurrentContext> {
        Context {
            egl_surface: self.egl_surface,
            context: self.context.treat_as_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.context.is_current()
    }

    #[inline]
    pub fn get_api(&self) -> crate::Api {
        self.context.get_api()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EGLContext {
        self.context.raw_handle()
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        Some(self.context.get_egl_display())
    }
}

impl Context<PossiblyCurrentContext> {
    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        self.egl_surface.resize(width as i32, height as i32, 0, 0);
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.context.swap_buffers()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format().clone()
    }
}
