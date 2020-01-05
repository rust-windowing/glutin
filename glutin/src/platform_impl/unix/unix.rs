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
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};
pub use crate::platform::unix::ConfigPlatformAttributes;

use glutin_interface::{
    NativeDisplay, NativePixmap, NativePixmapBuilder, NativeWindow, NativeWindowBuilder, RawDisplay,
};
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::marker::PhantomData;
use std::os::raw;
use std::sync::Arc;

#[derive(Debug)]
pub enum Config {
    X11(x11::Config),
    Wayland(wayland::Config),
}

impl Config {
    #[inline]
    pub fn new<ND: NativeDisplay>(
        cb: &ConfigBuilder,
        nd: &ND,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        Ok(match nd.display() {
            RawDisplay::Wayland { .. } => {
                let configs = wayland::Config::new(cb, nd)?;
                configs
                    .into_iter()
                    .map(|(attribs, config)| (attribs, Config::Wayland(config)))
                    .collect()
            }
            RawDisplay::Xlib { .. } => {
                let configs = x11::Config::new(cb, nd)?;
                configs
                    .into_iter()
                    .map(|(attribs, config)| (attribs, Config::X11(config)))
                    .collect()
            }
            // FIXME: GBM/EGLExtDevice/EGLMesaSurfaceles backends.
            _ => unimplemented!(),
        })
    }
}

#[derive(Debug)]
pub enum Context {
    X11(x11::Context),
    Wayland(wayland::Context),
}

impl Context {
    fn inner_cb_wayland(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&wayland::Context>, Error> {
        match cb.sharing {
            Some(Context::Wayland(_)) | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a Wayland context with a non-Wayland context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context::Wayland(ctx) => ctx,
            _ => unreachable!(),
        }))
    }

    fn inner_cb_x11(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&x11::Context>, Error> {
        match cb.sharing {
            Some(Context::X11(_)) | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a X11 context with a non-X11 context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context::X11(ctx) => ctx,
            _ => unreachable!(),
        }))
    }

    #[inline]
    pub(crate) fn new(
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Wayland(config) => {
                wayland::Context::new(Context::inner_cb_wayland(cb)?, conf.map_config(|_| config))
                    .map(Context::Wayland)
            }
            Config::X11(config) => {
                x11::Context::new(Context::inner_cb_x11(cb)?, conf.map_config(|_| config))
                    .map(Context::X11)
            }
        }
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        match self {
            Context::Wayland(ref ctx) => ctx.make_current_surfaceless(),
            Context::X11(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        match (self, surf) {
            (Context::Wayland(ref ctx), Surface::Wayland(ref surf)) => ctx.make_current(surf),
            (Context::X11(ref ctx), Surface::X11(ref surf)) => ctx.make_current(surf),
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Context::Wayland(ref ctx) => ctx.make_not_current(),
            Context::X11(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Context::Wayland(ref ctx) => ctx.is_current(),
            Context::X11(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self {
            Context::Wayland(ref ctx) => ctx.get_api(),
            Context::X11(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const raw::c_void {
        match self {
            Context::Wayland(ref ctx) => ctx.get_proc_address(addr),
            Context::X11(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Context::Wayland(ref ctx) => ctx.get_config().map_config(Config::Wayland),
            Context::X11(ref ctx) => ctx.get_config().map_config(Config::X11),
        }
    }
}

#[derive(Debug)]
pub enum Surface<T: SurfaceTypeTrait> {
    X11(x11::Surface<T>),
    Wayland(wayland::Surface<T>),
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Surface::Wayland(ref surf) => surf.is_current(),
            Surface::X11(ref surf) => surf.is_current(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Surface::Wayland(ref surf) => surf.get_config().map_config(Config::Wayland),
            Surface::X11(ref surf) => surf.get_config().map_config(Config::X11),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Surface::Wayland(ref surf) => surf.make_not_current(),
            Surface::X11(ref surf) => surf.make_not_current(),
        }
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Wayland(config) => {
                wayland::Surface::<PBuffer>::new(conf.map_config(|_| config), size)
                    .map(Surface::Wayland)
            }
            Config::X11(config) => {
                x11::Surface::<PBuffer>::new(conf.map_config(|_| config), size).map(Surface::X11)
            }
        }
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new<NPB: NativePixmapBuilder>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        npb: NPB,
    ) -> Result<(NPB::Pixmap, Self), Error> {
        match conf.config {
            Config::Wayland(config) => {
                wayland::Surface::<Pixmap>::new(conf.map_config(|_| config), npb)
                    .map(|(pix, surf)| (pix, Surface::Wayland(surf)))
            }
            Config::X11(config) => x11::Surface::<Pixmap>::new(conf.map_config(|_| config), npb)
                .map(|(pix, surf)| (pix, Surface::X11(surf))),
        }
    }

    #[inline]
    pub unsafe fn new_existing<NP: NativePixmap>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        np: &NP,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Wayland(config) => {
                wayland::Surface::<Pixmap>::new_existing(conf.map_config(|_| config), np)
                    .map(Surface::Wayland)
            }
            Config::X11(config) => {
                x11::Surface::<Pixmap>::new_existing(conf.map_config(|_| config), np)
                    .map(Surface::X11)
            }
        }
    }
}

impl Surface<Window> {
    #[inline]
    pub unsafe fn new<NWB: NativeWindowBuilder>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nwb: NWB,
    ) -> Result<(NWB::Window, Self), Error> {
        match conf.config {
            Config::Wayland(config) => {
                wayland::Surface::<Window>::new(conf.map_config(|_| config), nwb)
                    .map(|(win, surf)| (win, Surface::Wayland(surf)))
            }
            Config::X11(config) => x11::Surface::<Window>::new(conf.map_config(|_| config), nwb)
                .map(|(win, surf)| (win, Surface::X11(surf))),
        }
    }

    #[inline]
    pub unsafe fn new_existing<NW: NativeWindow>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Wayland(config) => {
                wayland::Surface::<Window>::new_existing(conf.map_config(|_| config), nw)
                    .map(Surface::Wayland)
            }
            Config::X11(config) => {
                x11::Surface::<Window>::new_existing(conf.map_config(|_| config), nw)
                    .map(Surface::X11)
            }
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        match self {
            Surface::Wayland(ref surf) => surf.swap_buffers(),
            Surface::X11(ref surf) => surf.swap_buffers(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        match self {
            Surface::Wayland(ref surf) => surf.swap_buffers_with_damage(rects),
            Surface::X11(ref surf) => surf.swap_buffers_with_damage(rects),
        }
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        match self {
            Surface::Wayland(ref surf) => surf.update_after_resize(size),
            Surface::X11(_) => (),
        }
    }
}
