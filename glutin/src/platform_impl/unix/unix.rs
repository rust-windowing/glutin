#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod wayland;
mod x11;

use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::ContextBuilderWrapper;
use crate::display::DisplayBuilder;
use crate::surface::{PBuffer, Pixmap, Rect, SurfaceTypeTrait, Window};

use glutin_interface::{
    NativeDisplay, NativePixmap, NativePixmapBuilder, NativeWindow, NativeWindowBuilder, RawDisplay,
};
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::ffi::c_void;
use std::marker::PhantomData;
use std::os::raw;
use std::sync::Arc;

/// Context handles available on Unix-like platforms.
#[derive(Clone, Debug)]
pub enum RawHandle {
    /// Context handle for a glx context.
    Glx(glutin_glx_sys::glx::types::GLXContext),
    /// Context handle for a egl context.
    Egl(glutin_egl_sys::EGLContext),
}

#[derive(Debug)]
pub enum ContextType {
    // X11,
    Wayland,
}

#[derive(Debug)]
pub enum Display {
    X11(x11::Display),
    Wayland(wayland::Display),
}

impl Display {
    pub fn new<ND: NativeDisplay>(db: DisplayBuilder, nd: &ND) -> Result<Self, Error> {
        match nd.display() {
            RawDisplay::Wayland { .. } => wayland::Display::new(db, nd).map(Display::Wayland),
            RawDisplay::Xlib { .. } => x11::Display::new(db, nd).map(Display::X11),
            _ => unimplemented!(),
        }
    }

    fn inner_wayland(disp: &Display) -> &wayland::Display {
        match disp {
            Display::Wayland(disp) => disp,
            _ => unreachable!(),
        }
    }

    fn inner_x11(disp: &Display) -> &x11::Display {
        match disp {
            Display::X11(disp) => disp,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum Config {
    // X11(x11::Config),
    Wayland(wayland::Config),
}

impl Config {
    #[inline]
    pub fn new(disp: &Display, cb: ConfigBuilder) -> Result<(ConfigAttribs, Config), Error> {
        wayland::Config::new(Display::inner_wayland(disp), cb)
            .map(|(attribs, config)| (attribs, Config::Wayland(config)))
    }

    fn inner_wayland<'a, 'b>(
        conf: ConfigWrapper<&'a Config, &'b ConfigAttribs>,
    ) -> ConfigWrapper<&'a wayland::Config, &'b ConfigAttribs> {
        conf.map_config(|conf| match conf {
            Config::Wayland(conf) => conf,
            _ => unreachable!(),
        })
    }
}

#[derive(Debug)]
pub enum Context {
    // X11(x11::Context),
    Wayland(wayland::Context),
}

impl Context {
    fn inner_cb_wayland(
        cb: ContextBuilderWrapper<&Context>,
    ) -> ContextBuilderWrapper<&wayland::Context> {
        cb.map_sharing(|ctx| match ctx {
            Context::Wayland(ctx) => ctx,
            _ => unreachable!(),
        })
    }

    fn is_compatible(c: &Option<&Context>, ct: ContextType) -> Result<(), Error> {
        if let Some(c) = *c {
            match ct {
                ContextType::Wayland => match *c {
                    Context::Wayland(_) => Ok(()),
                    _ => {
                        return Err(make_error!(ErrorType::BadApiUsage(
                            "Cannot share a Wayland context with a non-Wayland context".to_string()
                        )));
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
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        wayland::Context::new(
            Display::inner_wayland(disp),
            Context::inner_cb_wayland(cb),
            Config::inner_wayland(conf),
        )
        .map(Context::Wayland)
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        match self {
            Context::Wayland(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        match (self, surf) {
            (Context::Wayland(ref ctx), Surface::Wayland(ref surf)) => ctx.make_current(surf),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Context::Wayland(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Context::Wayland(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self {
            Context::Wayland(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match self {
            Context::Wayland(ref ctx) => ctx.raw_handle(),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match self {
            Context::Wayland(ref ctx) => ctx.get_egl_display(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const c_void {
        match self {
            Context::Wayland(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Context::Wayland(ref ctx) => ctx.get_config().map_config(Config::Wayland),
        }
    }
}

#[derive(Debug)]
pub enum Surface<T: SurfaceTypeTrait> {
    // X11(x11::Surface<T>),
    Wayland(wayland::Surface<T>),
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Surface::Wayland(ref surf) => surf.is_current(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Surface::Wayland(ref surf) => surf.get_config().map_config(Config::Wayland),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Surface::Wayland(ref surf) => surf.make_not_current(),
        }
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        match disp {
            Display::Wayland(_) => wayland::Surface::<PBuffer>::new(
                Display::inner_wayland(disp),
                Config::inner_wayland(conf),
                size,
            )
            .map(Surface::Wayland),
        }
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new<NPB: NativePixmapBuilder>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        npb: NPB,
    ) -> Result<(NPB::Pixmap, Self), Error> {
        match disp {
            Display::Wayland(_) => wayland::Surface::<Pixmap>::new(
                Display::inner_wayland(disp),
                Config::inner_wayland(conf),
                npb,
            )
            .map(|(pix, surf)| (pix, Surface::Wayland(surf))),
        }
    }

    #[inline]
    pub unsafe fn new_existing<NP: NativePixmap>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        np: &NP,
    ) -> Result<Self, Error> {
        match disp {
            Display::Wayland(_) => wayland::Surface::<Pixmap>::new_existing(
                Display::inner_wayland(disp),
                Config::inner_wayland(conf),
                np,
            )
            .map(Surface::Wayland),
        }
    }
}

impl Surface<Window> {
    #[inline]
    pub unsafe fn new<NWB: NativeWindowBuilder>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nwb: NWB,
    ) -> Result<(NWB::Window, Self), Error> {
        match disp {
            Display::Wayland(_) => wayland::Surface::<Window>::new(
                Display::inner_wayland(disp),
                Config::inner_wayland(conf),
                nwb,
            )
            .map(|(win, surf)| (win, Surface::Wayland(surf))),
        }
    }

    #[inline]
    pub unsafe fn new_existing<NW: NativeWindow>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        match disp {
            Display::Wayland(_) => wayland::Surface::<Window>::new_existing(
                Display::inner_wayland(disp),
                Config::inner_wayland(conf),
                nw,
            )
            .map(Surface::Wayland),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        match self {
            Surface::Wayland(ref surf) => surf.swap_buffers(),
        }
    }

    pub fn swap_buffers_with_damage(&self, rects: &[Rect]) -> Result<(), Error> {
        match self {
            Surface::Wayland(ref surf) => surf.swap_buffers_with_damage(rects),
        }
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        match self {
            Surface::Wayland(ref surf) => surf.update_after_resize(size),
        }
    }
}

// FIXME: Add SurfaceBuilder type
#[derive(Default, Debug, Clone)]
pub struct SurfacePlatformAttributes {
    /// X11 only: set internally to insure a certain visual xid is used when
    /// choosing the fbconfig.
    pub(crate) x11_visual_xid: Option<std::os::raw::c_ulong>,
}

#[derive(Default, Debug, Clone)]
pub struct ContextPlatformAttributes {
    /// GLX only: Whether the context will have transparency support.
    pub glx_transparency: Option<bool>,
}

#[derive(Default, Debug, Clone)]
pub struct DisplayPlatformAttributes {
    pub backing_api: BackingApi,
}

#[derive(Debug, Clone)]
pub enum BackingApi {
    GLXThenEGL,
    EGLThenGLX,
    EGL,
    GLX,
}

impl Default for BackingApi {
    fn default() -> Self {
        BackingApi::GLXThenEGL
    }
}
