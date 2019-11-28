#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod wayland;
// mod x11;

// use self::x11::X11Context;
use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::{ContextBuilderWrapper, ContextError};
use crate::platform::unix::x11::XConnection;
use crate::platform::unix::{EventLoopExtUnix, EventLoopWindowTargetExtUnix};
use crate::{
    CreationError, PixelFormat, PixelFormatRequirements,
    Rect,
};

// pub use self::x11::utils as x11_utils;

use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

use std::ffi::c_void;
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
    // X11,
    Wayland,
}

#[derive(Debug)]
pub enum Context {
    // X11(x11::Context),
    Wayland(wayland::Context),
}

#[derive(Debug)]
pub enum Config {
    // X11(x11::Config),
    Wayland(wayland::Config),
}

impl Config {
    #[inline]
    pub fn new(el: &Display, cb: ConfigBuilder) -> (ConfigAttribs, Config) {
        wayland::Config::new(el, cb)
            .map(|(attribs, config)| (attribs, Config::Wayland(config)))
    }
}

impl Context {
    fn is_compatible(
        c: &Option<&Context>,
        ct: ContextType,
    ) -> Result<(), CreationError> {
        if let Some(c) = *c {
            match ct {
                // ContextType::X11 => match *c {
                //    Context::X11(_) => Ok(()),
                //    _ => {
                //        let msg = "Cannot share an X11 context with a non-X11
                // context";        return
                // Err(CreationError::PlatformSpecific(
                //            msg.into(),
                //        ));
                //    }
                //},
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
    pub(crate) fn new(
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        supports_surfaceless: bool,
        conf: ConfigWrapper<&Config>,
    ) -> Result<Self, CreationError> {
        match disp.display {
            Display::Wayland(disp) => {
                Context::is_compatible(&cb.sharing, ContextType::Wayland)?;
                let cb = cb.map_sharing(|ctx| match *ctx {
                    Context::Wayland(ref ctx) => ctx,
                    _ => unreachable!(),
                });
                let conf = conf.map_config(|conf| match *conf {
                    Config::Wayland(ref ctx) => ctx,
                    _ => unreachable!(),
                });
                wayland::Context::new(disp, cb, supports_surfaceless, conf)
                    .map(|context| Context::Wayland(context))
            }
        }
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), ContextError> {
        match self {
            // Context::X11(ref ctx) => ctx.make_current_surfaceless(),
            Context::Wayland(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub unsafe fn make_current_surface(
        &self,
        surface: &WindowSurface,
    ) -> Result<(), ContextError> {
        match (self, surface) {
            (
                Context::Wayland(ref ctx),
                WindowSurface::Wayland(ref surface),
            ) => ctx.make_current_surface(surface),
        }
    }

    #[inline]
    pub unsafe fn make_current_pbuffer(
        &self,
        pbuffer: &PBuffer,
    ) -> Result<(), ContextError> {
        match (self, pbuffer) {
            (Context::Wayland(ref ctx), PBuffer::Wayland(ref pbuffer)) => {
                ctx.make_current_pbuffer(pbuffer)
            }
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match self {
            // Context::X11(ref ctx) => ctx.make_not_current(),
            Context::Wayland(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            // Context::X11(ref ctx) => ctx.is_current(),
            Context::Wayland(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self {
            // Context::X11(ref ctx) => ctx.get_api(),
            Context::Wayland(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match self {
            // Context::X11(ref ctx) => match *ctx.raw_handle() {
            //    X11Context::Glx(ref ctx) => RawHandle::Glx(ctx.raw_handle()),
            //    X11Context::Egl(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
            //},
            Context::Wayland(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match self {
            // Context::X11(ref ctx) => ctx.get_egl_display(),
            Context::Wayland(ref ctx) => ctx.get_egl_display(),
            _ => None,
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const c_void {
        match self {
            // Context::X11(ref ctx) => ctx.get_proc_address(addr),
            Context::Wayland(ref ctx) => ctx.get_proc_address(addr),
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
pub struct SurfacePlatformAttributes {
    /// X11 only: set internally to insure a certain visual xid is used when
    /// choosing the fbconfig.
    pub(crate) x11_visual_xid: Option<std::os::raw::c_ulong>,

    /// Ignored by surfaceless, which is always egl.
    pub backing_api: BackingApi,
}

#[derive(Default, Debug, Clone)]
pub struct ContextPlatformAttributes {
    /// GLX only: Whether the context will have transparency support.
    pub glx_transparency: Option<bool>,
}

#[derive(Debug)]
pub enum WindowSurface {
    // X11(x11::WindowSurface),
    Wayland(wayland::WindowSurface),
}

impl WindowSurface {
    #[inline]
    pub fn new(
        disp: &Display,
        conf: &ConfigWrapper<Config>,
        wb: WindowBuilder,
    ) -> Result<(Window, Self), CreationError> {
        match disp {
            Display::Wayland(ref disp) => wayland::WindowSurface::new(
                disp, conf.clone().map_config(|conf| match conf {
                    Config::Wayland(ref conf) => conf,
                    _ => panic!("Contradicting backend for config and display."),
                }), wb,
            )
            .map(|(win, surf)| (win, WindowSurface::Wayland(surf))),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match self {
            // WindowSurface::X11(surface) => surface.get_pixel_format(),
            WindowSurface::Wayland(surface) => surface.get_pixel_format(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            // WindowSurface::X11(surface) => surface.is_current(),
            WindowSurface::Wayland(surface) => surface.is_current(),
        }
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        match self {
            WindowSurface::Wayland(ref surface) => {
                surface.update_after_resize(size)
            }
            _ => (),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match self {
            // WindowSurface::X11(ref surface) => surface.swap_buffers(),
            WindowSurface::Wayland(ref surface) => surface.swap_buffers(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        match self {
            // WindowSurface::X11(ref surface) =>
            // surface.swap_buffers_with_damage(rects),
            WindowSurface::Wayland(ref surface) => {
                surface.swap_buffers_with_damage(rects)
            }
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match self {
            // WindowSurface::X11(ref surface) => surface.make_not_current(),
            WindowSurface::Wayland(ref surface) => surface.make_not_current(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config> {
        match self {
            // WindowSurface::X11(ref surface) => surface.get_config(),
            WindowSurface::Wayland(ref surface) => surface.get_config().map_config(|conf| Config::Wayland(conf)),
        }
    }
}

#[derive(Debug)]
pub enum PBuffer {
    // X11(x11::PBuffer),
    Wayland(wayland::PBuffer),
}

impl PBuffer {
    #[inline]
    pub fn new(
        disp: &Display,
        conf: &ConfigWrapper<Config>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        match disp {
            Display::Wayland(ref disp) => wayland::PBuffer::new(
                disp, conf.clone().map_config(|conf| match conf {
                    Config::Wayland(ref conf) => conf,
                    _ => panic!("Contradicting backend for config and display."),
                }), size,
            )
            .map(|(win, surf)| (win, PBuffer::Wayland(surf))),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match self {
            // PBuffer::X11(pbuffer) => pbuffer.get_pixel_format(),
            PBuffer::Wayland(pbuffer) => pbuffer.get_pixel_format(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            // PBuffer::X11(pbuffer) => pbuffer.is_current(),
            PBuffer::Wayland(pbuffer) => pbuffer.is_current(),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match self {
            // PBuffer::X11(ref pbuffer) => pbuffer.make_not_current(),
            PBuffer::Wayland(ref pbuffer) => pbuffer.make_not_current(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config> {
        match self {
            // PBuffer::X11(ref pbuffer) => pbuffer.get_config(),
            PBuffer::Wayland(ref pbuffer) => pbuffer.get_config().map_config(|conf| Config::Wayland(conf)),
        }
    }
}

#[derive(Debug)]
pub enum Display {
    // X11(x11::Display),
    Wayland(wayland::Display),
}

impl Display {
    pub fn new<TE>(
        el: &EventLoopWindowTarget<TE>,
    ) -> Result<Self, CreationError> {
        if el.is_wayland() {
            wayland::Display::new(el).map(|disp| Display::Wayland(disp))
        } else {
            unimplemented!()
            // Context::is_compatible(&cb.gl_attr.sharing, ContextType::X11)?;
            // let cb = cb.map_sharing(|ctx| match *ctx {
            //    Context::X11(ref ctx) => ctx,
            //    _ => unreachable!(),
            //});
            // x11::Context::new(
            //    el,
            //    cb,
            //    supports,
            //)
            //.map(|context| Context::X11(context))
        }
    }
}
