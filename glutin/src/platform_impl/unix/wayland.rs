use crate::api::egl::{
    Context as EglContext, NativeDisplay, SurfaceType as EglSurfaceType,
};
use crate::{
    ContextError, CreationError, GlAttributes, PixelFormat,
    PixelFormatRequirements, Rect,
};

use crate::platform::unix::{EventLoopWindowTargetExtUnix, WindowExtUnix};
use glutin_egl_sys as ffi;
pub use wayland_client::sys::client::wl_display;
use winit;
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

use std::ops::Deref;
use std::os::raw;
use std::sync::Arc;

pub struct EglSurface(Arc<wayland_egl::WlEglSurface>);

impl std::fmt::Debug for EglSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "EglSurface(...)")
    }
}

#[derive(Debug)]
pub enum Context {
    Windowed(EglContext, EglSurface),
    PBuffer(EglContext),
    Surfaceless(EglContext),
}

impl Deref for Context {
    type Target = EglContext;

    fn deref(&self) -> &Self::Target {
        match self {
            Context::Windowed(ctx, _) => ctx,
            Context::PBuffer(ctx) => ctx,
            Context::Surfaceless(ctx) => ctx,
        }
    }
}

impl Context {
    #[inline]
    pub fn new_headless<T>(
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: Option<dpi::PhysicalSize<u32>>,
    ) -> Result<Self, CreationError> {
        let gl_attr = gl_attr.clone().map_sharing(|c| &**c);
        let display_ptr = el.wayland_display().unwrap() as *const _;
        let native_display =
            NativeDisplay::Wayland(Some(display_ptr as *const _));
        if let Some(size) = size {
            let context = EglContext::new(
                pf_reqs,
                &gl_attr,
                native_display,
                EglSurfaceType::PBuffer,
                |c, _| Ok(c[0]),
            )
            .and_then(|p| p.finish_pbuffer(size))?;
            let context = Context::PBuffer(context);
            Ok(context)
        } else {
            // Surfaceless
            let context = EglContext::new(
                pf_reqs,
                &gl_attr,
                native_display,
                EglSurfaceType::Surfaceless,
                |c, _| Ok(c[0]),
            )
            .and_then(|p| p.finish_surfaceless())?;
            let context = Context::Surfaceless(context);
            Ok(context)
        }
    }

    #[inline]
    pub fn new<T>(
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(Window, Self), CreationError> {
        let win = wb.build(el)?;

        let size = win.inner_size();
        let (width, height): (u32, u32) = size.into();

        let display_ptr = win.wayland_display().unwrap() as *const _;
        let surface = win.wayland_surface();
        let surface = match surface {
            Some(s) => s,
            None => {
                return Err(CreationError::NotSupported(
                    "Wayland not found".to_string(),
                ));
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
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        let egl_surface = unsafe {
            wayland_egl::WlEglSurface::new_from_raw(
                surface as *mut _,
                width as i32,
                height as i32,
            )
        };
        let context = {
            let gl_attr = gl_attr.clone().map_sharing(|c| &**c);
            let native_display =
                NativeDisplay::Wayland(Some(display_ptr as *const _));
            EglContext::new(
                pf_reqs,
                &gl_attr,
                native_display,
                EglSurfaceType::Window,
                |c, _| Ok(c[0]),
            )
            .and_then(|p| p.finish(egl_surface.ptr() as *const _))?
        };
        let context =
            Context::Windowed(context, EglSurface(Arc::new(egl_surface)));
        Ok(context)
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        (**self).make_current()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        (**self).make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        (**self).is_current()
    }

    #[inline]
    pub fn get_api(&self) -> crate::Api {
        (**self).get_api()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EGLContext {
        (**self).raw_handle()
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        Some((**self).get_egl_display())
    }

    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        match self {
            Context::Windowed(_, surface) => {
                surface.0.resize(width as i32, height as i32, 0, 0)
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        (**self).get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        (**self).swap_buffers()
    }

    #[inline]
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        (**self).swap_buffers_with_damage(rects)
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        (**self).swap_buffers_with_damage_supported()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        (**self).get_pixel_format().clone()
    }
}
