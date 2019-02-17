use api::egl::{self, ffi, Context as EglContext};
use std::os::raw;
use std::sync::Arc;
use wayland_client::egl as wegl;
use winit;
use winit::os::unix::WindowExt;
use {
    ContextError, CreationError, GlAttributes, PixelFormat,
    PixelFormatRequirements,
};

pub struct Context {
    egl_surface: Arc<wegl::WlEglSurface>,
    context: EglContext,
}

impl Context {
    #[inline]
    pub fn new(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError> {
        let window = wb.build(el)?;
        let context = Self::new_separated(&window, el, pf_reqs, gl_attr)?;
        Ok((window, context))
    }

    #[inline]
    pub fn new_separated(
        window: &winit::Window,
        _el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        let logical_size = window.get_inner_size().unwrap();
        let (w, h) = (logical_size.width, logical_size.height);
        let surface = window.get_wayland_surface();
        let surface = match surface {
            Some(s) => s,
            None => {
                return Err(CreationError::NotSupported("Wayland not found"));
            }
        };
        let egl_surface = unsafe {
            wegl::WlEglSurface::new_from_raw(
                surface as *mut _,
                w as i32,
                h as i32,
            )
        };
        let context = {
            let gl_attr = gl_attr.clone().map_sharing(|c| &c.context);
            let native_display = egl::NativeDisplay::Wayland(Some(
                window.get_wayland_display().unwrap() as *const _,
            ));
            EglContext::new(pf_reqs, &gl_attr, native_display)
                .and_then(|p| p.finish(egl_surface.ptr() as *const _))?
        };
        let context = Context {
            egl_surface: Arc::new(egl_surface),
            context: context,
        };
        Ok(context)
    }

    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        self.egl_surface.resize(width as i32, height as i32, 0, 0);
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.context.is_current()
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
    pub fn get_api(&self) -> ::Api {
        self.context.get_api()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format().clone()
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
