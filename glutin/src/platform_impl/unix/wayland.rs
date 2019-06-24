use crate::api::egl::{
    Context as EglContext, NativeDisplay, SurfaceType as EglSurfaceType,
};
use crate::platform_impl::PlatformAttributes;
use crate::{
    ContextBuilderWrapper, ContextError, CreationError, GlAttributes,
    PixelFormat, PixelFormatRequirements,
};

use crate::platform::unix::{EventLoopExtUnix, WindowExtUnix};
use glutin_egl_sys as ffi;
use wayland_client::egl as wegl;
pub use wayland_client::sys::client::wl_display;
use winit;
use winit::dpi;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use std::ops::Deref;
use std::os::raw;
use std::sync::Arc;

#[derive(Debug)]
pub struct Context {
    context: EglContext,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WindowSurface {
    #[derivative(Debug = "ignore")]
    surface: Arc<wegl::WlEglSurface>,
}

impl WindowSurface {
    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        let (width, height): (u32, u32) = size.into();
        self.surface.resize(width as i32, height as i32, 0, 0)
    }
}

#[derive(Debug)]
pub struct PBuffer {}

impl Context {
    // #[inline]
    // pub fn new_headless<T>(
    // el: &EventLoop<T>,
    // cb: ContextBuilderWrapper<&Context>,
    // size: Option<dpi::PhysicalSize>,
    // ) -> Result<Self, CreationError> {
    // let cb = cb.map_sharing(|c| &c.context);
    // let display_ptr = el.wayland_display().unwrap() as *const _;
    // let native_display =
    // NativeDisplay::Wayland(Some(display_ptr as *const _));
    // if let Some(size) = size {
    // let context = EglContext::new(
    // &cb,
    // native_display,
    // EglSurfaceType::PBuffer,
    // |c, _| Ok(c[0]),
    // )
    // .and_then(|p| p.finish_pbuffer(size))?;
    // let context = Context::PBuffer(context);
    // Ok(context)
    // } else {
    // Surfaceless
    // let context = EglContext::new(
    // &cb,
    // native_display,
    // EglSurfaceType::Surfaceless,
    // |c, _| Ok(c[0]),
    // )
    // .and_then(|p| p.finish_surfaceless())?;
    // let context = Context::Surfaceless(context);
    // Ok(context)
    // }
    // }

    #[inline]
    pub fn new<T>(
        // wb: WindowBuilder,
        el: &EventLoop<T>,
        cb: ContextBuilderWrapper<&Context>,
        pbuffer_support: bool,
        window_surface_support: bool,
        surfaceless_support: bool,
    ) -> Result<Self, CreationError> {
        let win = wb.build(el)?;

        let dpi_factor = win.hidpi_factor();
        let size = win.inner_size().to_physical(dpi_factor);
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

        let context =
            Self::new_raw_context(display_ptr, surface, width, height, cb)?;
        Ok((win, context))
    }

    #[inline]
    pub fn new_raw_context(
        display_ptr: *const wl_display,
        surface: *mut raw::c_void,
        width: u32,
        height: u32,
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<Self, CreationError> {
        // let egl_surface = unsafe {
        // wegl::WlEglSurface::new_from_raw(
        // surface as *mut _,
        // width as i32,
        // height as i32,
        // )
        // };
        // let context = {
        // let cb = cb.map_sharing(|c| &c.context);
        // let native_display =
        // NativeDisplay::Wayland(Some(display_ptr as *const _));
        // EglContext::new(
        // &cb,
        // native_display,
        // EglSurfaceType::Window,
        // |c, _| Ok(c[0]),
        // )
        // .and_then(|p| p.finish(egl_surface.ptr() as *const _))?
        // };
        // let context =
        // Context::Windowed(context, EglSurface(Arc::new(egl_surface)));
        // Ok(context)
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        self.context.make_not_current()
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
