use crate::api::egl;
use crate::config::{ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapInterval};
use crate::context::ContextBuilderWrapper;
use crate::platform::unix::BackingApi;
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};
use crate::utils::{NoCmp, NoPrint};

use glutin_interface::{
    NativeDisplay, NativePixmap, NativePixmapSource, NativeWindow, NativeWindowSource, RawWindow,
    Seal, WaylandWindowParts,
};
use wayland_client::egl as wegl;
pub use wayland_client::sys::client::wl_display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};

use std::os::raw;

#[derive(Debug, PartialEq, Eq)]
pub struct Config(egl::Config);

impl Config {
    #[inline]
    pub fn new<ND: NativeDisplay>(
        cf: &ConfigsFinder,
        nd: &ND,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let glx_not_supported_error = make_error!(ErrorType::NotSupported(
            "GLX not supported by Wayland".to_string(),
        ));
        let backing_api = cf.plat_attr.backing_api;
        match backing_api {
            BackingApi::Glx => return Err(glx_not_supported_error),
            BackingApi::GlxThenEgl => {
                warn!("[glutin] Not trying GLX with Wayland, as not supported by Wayland.")
            }
            _ => (),
        }

        let configs = egl::Config::new(cf, nd, |confs, _| {
            confs.into_iter().map(|config| Ok(config)).collect()
        })
        .map_err(|mut err| match backing_api {
            BackingApi::GlxThenEgl => {
                err.append(glx_not_supported_error);
                err
            }
            _ => err,
        })?;
        Ok(configs
            .into_iter()
            .map(|(attribs, config)| (attribs, Config(config)))
            .collect())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Surface<T: SurfaceTypeTrait> {
    wsurface: Option<NoCmp<NoPrint<wegl::WlEglSurface>>>,
    surface: egl::Surface<T>,
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        self.surface.is_current()
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.surface.get_config().map_config(|conf| Config(conf))
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.surface.make_not_current()
    }
}

impl Surface<Window> {
    #[inline]
    pub unsafe fn new<NWS: NativeWindowSource>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nws: &NWS,
        wb: NWS::WindowBuilder,
    ) -> Result<(NWS::Window, Self), Error> {
        #[allow(deprecated)]
        let nw = nws.build_wayland(
            wb,
            WaylandWindowParts {
                _non_exhaustive_do_not_use: Seal,
            },
        )?;
        Self::new_existing(conf, &nw).map(|surf| (nw, surf))
    }

    #[inline]
    pub unsafe fn new_existing<NW: NativeWindow>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        let (width, height): (u32, u32) = nw.size().into();

        let surface = nw.raw_window();
        let surface = match surface {
            RawWindow::Wayland { wl_surface, .. } => wl_surface,
            _ => unreachable!(),
        };

        let wsurface =
            wegl::WlEglSurface::new_from_raw(surface as *mut _, width as i32, height as i32);

        egl::Surface::<Window>::new(conf.map_config(|conf| &conf.0), wsurface.ptr() as *const _)
            .map(|surface| Surface {
                wsurface: Some(NoCmp(NoPrint(wsurface))),
                surface,
            })
    }

    #[inline]
    pub fn update_after_resize(&self, size: &dpi::PhysicalSize<u32>) {
        let (width, height): (u32, u32) = (*size).into();
        self.wsurface
            .as_ref()
            .unwrap()
            .resize(width as i32, height as i32, 0, 0)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.surface.swap_buffers()
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        self.surface.swap_buffers_with_damage(rects)
    }

    #[inline]
    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        self.surface.modify_swap_interval(swap_interval)
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: &dpi::PhysicalSize<u32>,
    ) -> Result<Self, Error> {
        egl::Surface::<PBuffer>::new(conf.map_config(|conf| &conf.0), size).map(|surface| Surface {
            wsurface: None,
            surface,
        })
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new_existing<NP: NativePixmap>(
        _conf: ConfigWrapper<&Config, &ConfigAttribs>,
        _np: &NP,
    ) -> Result<Self, Error> {
        return Err(make_error!(ErrorType::NotSupported(
            "Wayland does not support pixmaps.".to_string(),
        )));
    }

    #[inline]
    pub unsafe fn new<NPS: NativePixmapSource>(
        _conf: ConfigWrapper<&Config, &ConfigAttribs>,
        _nps: &NPS,
        _pb: NPS::PixmapBuilder,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        return Err(make_error!(ErrorType::NotSupported(
            "Wayland does not support pixmaps.".to_string(),
        )));
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Context(egl::Context);

impl Context {
    #[inline]
    pub(crate) fn new(
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        egl::Context::new(
            cb.map_sharing(|ctx| &ctx.0),
            conf.map_config(|conf| &conf.0),
        )
        .map(Context)
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        self.0.make_current_surfaceless()
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        self.0.make_current(&surf.surface)
    }

    #[inline]
    pub unsafe fn make_current_rw<TR: SurfaceTypeTrait, TW: SurfaceTypeTrait>(
        &self,
        read_surf: &Surface<TR>,
        write_surf: &Surface<TW>,
    ) -> Result<(), Error> {
        self.0
            .make_current_rw(&read_surf.surface, &write_surf.surface)
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.0.make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        self.0.get_proc_address(addr)
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.0.get_config().map_config(Config)
    }
}
