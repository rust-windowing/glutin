use crate::api::egl;
use crate::config::{ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapInterval};
use crate::context::ContextBuilderWrapper;
use crate::platform::unix::BackingApi;
use crate::platform::unix::{RawConfig, RawContext, RawDisplay as GlutinRawDisplay, RawSurface};
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};
use crate::utils::{NoCmp, NoPrint};

use glutin_interface::{
    GbmWindowParts, NativeDisplay, NativePixmap, NativePixmapSource, NativeWindow,
    NativeWindowSource, RawDisplay, RawWindow, Seal, WaylandWindowParts,
};
use wayland_client::egl as wegl;
pub use wayland_client::sys::client::wl_display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};

use std::ops::Deref;
use std::os::raw;

#[derive(Debug, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    EglMesaSurfaceless,
    Gbm,
    EglExtDevice,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Config {
    Wayland(egl::Config),
    EglMesaSurfaceless(egl::Config),
    Gbm(egl::Config),
    EglExtDevice(egl::Config),
}

impl Deref for Config {
    type Target = egl::Config;

    fn deref(&self) -> &Self::Target {
        match self {
            Config::Wayland(conf) => conf,
            Config::EglMesaSurfaceless(conf) => conf,
            Config::Gbm(conf) => conf,
            Config::EglExtDevice(conf) => conf,
        }
    }
}

impl Config {
    #[inline]
    pub fn new<ND: NativeDisplay>(
        cf: &ConfigsFinder,
        nd: &ND,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let glx_not_supported_error = make_error!(ErrorType::NotSupported(
            "GLX not supported by any of generic_egl's backends (Wayland, GBM, ect).".to_string(),
        ));
        let backing_api = cf.plat_attr.backing_api;
        match backing_api {
            BackingApi::Glx => return Err(glx_not_supported_error),
            BackingApi::GlxThenEgl => {
                warn!("[glutin] Not trying GLX as none of generic_egl's backends (Wayland, GBM, ect) support GLX.")
            }
            _ => (),
        }

        let configs = egl::Config::new(cf, nd, |confs, _| confs.into_iter().map(Ok).collect())
            .map_err(|mut err| match backing_api {
                BackingApi::GlxThenEgl => {
                    err.append(glx_not_supported_error);
                    err
                }
                _ => err,
            })?;
        Ok(configs
            .into_iter()
            .map(|(attribs, config)| {
                (
                    attribs,
                    match nd.raw_display() {
                        RawDisplay::Wayland { .. } => Config::Wayland(config),
                        RawDisplay::EglMesaSurfaceless { .. } => Config::EglMesaSurfaceless(config),
                        RawDisplay::Gbm { .. } => Config::Gbm(config),
                        RawDisplay::EglExtDevice { .. } => Config::EglExtDevice(config),
                        _ => unreachable!(),
                    },
                )
            })
            .collect())
    }

    #[inline]
    fn backend(&self) -> Backend {
        match self {
            Config::Wayland(_) => Backend::Wayland,
            Config::EglMesaSurfaceless(_) => Backend::EglMesaSurfaceless,
            Config::Gbm(_) => Backend::Gbm,
            Config::EglExtDevice(_) => Backend::EglExtDevice,
        }
    }

    #[inline]
    pub fn raw_config(&self) -> RawConfig {
        RawConfig::Egl((**self).raw_config())
    }

    #[inline]
    pub fn raw_display(&self) -> GlutinRawDisplay {
        GlutinRawDisplay::Egl((**self).raw_display())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Surface<T: SurfaceTypeTrait> {
    WaylandWindow {
        wsurface: NoCmp<NoPrint<wegl::WlEglSurface>>,
        surface: egl::Surface<T>,
    },
    WaylandPbuffer(egl::Surface<T>),
    EglMesaSurfaceless(egl::Surface<T>),
    Gbm(egl::Surface<T>),
    EglExtDevice(egl::Surface<T>),
}

impl<T: SurfaceTypeTrait> Deref for Surface<T> {
    type Target = egl::Surface<T>;

    fn deref(&self) -> &Self::Target {
        match self {
            Surface::WaylandWindow { surface, .. } | Surface::WaylandPbuffer(surface) => surface,
            Surface::EglMesaSurfaceless(surf) => surf,
            Surface::Gbm(surf) => surf,
            Surface::EglExtDevice(surf) => surf,
        }
    }
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        (**self).is_current()
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        (**self)
            .get_config()
            .map_config(|conf| match self.backend() {
                Backend::Wayland => Config::Wayland(conf),
                Backend::EglMesaSurfaceless => Config::EglMesaSurfaceless(conf),
                Backend::Gbm => Config::Gbm(conf),
                Backend::EglExtDevice => Config::EglExtDevice(conf),
            })
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        (**self).make_not_current()
    }

    #[inline]
    fn backend(&self) -> Backend {
        match self {
            Surface::WaylandWindow { .. } | Surface::WaylandPbuffer(_) => Backend::Wayland,
            Surface::EglMesaSurfaceless(_) => Backend::EglMesaSurfaceless,
            Surface::Gbm(_) => Backend::Gbm,
            Surface::EglExtDevice(_) => Backend::EglExtDevice,
        }
    }

    #[inline]
    pub fn raw_surface(&self) -> RawSurface {
        RawSurface::Egl((**self).raw_surface())
    }

    #[inline]
    pub fn size(&self) -> Result<dpi::PhysicalSize<u32>, Error> {
        (**self).size()
    }
}

impl Surface<Window> {
    #[inline]
    pub unsafe fn new<NWS: NativeWindowSource>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nws: &NWS,
        wb: NWS::WindowBuilder,
    ) -> Result<(NWS::Window, Self), Error> {
        match conf.config.backend() {
            Backend::Wayland => {
                #[allow(deprecated)]
                let nw = nws.build_wayland(
                    wb,
                    WaylandWindowParts {
                        _non_exhaustive_do_not_use: Seal,
                    },
                )?;
                Self::new_existing(conf, &nw).map(|surf| (nw, surf))
            }
            Backend::Gbm => {
                #[allow(deprecated)]
                let nw = nws.build_gbm(
                    wb,
                    GbmWindowParts {
                        color_format: (**conf.config).get_native_visual_id()? as u32,
                        _non_exhaustive_do_not_use: Seal,
                    },
                )?;
                Self::new_existing(conf, &nw).map(|surf| (nw, surf))
            }
            _ => Err(make_error!(ErrorType::NotSupported(
                "Only Wayland and GBM support native window surfaces.".to_string(),
            ))),
        }
    }

    #[inline]
    pub unsafe fn new_existing<NW: NativeWindow>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        let surface = nw.raw_window();
        match surface {
            RawWindow::Wayland { wl_surface, .. } => {
                let (width, height): (u32, u32) = nw.size().into();
                let wl_surface = wegl::WlEglSurface::new_from_raw(
                    wl_surface as *mut _,
                    width as i32,
                    height as i32,
                );

                egl::Surface::<Window>::new(
                    conf.map_config(|conf| &**conf),
                    wl_surface.ptr() as *mut _,
                )
                .map(|surface| Surface::WaylandWindow {
                    wsurface: NoCmp(NoPrint(wl_surface)),
                    surface,
                })
            }
            RawWindow::Gbm { gbm_surface, .. } => {
                egl::Surface::<Window>::new(conf.map_config(|conf| &**conf), gbm_surface)
                    .map(Surface::Gbm)
            }
            _ => Err(make_error!(ErrorType::NotSupported(
                "Non-Wayland backends do not support native surface types.".to_string(),
            ))),
        }
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize<u32>) {
        if let Surface::WaylandWindow { wsurface, .. } = self {
            let (width, height): (u32, u32) = size.into();
            wsurface.resize(width as i32, height as i32, 0, 0)
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        (**self).swap_buffers()
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        (**self).swap_buffers_with_damage(rects)
    }

    #[inline]
    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        (**self).modify_swap_interval(swap_interval)
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize<u32>,
        largest: bool,
    ) -> Result<Self, Error> {
        let backend = conf.config.backend();
        egl::Surface::<PBuffer>::new(conf.map_config(|conf| &**conf), size, largest).map(|surf| {
            match backend {
                Backend::Wayland => Surface::WaylandPbuffer(surf),
                Backend::EglMesaSurfaceless => Surface::EglMesaSurfaceless(surf),
                Backend::Gbm => Surface::Gbm(surf),
                Backend::EglExtDevice => Surface::EglExtDevice(surf),
            }
        })
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new_existing<NP: NativePixmap>(
        _conf: ConfigWrapper<&Config, &ConfigAttribs>,
        _np: &NP,
    ) -> Result<Self, Error> {
        Err(make_error!(ErrorType::NotSupported(
            "None of generic_egl's backends (Wayland, GBM, ect) support pixmaps.".to_string(),
        )))
    }

    #[inline]
    pub unsafe fn new<NPS: NativePixmapSource>(
        _conf: ConfigWrapper<&Config, &ConfigAttribs>,
        _nps: &NPS,
        _pb: NPS::PixmapBuilder,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        Err(make_error!(ErrorType::NotSupported(
            "None of generic_egl's backends (Wayland, GBM, ect) support pixmaps.".to_string(),
        )))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Context {
    Wayland(egl::Context),
    EglMesaSurfaceless(egl::Context),
    Gbm(egl::Context),
    EglExtDevice(egl::Context),
}

impl Deref for Context {
    type Target = egl::Context;

    fn deref(&self) -> &Self::Target {
        match self {
            Context::Wayland(ctx) => ctx,
            Context::EglMesaSurfaceless(ctx) => ctx,
            Context::Gbm(ctx) => ctx,
            Context::EglExtDevice(ctx) => ctx,
        }
    }
}

impl Context {
    #[inline]
    pub(crate) fn new(
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        let backend = conf.config.backend();
        egl::Context::new(
            cb.map_sharing(|ctx| &**ctx),
            conf.map_config(|conf| &**conf),
        )
        .map(|ctx| match backend {
            Backend::Wayland => Context::Wayland(ctx),
            Backend::EglMesaSurfaceless => Context::EglMesaSurfaceless(ctx),
            Backend::Gbm => Context::Gbm(ctx),
            Backend::EglExtDevice => Context::EglExtDevice(ctx),
        })
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        (**self).make_current_surfaceless()
    }

    #[inline]
    pub(crate) unsafe fn make_current<T: SurfaceTypeTrait>(
        &self,
        surf: &Surface<T>,
    ) -> Result<(), Error> {
        if self.backend() != surf.backend() {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            )));
        }

        (**self).make_current(&**surf)
    }

    #[inline]
    pub(crate) unsafe fn make_current_rw<TR: SurfaceTypeTrait, TW: SurfaceTypeTrait>(
        &self,
        read_surf: &Surface<TR>,
        write_surf: &Surface<TW>,
    ) -> Result<(), Error> {
        if self.backend() != read_surf.backend() || read_surf.backend() != write_surf.backend() {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            )));
        }

        (**self).make_current_rw(&**read_surf, &**write_surf)
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        (**self).make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        (**self).is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        (**self).get_proc_address(addr)
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        (**self)
            .get_config()
            .map_config(|conf| match self.backend() {
                Backend::Wayland => Config::Wayland(conf),
                Backend::EglMesaSurfaceless => Config::EglMesaSurfaceless(conf),
                Backend::Gbm => Config::Gbm(conf),
                Backend::EglExtDevice => Config::EglExtDevice(conf),
            })
    }

    #[inline]
    fn backend(&self) -> Backend {
        match self {
            Context::Wayland(_) => Backend::Wayland,
            Context::EglMesaSurfaceless(_) => Backend::EglMesaSurfaceless,
            Context::Gbm(_) => Backend::Gbm,
            Context::EglExtDevice(_) => Backend::EglExtDevice,
        }
    }

    #[inline]
    pub fn raw_context(&self) -> RawContext {
        RawContext::Egl((**self).raw_context())
    }
}
