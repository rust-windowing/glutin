use crate::api::egl;
use crate::api::glx::{self, ffi};
use crate::config::{ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapInterval};
use crate::context::ContextBuilderWrapper;
use crate::platform::unix::BackingApi;
use crate::platform::unix::{RawConfig, RawContext, RawDisplay as GlutinRawDisplay, RawSurface};
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};

use glutin_interface::{
    NativeDisplay, NativePixmap, NativePixmapSource, NativeWindow, NativeWindowSource, RawDisplay,
    RawPixmap, RawWindow, Seal, X11PixmapParts, X11WindowParts,
};
use glutin_x11_sym::Display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::fmt::Debug;
use std::os::raw;
use std::sync::Arc;

pub mod utils;

#[derive(Debug, PartialEq, Eq)]
pub enum Config {
    Egl {
        config: egl::Config,
        display: Arc<Display>,
        screen: raw::c_int,
    },
    Glx(glx::Config),
}

impl Config {
    #[inline]
    pub fn new<ND: NativeDisplay>(
        cf: &ConfigsFinder,
        nd: &ND,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let xlib = syms!(XLIB);
        let (disp, screen) = match nd.raw_display() {
            RawDisplay::Xlib {
                display, screen, ..
            } => (Display::from_raw(display), screen),
            _ => unreachable!(),
        };
        let screen = unsafe { screen.unwrap_or_else(|| (xlib.XDefaultScreen)(**disp)) };

        let conf = match cf.plat_attr.backing_api {
            BackingApi::Glx | BackingApi::GlxThenEgl => Self::new_glx(cf, screen, &disp),
            BackingApi::Egl | BackingApi::EglThenGlx => Self::new_egl(cf, nd, screen, &disp),
        };

        match (&conf, cf.plat_attr.backing_api) {
            (_, BackingApi::Glx) | (_, BackingApi::Egl) | (Ok(_), _) => return conf,
            _ => (),
        }

        let conf2 = match cf.plat_attr.backing_api {
            BackingApi::EglThenGlx => Self::new_glx(cf, screen, &disp),
            BackingApi::GlxThenEgl => Self::new_egl(cf, nd, screen, &disp),
            _ => unreachable!(),
        };

        match (conf, conf2) {
            (Ok(_), _) => unreachable!(),
            (_, Ok(conf2)) => Ok(conf2),
            (Err(mut err1), Err(err2)) => Err({
                err1.append(err2);
                err1
            }),
        }
    }

    #[inline]
    fn new_glx(
        cf: &ConfigsFinder,
        screen: raw::c_int,
        disp: &Arc<Display>,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let configs = glx::Config::new(cf, screen, disp, |confs| {
            select_configs(
                disp,
                cf.plat_attr.x11_transparency,
                cf.plat_attr.x11_visual_xid,
                confs.into_iter().map(|conf| {
                    let xid = glx::get_native_visual_id(disp, conf)? as ffi::VisualID;
                    utils::get_visual_info_from_xid(disp, xid).map(|vis| (conf, vis, xid))
                }),
                // FIXME: A cookie for whoever gets rid of this clone.
                |conf| conf.clone().map(|(_, vis, xid)| (vis, xid)),
            )
            .into_iter()
            .map(|conf| conf.map(|(conf, vis)| (conf.unwrap().0, vis)))
            .collect()
        })?;
        Ok(configs
            .into_iter()
            .map(|(attribs, config)| (attribs, Config::Glx(config)))
            .collect())
    }

    #[inline]
    fn new_egl<ND: NativeDisplay>(
        cf: &ConfigsFinder,
        nd: &ND,
        screen: raw::c_int,
        disp: &Arc<Display>,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let configs = egl::Config::new(cf, nd, |confs, egl_disp| {
            select_configs(
                disp,
                cf.plat_attr.x11_transparency,
                cf.plat_attr.x11_visual_xid,
                confs,
                |config_id| {
                    let xid = egl::get_native_visual_id(***egl_disp, *config_id)? as ffi::VisualID;
                    utils::get_visual_info_from_xid(disp, xid).map(|vis| (vis, xid))
                },
            )
            .into_iter()
            .map(|config| config.map(|(conf, _)| conf))
            .collect()
        })?;
        Ok(configs
            .into_iter()
            .map(|(attribs, config)| {
                (
                    attribs,
                    Config::Egl {
                        config,
                        display: Arc::clone(disp),
                        screen,
                    },
                )
            })
            .collect())
    }

    #[inline]
    fn get_visual_info(&self) -> Result<ffi::XVisualInfo, Error> {
        match self {
            Config::Egl {
                config, display, ..
            } => utils::get_visual_info_from_xid(
                display,
                config.get_native_visual_id()? as ffi::VisualID,
            ),
            Config::Glx(conf) => Ok(conf.get_visual_info()),
        }
    }

    #[inline]
    fn display(&self) -> &Arc<Display> {
        match self {
            Config::Egl { display, .. } => display,
            Config::Glx(conf) => conf.display(),
        }
    }

    #[inline]
    fn screen(&self) -> raw::c_int {
        match self {
            Config::Egl { screen, .. } => *screen,
            Config::Glx(conf) => conf.screen(),
        }
    }

    #[inline]
    pub fn raw_display(&self) -> GlutinRawDisplay {
        match self {
            Config::Egl { config, .. } => GlutinRawDisplay::Egl(config.raw_display()),
            Config::Glx(conf) => GlutinRawDisplay::Glx(***conf.display() as *mut _),
        }
    }

    #[inline]
    pub fn raw_config(&self) -> RawConfig {
        match self {
            Config::Egl { config, .. } => RawConfig::Egl(config.raw_config()),
            Config::Glx(conf) => RawConfig::Glx(conf.raw_config()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Context {
    Egl {
        context: egl::Context,
        display: Arc<Display>,
        screen: raw::c_int,
    },
    Glx(glx::Context),
}

impl Context {
    #[inline]
    pub(crate) fn new(
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Egl {
                config,
                display,
                screen,
            } => egl::Context::new(Context::inner_cb_egl(cb)?, conf.map_config(|_| config)).map(
                |context| Context::Egl {
                    context,
                    display: Arc::clone(display),
                    screen: *screen,
                },
            ),
            Config::Glx(config) => {
                glx::Context::new(Context::inner_cb_glx(cb)?, conf.map_config(|_| config))
                    .map(Context::Glx)
            }
        }
    }

    #[inline]
    fn inner_cb_glx(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&glx::Context>, Error> {
        match cb.sharing {
            Some(Context::Glx(_)) | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a GLX context with a non-GLX context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context::Glx(ctx) => ctx,
            _ => unreachable!(),
        }))
    }

    #[inline]
    fn inner_cb_egl(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&egl::Context>, Error> {
        match cb.sharing {
            Some(Context::Egl { .. }) | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a EGL context with a non-EGL context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context::Egl { context, .. } => context,
            _ => unreachable!(),
        }))
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        match self {
            Context::Egl { context, .. } => context.make_current_surfaceless(),
            Context::Glx(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        match (self, surf) {
            (Context::Egl { context, .. }, Surface::Egl { surface, .. }) => {
                context.make_current(surface)
            }
            (Context::Glx(ref ctx), Surface::Glx(ref surf)) => ctx.make_current(surf),
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn make_current_rw<TR: SurfaceTypeTrait, TW: SurfaceTypeTrait>(
        &self,
        read_surf: &Surface<TR>,
        write_surf: &Surface<TW>,
    ) -> Result<(), Error> {
        match (self, read_surf, write_surf) {
            (
                Context::Egl { context, .. },
                Surface::Egl {
                    surface: read_surf, ..
                },
                Surface::Egl {
                    surface: write_surf,
                    ..
                },
            ) => context.make_current_rw(read_surf, write_surf),
            (Context::Glx(ref ctx), Surface::Glx(ref read_surf), Surface::Glx(ref write_surf)) => {
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
            Context::Egl { context, .. } => context.make_not_current(),
            Context::Glx(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Context::Egl { context, .. } => context.is_current(),
            Context::Glx(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn raw_context(&self) -> RawContext {
        match self {
            Context::Egl { context, .. } => RawContext::Egl(context.raw_context()),
            Context::Glx(ref ctx) => RawContext::Glx(ctx.raw_context()),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        match self {
            Context::Egl { context, .. } => context.get_proc_address(addr),
            Context::Glx(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Context::Egl {
                context,
                display,
                screen,
            } => context.get_config().map_config(|config| Config::Egl {
                config,
                display: Arc::clone(display),
                screen: *screen,
            }),
            Context::Glx(ref ctx) => ctx.get_config().map_config(Config::Glx),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Surface<T: SurfaceTypeTrait> {
    Egl {
        surface: egl::Surface<T>,
        display: Arc<Display>,
        screen: raw::c_int,
    },
    Glx(glx::Surface<T>),
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Surface::Egl { surface, .. } => surface.is_current(),
            Surface::Glx(ref surf) => surf.is_current(),
        }
    }

    #[inline]
    pub fn raw_surface(&self) -> RawSurface {
        match self {
            Surface::Egl { surface, .. } => RawSurface::Egl(surface.raw_surface()),
            Surface::Glx(ref surf) => RawSurface::Glx(surf.raw_surface()),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Surface::Egl {
                surface,
                display,
                screen,
            } => surface.get_config().map_config(|config| Config::Egl {
                config,
                display: Arc::clone(display),
                screen: *screen,
            }),
            Surface::Glx(ref ctx) => ctx.get_config().map_config(Config::Glx),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Surface::Egl { surface, .. } => surface.make_not_current(),
            Surface::Glx(ref surf) => surf.make_not_current(),
        }
    }

    #[inline]
    pub fn size(&self) -> Result<dpi::PhysicalSize<u32>, Error> {
        match self {
            Surface::Egl { surface, .. } => surface.size(),
            Surface::Glx(ref surf) => surf.size(),
        }
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize<u32>,
        largest: bool,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Egl {
                config,
                display,
                screen,
            } => egl::Surface::<PBuffer>::new(conf.map_config(|_| config), size, largest).map(
                |surface| Surface::Egl {
                    surface,
                    display: Arc::clone(&display),
                    screen: *screen,
                },
            ),
            Config::Glx(config) => {
                glx::Surface::<PBuffer>::new(conf.map_config(|_| config), size, largest)
                    .map(Surface::Glx)
            }
        }
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub fn new<NPS: NativePixmapSource>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nps: &NPS,
        pb: NPS::PixmapBuilder,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        // Get the screen_id for the window being built.
        let visual_info: ffi::XVisualInfo = conf.config.get_visual_info()?;
        #[allow(deprecated)]
        let np = nps.build_x11(
            pb,
            X11PixmapParts {
                depth: visual_info.depth as u16,
                _non_exhaustive_do_not_use: Seal,
            },
        )?;
        Self::new_existing(conf, &np).map(|surf| (np, surf))
    }

    #[inline]
    pub fn new_existing<NP: NativePixmap>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        np: &NP,
    ) -> Result<Self, Error> {
        let surface = np.raw_pixmap();
        let mut surface = match surface {
            RawPixmap::Xlib { pixmap, .. } => pixmap,
            _ => unreachable!(),
        };

        match conf.config {
            Config::Egl {
                config,
                display,
                screen,
            } => egl::Surface::<Pixmap>::new(
                conf.map_config(|_| config),
                &mut surface as *mut _ as *mut _,
            )
            .map(|surface| Surface::Egl {
                surface,
                display: Arc::clone(display),
                screen: *screen,
            }),
            Config::Glx(config) => {
                glx::Surface::<Pixmap>::new(conf.map_config(|_| config), surface).map(Surface::Glx)
            }
        }
    }
}

impl Surface<Window> {
    #[inline]
    pub fn new<NWS: NativeWindowSource>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nws: &NWS,
        wb: NWS::WindowBuilder,
    ) -> Result<(NWS::Window, Self), Error> {
        // Get the screen_id for the window being built.
        let visual_info: ffi::XVisualInfo = conf.config.get_visual_info()?;
        #[allow(deprecated)]
        let nw = nws.build_x11(
            wb,
            X11WindowParts {
                x_visual_info: &visual_info as *const _ as *const _,
                screen: conf.config.screen(),
                _non_exhaustive_do_not_use: Seal,
            },
        )?;
        Self::new_existing(conf, &nw).map(|surf| (nw, surf))
    }

    #[inline]
    pub fn new_existing<NW: NativeWindow>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        let xlib = syms!(XLIB);
        let surface = nw.raw_window();
        let mut surface = match surface {
            RawWindow::Xlib { window, .. } => window,
            _ => unreachable!(),
        };

        let visual_info = conf.config.get_visual_info()?;
        let window_attrs = {
            let mut window_attrs = unsafe { std::mem::zeroed() };
            let window_attr_error = make_oserror!(OsError::Misc(
                "Glutin failed to query for a window's window attributes.".to_string()
            ));
            conf.config.display().check_errors().map_err(|mut err| {
                err.append(window_attr_error.clone());
                err
            })?;
            if unsafe {
                (xlib.XGetWindowAttributes)(***conf.config.display(), surface, &mut window_attrs)
            } == 0
            {
                return Err(window_attr_error);
            }
            window_attrs
        };

        #[inline]
        fn assemble_non_match_error<T: Debug + PartialEq>(
            name: &str,
            a: T,
            b: T,
        ) -> Result<(), Error> {
            if a != b {
                return Err(make_oserror!(OsError::Misc(format!(
                    "Config's {} and window's {} do not match, {:?} != {:?}",
                    name, name, a, b
                ))));
            }
            Ok(())
        }
        assemble_non_match_error("visual", visual_info.visual, window_attrs.visual)?;
        assemble_non_match_error("depth", visual_info.depth, window_attrs.depth)?;

        match conf.config {
            Config::Egl {
                config,
                display,
                screen,
            } => egl::Surface::<Window>::new(
                conf.map_config(|_| config),
                &mut surface as *mut _ as *mut _,
            )
            .map(|surface| Surface::Egl {
                surface,
                display: Arc::clone(display),
                screen: *screen,
            }),
            Config::Glx(config) => {
                glx::Surface::<Window>::new(conf.map_config(|_| config), surface).map(Surface::Glx)
            }
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        match self {
            Surface::Egl { surface, .. } => surface.swap_buffers(),
            Surface::Glx(ref surf) => surf.swap_buffers(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        match self {
            Surface::Egl { surface, .. } => surface.swap_buffers_with_damage(rects),
            Surface::Glx(ref surf) => surf.swap_buffers_with_damage(rects),
        }
    }

    #[inline]
    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        match self {
            Surface::Egl { surface, .. } => surface.modify_swap_interval(swap_interval),
            Surface::Glx(ref surf) => surf.modify_swap_interval(swap_interval),
        }
    }
}

// FIXME:
// When using egl, all the configs will not support transparency, even if
// transparency does work with glx.
//
// https://bugs.freedesktop.org/show_bug.cgi?id=67676
// I'm working on a patch.
#[inline]
pub fn select_configs<T, I: IntoIterator<Item = T>, F>(
    disp: &Arc<Display>,
    target_transparency: Option<bool>,
    target_visual_xid: Option<raw::c_ulong>,
    conf_ids: I,
    mut convert_to_xvisualinfo: F,
) -> Vec<Result<(T, ffi::XVisualInfo), Error>>
where
    F: FnMut(&T) -> Result<(ffi::XVisualInfo, ffi::VisualID), Error>,
{
    conf_ids
        .into_iter()
        .map(|conf_id| {
            let (visual_info, xid) = convert_to_xvisualinfo(&conf_id)?;

            match utils::examine_visual_info(
                disp,
                visual_info,
                target_transparency,
                target_visual_xid,
            ) {
                Ok(()) => Ok((conf_id, visual_info)),
                Err(lacks) => Err(make_oserror!(OsError::Misc(format!(
                    "X11 xid {:?} is lacking {:?}",
                    xid, lacks
                )))),
            }
        })
        .collect()
}
