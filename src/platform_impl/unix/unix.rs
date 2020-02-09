#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod generic_egl;
pub mod x11;

use crate::config::{ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapInterval};
use crate::context::ContextBuilderWrapper;
pub use crate::platform::unix::ConfigPlatformAttributes;
use crate::platform::unix::{RawConfig, RawContext, RawDisplay as GlutinRawDisplay, RawSurface};
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};

use glutin_interface::{
    NativeDisplay, NativePixmap, NativePixmapSource, NativeWindow, NativeWindowSource, RawDisplay,
};
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};

use std::os::raw;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Config {
    X11(x11::Config),
    GenericEgl(generic_egl::Config),
}

impl Config {
    #[inline]
    pub fn new<ND: NativeDisplay>(
        cf: &ConfigsFinder,
        nd: &ND,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        Ok(match nd.raw_display() {
            RawDisplay::Wayland { .. }
            | RawDisplay::EglMesaSurfaceless { .. }
            | RawDisplay::EglExtDevice { .. }
            | RawDisplay::Gbm { .. } => {
                let configs = generic_egl::Config::new(cf, nd)?;
                configs
                    .into_iter()
                    .map(|(attribs, config)| (attribs, Config::GenericEgl(config)))
                    .collect()
            }
            RawDisplay::Xlib { .. } => {
                let configs = x11::Config::new(cf, nd)?;
                configs
                    .into_iter()
                    .map(|(attribs, config)| (attribs, Config::X11(config)))
                    .collect()
            }
            // FIXME: GBM/EGLExtDevice backends.
            _ => unimplemented!(),
        })
    }

    #[inline]
    pub fn raw_config(&self) -> RawConfig {
        match self {
            Config::GenericEgl(ref conf) => conf.raw_config(),
            Config::X11(ref conf) => conf.raw_config(),
        }
    }

    #[inline]
    pub fn raw_display(&self) -> GlutinRawDisplay {
        match self {
            Config::GenericEgl(ref conf) => conf.raw_display(),
            Config::X11(ref conf) => conf.raw_display(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Context {
    X11(x11::Context),
    GenericEgl(generic_egl::Context),
}

impl Context {
    #[inline]
    fn inner_cb_generic_egl(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&generic_egl::Context>, Error> {
        match cb.sharing {
            Some(Context::GenericEgl(_)) | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a GenericEgl context with a non-GenericEgl context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context::GenericEgl(ctx) => ctx,
            _ => unreachable!(),
        }))
    }

    #[inline]
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
            Config::GenericEgl(config) => generic_egl::Context::new(
                Context::inner_cb_generic_egl(cb)?,
                conf.map_config(|_| config),
            )
            .map(Context::GenericEgl),
            Config::X11(config) => {
                x11::Context::new(Context::inner_cb_x11(cb)?, conf.map_config(|_| config))
                    .map(Context::X11)
            }
        }
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        match self {
            Context::GenericEgl(ref ctx) => ctx.make_current_surfaceless(),
            Context::X11(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub(crate) unsafe fn make_current<T: SurfaceTypeTrait>(
        &self,
        surf: &Surface<T>,
    ) -> Result<(), Error> {
        match (self, surf) {
            (Context::GenericEgl(ref ctx), Surface::GenericEgl(ref surf)) => ctx.make_current(surf),
            (Context::X11(ref ctx), Surface::X11(ref surf)) => ctx.make_current(surf),
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub(crate) unsafe fn make_current_rw<TR: SurfaceTypeTrait, TW: SurfaceTypeTrait>(
        &self,
        read_surf: &Surface<TR>,
        write_surf: &Surface<TW>,
    ) -> Result<(), Error> {
        match (self, read_surf, write_surf) {
            (
                Context::GenericEgl(ref ctx),
                Surface::GenericEgl(ref read_surf),
                Surface::GenericEgl(ref write_surf),
            ) => ctx.make_current_rw(read_surf, write_surf),
            (Context::X11(ref ctx), Surface::X11(ref read_surf), Surface::X11(ref write_surf)) => {
                ctx.make_current_rw(read_surf, write_surf)
            }
            (_, _, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Context::GenericEgl(ref ctx) => ctx.make_not_current(),
            Context::X11(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Context::GenericEgl(ref ctx) => ctx.is_current(),
            Context::X11(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn raw_context(&self) -> RawContext {
        match self {
            Context::GenericEgl(ref ctx) => ctx.raw_context(),
            Context::X11(ref ctx) => ctx.raw_context(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        match self {
            Context::GenericEgl(ref ctx) => ctx.get_proc_address(addr),
            Context::X11(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Context::GenericEgl(ref ctx) => ctx.get_config().map_config(Config::GenericEgl),
            Context::X11(ref ctx) => ctx.get_config().map_config(Config::X11),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Surface<T: SurfaceTypeTrait> {
    X11(x11::Surface<T>),
    GenericEgl(generic_egl::Surface<T>),
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Surface::GenericEgl(ref surf) => surf.is_current(),
            Surface::X11(ref surf) => surf.is_current(),
        }
    }

    #[inline]
    pub fn raw_surface(&self) -> RawSurface {
        match self {
            Surface::GenericEgl(ref surf) => surf.raw_surface(),
            Surface::X11(ref surf) => surf.raw_surface(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Surface::GenericEgl(ref surf) => surf.get_config().map_config(Config::GenericEgl),
            Surface::X11(ref surf) => surf.get_config().map_config(Config::X11),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Surface::GenericEgl(ref surf) => surf.make_not_current(),
            Surface::X11(ref surf) => surf.make_not_current(),
        }
    }

    #[inline]
    pub fn size(&self) -> Result<dpi::PhysicalSize<u32>, Error> {
        match self {
            Surface::GenericEgl(ref surf) => surf.size(),
            Surface::X11(ref surf) => surf.size(),
        }
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize<u32>,
        largest: bool,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::GenericEgl(config) => {
                generic_egl::Surface::<PBuffer>::new(conf.map_config(|_| config), size, largest)
                    .map(Surface::GenericEgl)
            }
            Config::X11(config) => {
                x11::Surface::<PBuffer>::new(conf.map_config(|_| config), size, largest)
                    .map(Surface::X11)
            }
        }
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn build_pixmap<NPS: NativePixmapSource>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nps: &NPS,
        pb: NPS::PixmapBuilder,
    ) -> Result<NPS::Pixmap, Error> {
        match conf.config {
            Config::GenericEgl(config) => {
                generic_egl::Surface::<Pixmap>::build_pixmap(conf.map_config(|_| config), nps, pb)
            }
            Config::X11(config) => {
                x11::Surface::<Pixmap>::build_pixmap(conf.map_config(|_| config), nps, pb)
            }
        }
    }

    #[inline]
    pub unsafe fn new_existing<NP: NativePixmap>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        np: &NP,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::GenericEgl(config) => {
                generic_egl::Surface::<Pixmap>::new_existing(conf.map_config(|_| config), np)
                    .map(Surface::GenericEgl)
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
    pub unsafe fn build_window<NWS: NativeWindowSource>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nws: &NWS,
        wb: NWS::WindowBuilder,
    ) -> Result<NWS::Window, Error> {
        match conf.config {
            Config::GenericEgl(config) => {
                generic_egl::Surface::<Window>::build_window(conf.map_config(|_| config), nws, wb)
            }
            Config::X11(config) => {
                x11::Surface::<Window>::build_window(conf.map_config(|_| config), nws, wb)
            }
        }
    }

    #[inline]
    pub unsafe fn new_existing<NW: NativeWindow>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::GenericEgl(config) => {
                generic_egl::Surface::<Window>::new_existing(conf.map_config(|_| config), nw)
                    .map(Surface::GenericEgl)
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
            Surface::GenericEgl(ref surf) => surf.swap_buffers(),
            Surface::X11(ref surf) => surf.swap_buffers(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        match self {
            Surface::GenericEgl(ref surf) => surf.swap_buffers_with_damage(rects),
            Surface::X11(ref surf) => surf.swap_buffers_with_damage(rects),
        }
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize<u32>) {
        match self {
            Surface::GenericEgl(ref surf) => surf.update_after_resize(size),
            Surface::X11(_) => (),
        }
    }

    #[inline]
    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        match self {
            Surface::GenericEgl(ref surf) => surf.modify_swap_interval(swap_interval),
            Surface::X11(ref surf) => surf.modify_swap_interval(swap_interval),
        }
    }
}
