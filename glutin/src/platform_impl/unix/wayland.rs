use crate::api::egl;
use crate::api::egl::ffi;
use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::ContextBuilderWrapper;
use crate::display::DisplayBuilder;
use crate::platform_impl::BackingApi;
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};
use crate::utils::NoPrint;

use glutin_interface::inputs::{
    NativeDisplay, NativePixmap, NativePixmapBuilder, NativeWindow, NativeWindowBuilder, RawWindow,
};
use wayland_client::egl as wegl;
pub use wayland_client::sys::client::wl_display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};

use std::ops::Deref;
use std::os::raw;
use std::sync::Arc;

#[derive(Debug)]
pub struct Display(egl::Display);

impl Display {
    pub fn new<ND: NativeDisplay>(db: DisplayBuilder, nd: &ND) -> Result<Self, Error> {
        let glx_not_supported_error = make_error!(ErrorType::NotSupported(
            "GLX not supported by Wayland".to_string(),
        ));
        let backing_api = db.plat_attr.backing_api;
        match backing_api {
            BackingApi::Glx => return Err(glx_not_supported_error),
            BackingApi::GlxThenEgl => {
                warn!("[glutin] Not trying GLX with Wayland, as not supported by Wayland.")
            }
            _ => (),
        }

        egl::Display::new(db, nd)
            .map(Display)
            .map_err(|mut err| match backing_api {
                BackingApi::GlxThenEgl => { err.append(glx_not_supported_error); err},
                _ => err,
            })
    }
}

#[derive(Debug)]
pub struct Config(egl::Config);

impl Config {
    pub fn new(disp: &Display, cb: ConfigBuilder) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let configs = egl::Config::new(&disp.0, cb, |confs| {
            confs.into_iter().map(|config| Ok(config)).collect()
        })?;
        Ok(configs
            .into_iter()
            .map(|(attribs, config)| (attribs, Config(config)))
            .collect())
    }
}

#[derive(Debug)]
pub struct Surface<T: SurfaceTypeTrait> {
    wsurface: Option<NoPrint<wegl::WlEglSurface>>,
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
    pub unsafe fn new<NWB: NativeWindowBuilder>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nwb: NWB,
    ) -> Result<(NWB::Window, Self), Error> {
        let nw = nwb.build_wayland()?;
        Self::new_existing(disp, conf, &nw).map(|surf| (nw, surf))
    }

    #[inline]
    pub unsafe fn new_existing<NW: NativeWindow>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        let (width, height): (u32, u32) = nw.size().into();

        let surface = nw.raw_window();
        let surface = match surface {
            RawWindow::Wayland { wl_surface, .. } => wl_surface,
            _ => unreachable!(),
        };

        let wsurface = unsafe {
            wegl::WlEglSurface::new_from_raw(surface as *mut _, width as i32, height as i32)
        };

        egl::Surface::<Window>::new(
            &disp.0,
            conf.map_config(|conf| &conf.0),
            wsurface.ptr() as *const _,
        )
        .map(|surface| Surface {
            wsurface: Some(NoPrint(wsurface)),
            surface,
        })
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        let (width, height): (u32, u32) = size.into();
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
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        egl::Surface::<PBuffer>::new(&disp.0, conf.map_config(|conf| &conf.0), size).map(
            |surface| Surface {
                wsurface: None,
                surface,
            },
        )
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new_existing<NP: NativePixmap>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        np: &NP,
    ) -> Result<Self, Error> {
        return Err(make_error!(ErrorType::NotSupported(
            "Wayland does not support pixmaps.".to_string(),
        )));
    }

    #[inline]
    pub unsafe fn new<NPB: NativePixmapBuilder>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        npb: NPB,
    ) -> Result<(NPB::Pixmap, Self), Error> {
        return Err(make_error!(ErrorType::NotSupported(
            "Wayland does not support pixmaps.".to_string(),
        )));
    }
}

#[derive(Debug)]
pub struct Context(egl::Context);

impl Context {
    #[inline]
    pub(crate) fn new(
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        egl::Context::new(
            &disp.0,
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
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.0.make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.0.get_api()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const raw::c_void {
        self.0.get_proc_address(addr)
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.0.get_config().map_config(Config)
    }
}
