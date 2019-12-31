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
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};

use glutin_interface::inputs::{
    NativeDisplay, NativePixmap, NativePixmapBuilder, NativeWindow, NativeWindowBuilder, RawDisplay,
};
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::marker::PhantomData;
use std::os::raw;
use std::sync::Arc;

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
            // FIXME: GBM/EGLExtDevice/EGLMesaSurfaceles backends.
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum Config {
    X11(x11::Config),
    Wayland(wayland::Config),
}

impl Config {
    #[inline]
    pub fn new(disp: &Display, cb: ConfigBuilder) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        Ok(match disp {
            Display::Wayland(disp) => {
                let configs = wayland::Config::new(disp, cb)?;
                configs
                    .into_iter()
                    .map(|(attribs, config)| (attribs, Config::Wayland(config)))
                    .collect()
            }
            Display::X11(disp) => {
                let configs = x11::Config::new(disp, cb)?;
                configs
                    .into_iter()
                    .map(|(attribs, config)| (attribs, Config::X11(config)))
                    .collect()
            }
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
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        match (disp, conf.config) {
            (Display::Wayland(disp), Config::Wayland(config)) => wayland::Context::new(
                disp,
                Context::inner_cb_wayland(cb)?,
                conf.map_config(|_| config),
            )
            .map(Context::Wayland),
            (Display::X11(disp), Config::X11(config)) => x11::Context::new(
                disp,
                Context::inner_cb_x11(cb)?,
                conf.map_config(|_| config),
            )
            .map(Context::X11),
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
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
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        match (disp, conf.config) {
            (Display::Wayland(disp), Config::Wayland(config)) => {
                wayland::Surface::<PBuffer>::new(disp, conf.map_config(|_| config), size)
                    .map(Surface::Wayland)
            }
            (Display::X11(disp), Config::X11(config)) => {
                x11::Surface::<PBuffer>::new(disp, conf.map_config(|_| config), size)
                    .map(Surface::X11)
            }
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
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
        match (disp, conf.config) {
            (Display::Wayland(disp), Config::Wayland(config)) => {
                wayland::Surface::<Pixmap>::new(disp, conf.map_config(|_| config), npb)
                    .map(|(pix, surf)| (pix, Surface::Wayland(surf)))
            }
            (Display::X11(disp), Config::X11(config)) => {
                x11::Surface::<Pixmap>::new(disp, conf.map_config(|_| config), npb)
                    .map(|(pix, surf)| (pix, Surface::X11(surf)))
            }
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn new_existing<NP: NativePixmap>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        np: &NP,
    ) -> Result<Self, Error> {
        match (disp, conf.config) {
            (Display::Wayland(disp), Config::Wayland(config)) => {
                wayland::Surface::<Pixmap>::new_existing(disp, conf.map_config(|_| config), np)
                    .map(Surface::Wayland)
            }
            (Display::X11(disp), Config::X11(config)) => {
                x11::Surface::<Pixmap>::new_existing(disp, conf.map_config(|_| config), np)
                    .map(Surface::X11)
            }
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
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
        match (disp, conf.config) {
            (Display::Wayland(disp), Config::Wayland(config)) => {
                wayland::Surface::<Window>::new(disp, conf.map_config(|_| config), nwb)
                    .map(|(win, surf)| (win, Surface::Wayland(surf)))
            }
            (Display::X11(disp), Config::X11(config)) => {
                x11::Surface::<Window>::new(disp, conf.map_config(|_| config), nwb)
                    .map(|(win, surf)| (win, Surface::X11(surf)))
            }
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn new_existing<NW: NativeWindow>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        match (disp, conf.config) {
            (Display::Wayland(disp), Config::Wayland(config)) => {
                wayland::Surface::<Window>::new_existing(disp, conf.map_config(|_| config), nw)
                    .map(Surface::Wayland)
            }
            (Display::X11(disp), Config::X11(config)) => {
                x11::Surface::<Window>::new_existing(disp, conf.map_config(|_| config), nw)
                    .map(Surface::X11)
            }
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
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

#[derive(Default, Debug, Clone)]
pub struct ConfigPlatformAttributes {
    /// X11 only: set to insure a certain visual xid is used when
    /// choosing the fbconfig.
    pub x11_visual_xid: Option<raw::c_ulong>,

    /// Whether the X11 Visual will have transparency support.
    pub x11_transparency: Option<bool>,
}

#[derive(Default, Debug, Clone)]
pub struct ContextPlatformAttributes {}

#[derive(Default, Debug, Clone)]
pub struct DisplayPlatformAttributes {
    /// Wayland/X11 only.
    pub backing_api: BackingApi,
}

#[derive(Debug, Clone, Copy)]
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
