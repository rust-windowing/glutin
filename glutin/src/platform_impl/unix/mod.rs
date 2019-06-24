#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod wayland;
mod x11;

use self::x11::X11Context;
use crate::{
    Api, ContextBuilderWrapper, ContextCurrentState, ContextError,
    CreationError, GlAttributes, NotCurrent, PixelFormat,
    PixelFormatRequirements,
};
pub use x11::utils as x11_utils;

use crate::platform::unix::x11::XConnection;
use crate::platform::unix::EventLoopExtUnix;
use winit::dpi;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use std::marker::PhantomData;
use std::os::raw;
use std::sync::Arc;

/// Context handles available on Unix-like platforms.
#[derive(Clone, Debug)]
pub enum RawHandle {
    /// Context handle for a glx context.
    Glx(glutin_glx_sys::GLXContext),
    /// Context handle for a egl context.
    Egl(glutin_egl_sys::EGLContext),
}

#[derive(Debug)]
pub enum ContextType {
    X11,
    Wayland,
}

#[derive(Debug)]
pub enum Context {
    X11(x11::Context),
    Wayland(wayland::Context),
}

impl Context {
    fn is_compatible(
        c: &Option<&Context>,
        ct: ContextType,
    ) -> Result<(), CreationError> {
        if let Some(c) = *c {
            match ct {
                ContextType::X11 => match *c {
                    Context::X11(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share an X11 context with a non-X11 context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
                ContextType::Wayland => match *c {
                    Context::Wayland(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share a Wayland context with a non-Wayland context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn new<T>(
        el: &EventLoop<T>,
        cb: ContextBuilderWrapper<&Context>,
        pbuffer_support: bool,
        window_surface_support: bool,
        surfaceless_support: bool,
    ) -> Result<Self, CreationError> {
        if el.is_wayland() {
            Context::is_compatible(&cb.gl_attr.sharing, ContextType::Wayland)?;
            let cb = cb.clone().map_sharing(|ctx| match *ctx {
                Context::Wayland(ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new(
                el,
                cb,
                pbuffer_support,
                window_surface_support,
                surfaceless_support,
            )
            .map(|context| Context::Wayland(context))
        } else {
            Context::is_compatible(&cb.gl_attr.sharing, ContextType::X11)?;
            let cb = cb.map_sharing(|ctx| match *ctx {
                Context::X11(ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new(
                el,
                cb,
                pbuffer_support,
                window_surface_support,
                surfaceless_support,
            )
            .map(|context| Context::X11(context))
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.make_current(),
            Context::Wayland(ref ctx) => ctx.make_current(),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.make_not_current(),
            Context::Wayland(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::X11(ref ctx) => ctx.is_current(),
            Context::Wayland(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match *self {
            Context::X11(ref ctx) => ctx.get_api(),
            Context::Wayland(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            Context::X11(ref ctx) => match *ctx.raw_handle() {
                X11Context::Glx(ref ctx) => RawHandle::Glx(ctx.raw_handle()),
                X11Context::Egl(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
            },
            Context::Wayland(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match *self {
            Context::X11(ref ctx) => ctx.get_egl_display(),
            Context::Wayland(ref ctx) => ctx.get_egl_display(),
            _ => None,
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::X11(ref ctx) => ctx.get_proc_address(addr),
            Context::Wayland(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.swap_buffers(),
            Context::Wayland(ref ctx) => ctx.swap_buffers(),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BackingApi {
    GlxThenEgl,
    EglThenGlx,
    Egl,
    Glx,
}

impl Default for BackingApi {
    fn default() -> Self {
        BackingApi::GlxThenEgl
    }
}

#[derive(Default, Debug, Clone)]
pub struct PlatformAttributes {
    /// X11 only: set internally to insure a certain visual xid is used when
    /// choosing the fbconfig.
    pub(crate) x11_visual_xid: Option<std::os::raw::c_ulong>,

    /// GLX only: Whether the context will have transparency support.
    pub glx_transparency: Option<bool>,

    /// Ignored by surfaceless, which is always egl.
    pub backing_api: BackingApi,
}

#[derive(Debug)]
pub enum WindowSurface {
    X11(x11::WindowSurface),
    Wayland(wayland::WindowSurface),
}

impl WindowSurface {
    #[inline]
    pub fn new<T>(
        el: &EventLoop<T>,
        ctx: &Context,
        wb: WindowBuilder,
    ) -> Result<(Self, Window), CreationError> {
        match ctx {
            Context::X11(ref ctx) => x11::WindowSurface::new(el, ctx, wb)
                .map(|ws| WindowSurface::X11(ws)),
            Context::Wayland(ref ctx) => {
                wayland::WindowSurface::new(el, ctx, wb)
                    .map(|ws| WindowSurface::Wayland(ws))
            }
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match self {
            WindowSurface::X11(ws) => ws.get_pixel_format(),
            WindowSurface::Wayland(ws) => ws.get_pixel_format(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            WindowSurface::X11(ws) => ws.is_current(),
            WindowSurface::Wayland(ws) => ws.is_current(),
        }
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        match self {
            Context::Wayland(ref ctx) => ctx.update_after_resize(size),
            _ => (),
        }
    }
}

#[derive(Debug)]
pub enum PBuffer {
    X11(x11::PBuffer),
    Wayland(wayland::PBuffer),
}

impl PBuffer {
    #[inline]
    pub fn new<T>(
        el: &EventLoop<T>,
        ctx: &Context,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        match ctx {
            Context::X11(ref ctx) => {
                x11::PBuffer::new(el, ctx, size).map(|pb| PBuffer::X11(pb))
            }
            Context::Wayland(ref ctx) => wayland::PBuffer::new(el, ctx, size)
                .map(|pb| PBuffer::Wayland(pb)),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match self {
            PBuffer::X11(pb) => pb.get_pixel_format(),
            PBuffer::Wayland(pb) => pb.get_pixel_format(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            PBuffer::X11(pb) => pb.is_current(),
            PBuffer::Wayland(pb) => pb.is_current(),
        }
    }
}
