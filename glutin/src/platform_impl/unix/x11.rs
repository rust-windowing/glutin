use crate::api::egl::{self, EGL};
use crate::api::glx::{self, ffi, GLX};
use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::ContextBuilderWrapper;
use crate::platform::unix::BackingApi;
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};

use glutin_interface::{
    NativeDisplay, NativePixmap, NativePixmapSource, NativeWindow, NativeWindowSource, RawDisplay,
    RawWindow, Seal, X11WindowParts,
};
use glutin_x11_sym::Display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::os::raw;
use std::sync::Arc;

pub mod utils;

#[derive(Debug)]
pub enum Config {
    Egl(egl::Config, Arc<Display>, raw::c_int),
    Glx(glx::Config),
}

impl Config {
    pub fn new<ND: NativeDisplay>(
        cb: &ConfigBuilder,
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

        let conf = match cb.plat_attr.backing_api {
            BackingApi::Glx | BackingApi::GlxThenEgl => Self::new_glx(cb, screen, &disp),
            BackingApi::Egl | BackingApi::EglThenGlx => Self::new_egl(cb, nd, screen, &disp),
        };

        match (&conf, cb.plat_attr.backing_api) {
            (_, BackingApi::Glx) | (_, BackingApi::Egl) | (Ok(_), _) => return conf,
            _ => (),
        }

        let conf2 = match cb.plat_attr.backing_api {
            BackingApi::EglThenGlx => Self::new_glx(cb, screen, &disp),
            BackingApi::GlxThenEgl => Self::new_egl(cb, nd, screen, &disp),
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
        cb: &ConfigBuilder,
        screen: raw::c_int,
        disp: &Arc<Display>,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let configs = glx::Config::new(cb, screen, disp, |confs| {
            select_configs(
                disp,
                cb.plat_attr.x11_transparency,
                cb.plat_attr.x11_visual_xid,
                confs,
                |config_id| unimplemented!(),
            )
            .into_iter()
            .map(|config| config.map(|(conf, _)| conf))
            .collect()
        })?;
        Ok(configs
            .into_iter()
            .map(|(attribs, config)| (attribs, Config::Glx(config)))
            .collect())
    }

    #[inline]
    fn new_egl<ND: NativeDisplay>(
        cb: &ConfigBuilder,
        nd: &ND,
        screen: raw::c_int,
        disp: &Arc<Display>,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        let configs = egl::Config::new(cb, nd, |confs, egl_disp| {
            select_configs(
                disp,
                cb.plat_attr.x11_transparency,
                cb.plat_attr.x11_visual_xid,
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
            .map(|(attribs, config)| (attribs, Config::Egl(config, Arc::clone(disp), screen)))
            .collect())
    }

    #[inline]
    fn get_visual_info(&self) -> Result<ffi::XVisualInfo, Error> {
        match self {
            Config::Egl(conf, disp, _) => {
                utils::get_visual_info_from_xid(disp, conf.get_native_visual_id()? as ffi::VisualID)
            }
            _ => unimplemented!(),
        }
    }

    #[inline]
    fn display(&self) -> &Arc<Display> {
        match self {
            Config::Egl(_, disp, _) => disp,
            Config::Glx(conf) => conf.display(),
        }
    }

    #[inline]
    fn screen(&self) -> raw::c_int {
        match self {
            Config::Egl(_, _, screen) => *screen,
            Config::Glx(conf) => conf.screen(),
        }
    }
}

#[derive(Debug)]
pub enum Context {
    Egl(egl::Context, Arc<Display>, raw::c_int),
    //Glx(glx::Display),
}

impl Context {
    #[inline]
    pub(crate) fn new(
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Egl(config, disp, screen) => {
                egl::Context::new(Context::inner_cb_egl(cb)?, conf.map_config(|_| config))
                    .map(|ctx| Context::Egl(ctx, Arc::clone(disp), *screen))
            }
            _ => unimplemented!(),
            //(BackendDisplay::Glx(disp), Config::Glx(config)) => {
            //    glx::Context::new(
            //        disp,
            //        Context::inner_cb_glx(cb)?,
            //        conf.map_config(|_| config),
            //    )
            //    .map(Context::Glx)
            //},
        }
    }

    fn inner_cb_egl(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&egl::Context>, Error> {
        match cb.sharing {
            Some(Context::Egl(_, _, _)) | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a EGL context with a non-EGL context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context::Egl(ctx, _, _) => ctx,
            _ => unreachable!(),
        }))
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        match self {
            Context::Egl(ref ctx, _, _) => ctx.make_current_surfaceless(),
            //Context::Glx(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        match (self, surf) {
            (Context::Egl(ref ctx, _, _), Surface::Egl(ref surf, _, _)) => ctx.make_current(surf),
            //(Context::Glx(ref ctx), Surface::Glx(ref surf)) => ctx.make_current(surf),
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Context::Egl(ref ctx, _, _) => ctx.make_not_current(),
            //Context::Glx(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Context::Egl(ref ctx, _, _) => ctx.is_current(),
            //Context::Glx(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self {
            Context::Egl(ref ctx, _, _) => ctx.get_api(),
            //Context::Glx(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const raw::c_void {
        match self {
            Context::Egl(ref ctx, _, _) => ctx.get_proc_address(addr),
            //Context::Glx(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Context::Egl(ref ctx, disp, screen) => ctx
                .get_config()
                .map_config(|conf| Config::Egl(conf, Arc::clone(disp), *screen)),
            //Context::Glx(ref ctx) => ctx.get_config().map_config(Config::Glx),
        }
    }
}

#[derive(Debug)]
pub enum Surface<T: SurfaceTypeTrait> {
    Egl(egl::Surface<T>, Arc<Display>, raw::c_int),
    //Glx(glx::Display),
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Surface::Egl(ref surf, _, _) => surf.is_current(),
            //Surface::Glx(ref surf) => surf.is_current(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Surface::Egl(ref surf, disp, screen) => surf
                .get_config()
                .map_config(|conf| Config::Egl(conf, Arc::clone(disp), *screen)),
            //Context::Glx(ref ctx) => ctx.get_config().map_config(Config::Glx),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Surface::Egl(ref surf, _, _) => surf.make_not_current(),
            //Surface::Glx(ref surf) => surf.make_not_current(),
        }
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize<u32>,
    ) -> Result<Self, Error> {
        match conf.config {
            Config::Egl(config, disp, screen) => {
                egl::Surface::<PBuffer>::new(conf.map_config(|_| config), size)
                    .map(|surf| Surface::Egl(surf, Arc::clone(&disp), *screen))
            }
            _ => unimplemented!(),
            //(BackendDisplay::Glx(disp), Config::Glx(config)) => {
            //    glx::Surface::<PBuffer>::new(
            //        disp,
            //        conf.map_config(|_| config),
            //        size,
            //    )
            //    .map(Surface::Glx)
            //},
        }
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub fn new<NPS: NativePixmapSource>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nps: &NPS,
        wb: NPS::PixmapBuilder,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        unimplemented!()
        //match (disp, conf.config) {
        //    (Display::Egl(disp), Config::Egl(config)) => {
        //        egl::Surface::<Pixmap>::new(
        //            disp,
        //            conf.map_config(|_| config),
        //            nps,
        //        )
        //        .map(|(pix, surf)| (pix, Surface::Egl(surf)))
        //    },
        //    (Display::Glx(disp), Config::Glx(config)) => {
        //        glx::Surface::<Pixmap>::new(
        //            disp,
        //            conf.map_config(|_| config),
        //            nps,
        //        )
        //        .map(|(pix, surf)| (pix, Surface::Glx(surf)))
        //    },
        //    (_, _) => Err(make_error!(ErrorType::BadApiUsage(
        //        "Incompatible display and config backends.".to_string()
        //    )))
        //}
    }

    #[inline]
    pub fn new_existing<NP: NativePixmap>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        np: &NP,
    ) -> Result<Self, Error> {
        unimplemented!()
        //match (disp, conf.config) {
        //    (Display::Egl(disp), Config::Egl(config)) => {
        //        egl::Surface::<Pixmap>::new_existing(
        //            disp,
        //            conf.map_config(|_| config),
        //            np,
        //        )
        //        .map(Surface::Egl)
        //    },
        //    (Display::Glx(disp), Config::Glx(config)) => {
        //        glx::Surface::<Pixmap>::new_existing(
        //            disp,
        //            conf.map_config(|_| config),
        //            np,
        //        )
        //        .map(Surface::Glx)
        //    },
        //    (_, _) => Err(make_error!(ErrorType::BadApiUsage(
        //        "Incompatible display and config backends.".to_string()
        //    )))
        //}
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
        let visual_info = conf.config.get_visual_info();
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
        let surface = match surface {
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
            Config::Egl(config, disp, screen) => {
                egl::Surface::<Window>::new(conf.map_config(|_| config), surface as *const _)
                    .map(|surf| Surface::Egl(surf, Arc::clone(disp), *screen))
            }
            _ => unimplemented!(),
            //(Display::Glx(disp), Config::Glx(config)) => {
            //    glx::Surface::<Window>::new(
            //        disp,
            //        conf.map_config(|_| config),
            //        nws,
            //    )
            //    .map(|surf| (win, Surface::Glx(surf)))
            //},
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        match self {
            Surface::Egl(ref surf, _, _) => surf.swap_buffers(),
            //Surface::Glx(ref surf) => surf.swap_buffers(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        match self {
            Surface::Egl(ref surf, _, _) => surf.swap_buffers_with_damage(rects),
            //Surface::Glx(ref surf) => surf.swap_buffers_with_damage(rects),
        }
    }
}

// FIXME:
// When using egl, all the configs will not support transparency, even if
// transparency does work with glx.
//
// https://bugs.freedesktop.org/show_bug.cgi?id=67676
// I'm working on a patch.
pub fn select_configs<'a, T, I: IntoIterator<Item = T>, F>(
    disp: &Arc<Display>,
    target_transparency: Option<bool>,
    target_visual_xid: Option<raw::c_ulong>,
    conf_ids: I,
    mut convert_to_xvisualinfo: F,
) -> Vec<Result<(T, ffi::XVisualInfo), Error>>
where
    F: FnMut(&T) -> Result<(ffi::XVisualInfo, ffi::VisualID), Error>,
{
    use utils::Lacks;

    conf_ids
        .into_iter()
        .map(|conf_id| {
            let (visual_infos, xid) = convert_to_xvisualinfo(&conf_id)?;

            match utils::examine_visual_info(
                disp,
                visual_infos,
                target_transparency,
                target_visual_xid,
            ) {
                Ok(()) => Ok((conf_id, visual_infos)),
                Err(lacks) => Err(make_oserror!(OsError::Misc(format!(
                    "X11 xid {:?} is lacking {:?}",
                    xid, lacks
                )))),
            }
        })
        .collect()
}
