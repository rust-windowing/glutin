use crate::api::egl::{self, NativeDisplay};
use crate::platform_impl::PlatformAttributes;
use crate::{
    ContextBuilderWrapper, ContextError, ContextSupports, CreationError,
    GlAttributes, PixelFormat, PixelFormatRequirements,
};

use crate::platform::unix::{
    EventLoopExtUnix, EventLoopWindowTargetExtUnix, WindowExtUnix,
};
use glutin_egl_sys as ffi;
use wayland_client::egl as wegl;
pub use wayland_client::sys::client::wl_display;
use winit;
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

use std::ops::Deref;
use std::os::raw;
use std::sync::Arc;

#[derive(Debug)]
pub struct Context {
    context: egl::Context,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WindowSurface {
    #[derivative(Debug = "ignore")]
    wsurface: wegl::WlEglSurface,
    surface: egl::WindowSurface,
}

impl WindowSurface {
    #[inline]
    pub fn new<T>(
        el: &EventLoopWindowTarget<T>,
        ctx: &Context,
        wb: WindowBuilder,
    ) -> Result<(Window, Self), CreationError> {
        let win = wb.build(el)?;

        let dpi_factor = win.hidpi_factor();
        let size = win.inner_size().to_physical(dpi_factor);
        let (width, height): (u32, u32) = size.into();

        let surface = win.wayland_surface();
        let surface = match surface {
            Some(s) => s,
            None => {
                return Err(CreationError::NotSupported(
                    "Wayland not found".to_string(),
                ));
            }
        };

        let wsurface = unsafe {
            wegl::WlEglSurface::new_from_raw(
                surface as *mut _,
                width as i32,
                height as i32,
            )
        };

        egl::WindowSurface::new_window_surface(
            el,
            &ctx.context,
            wsurface.ptr() as *const _,
        )
        .map(|surface| (win, WindowSurface { wsurface, surface }))
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        let (width, height): (u32, u32) = size.into();
        self.wsurface.resize(width as i32, height as i32, 0, 0)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.surface.swap_buffers()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.surface.get_pixel_format()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.surface.is_current()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        self.surface.make_not_current()
    }
}

#[derive(Debug)]
pub struct PBuffer {
    pbuffer: egl::PBuffer,
}

impl PBuffer {
    #[inline]
    pub fn new<T>(
        el: &EventLoopWindowTarget<T>,
        ctx: &Context,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        egl::PBuffer::new_pbuffer(el, &ctx.context, size)
            .map(|pbuffer| PBuffer { pbuffer })
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pbuffer.get_pixel_format()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.pbuffer.is_current()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        self.pbuffer.make_not_current()
    }
}

impl Context {
    #[inline]
    pub(crate) fn new<T>(
        el: &EventLoopWindowTarget<T>,
        cb: ContextBuilderWrapper<&Context>,
        ctx_supports: ContextSupports,
    ) -> Result<Self, CreationError> {
        let display_ptr = el.wayland_display().unwrap() as *const _;
        let context = {
            let cb = cb.map_sharing(|c| &c.context);
            let native_display =
                NativeDisplay::Wayland(Some(display_ptr as *const _));
            egl::Context::new(&cb, native_display, ctx_supports, |c, _| {
                Ok(c[0])
            })?
        };
        Ok(Context { context })
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), ContextError> {
        self.context.make_current_surfaceless()
    }

    #[inline]
    pub unsafe fn make_current_surface(
        &self,
        surface: &WindowSurface,
    ) -> Result<(), ContextError> {
        self.context.make_current_surface(&surface.surface)
    }

    #[inline]
    pub unsafe fn make_current_pbuffer(
        &self,
        pbuffer: &PBuffer,
    ) -> Result<(), ContextError> {
        self.context.make_current_pbuffer(&pbuffer.pbuffer)
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
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format()
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
}
