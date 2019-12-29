use crate::api::egl::{self, EGL};
use crate::api::glx::{self, ffi, GLX};
use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::ContextBuilderWrapper;
use crate::display::DisplayBuilder;
use crate::platform_impl::BackingApi;
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};

use glutin_interface::inputs::{
    NativeDisplay, NativePixmap, NativePixmapBuilder, NativeWindow, NativeWindowBuilder, RawDisplay,
};
use glutin_x11_sym::Display as X11Display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::ops::{Deref, DerefMut};
use std::os::raw;
use std::sync::Arc;

pub mod utils;

#[derive(Debug)]
pub struct Display {
    native_display: Arc<X11Display>,
    display: BackendDisplay,
    screen: Option<raw::c_int>,
}

#[derive(Debug)]
pub enum BackendDisplay {
    Egl(egl::Display),
    //Glx(glx::Display),
}

impl Display {
    pub fn new<ND: NativeDisplay>(db: DisplayBuilder, nd: &ND) -> Result<Self, Error> {
        let (native_display, screen) = match nd.display() {
            RawDisplay::Xlib {
                display, screen, ..
            } => (X11Display::from_raw(display), screen),
            _ => unreachable!(),
        };

        let display = BackendDisplay::new(db, nd)?;
        Ok(Display {
            display,
            native_display,
            screen,
        })
    }
}

impl BackendDisplay {
    fn new<ND: NativeDisplay>(db: DisplayBuilder, nd: &ND) -> Result<Self, Error> {
        let backing_api = db.plat_attr.backing_api;
        let disp = match backing_api {
            BackingApi::Glx | BackingApi::GlxThenEgl => unimplemented!(),
            BackingApi::Egl | BackingApi::EglThenGlx => {
                egl::Display::new(db.clone(), nd).map(BackendDisplay::Egl)
            }
        };

        match (&disp, backing_api) {
            (_, BackingApi::Glx) | (_, BackingApi::Egl) | (Ok(_), _) => return disp,
            _ => (),
        }

        let disp2 = match backing_api {
            BackingApi::EglThenGlx => unimplemented!(),
            BackingApi::GlxThenEgl => egl::Display::new(db, nd).map(BackendDisplay::Egl),
            _ => unreachable!(),
        };

        match (disp, disp2) {
            (Ok(_), _) => unreachable!(),
            (_, Ok(disp2)) => Ok(disp2),
            (Err(err1), Err(err2)) => Err(append_errors!(err1, err2)),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    native_display: Arc<X11Display>,
    config: BackendConfig,
}

#[derive(Debug)]
pub enum BackendConfig {
    Egl(egl::Config),
    //Glx(glx::Display),
}

impl BackendConfig {
    pub fn new(disp: &Display, cb: ConfigBuilder) -> Result<(ConfigAttribs, Self), Error> {
        match disp.display {
            BackendDisplay::Egl(ref bdisp) => egl::Config::new(bdisp, cb, |confs| {
                select_config(
                    &disp.native_display,
                    cb.plat_attr.x11_transparency,
                    cb.plat_attr.x11_visual_xid,
                    confs,
                    |config_id| {
                        utils::get_visual_info_from_xid(
                            &disp.native_display,
                            egl::get_native_visual_id(***bdisp, *config_id) as ffi::VisualID,
                        )
                    },
                )
                .map(|(conf, _)| conf)
            })
            .map(|(attribs, conf)| (attribs, BackendConfig::Egl(conf))),
        }
    }
}

impl Config {
    pub fn new(disp: &Display, cb: ConfigBuilder) -> Result<(ConfigAttribs, Self), Error> {
        let (attribs, config) = BackendConfig::new(disp, cb)?;
        Ok((
            attribs,
            Config {
                config,
                native_display: Arc::clone(&disp.native_display),
            },
        ))
    }

    #[inline]
    pub fn get_visual_info(&self) -> Result<ffi::XVisualInfo, Error> {
        match self.config {
            BackendConfig::Egl(conf) => utils::get_visual_info_from_xid(
                &self.native_display,
                conf.get_native_visual_id() as ffi::VisualID,
            ),
        }
    }
}

#[derive(Debug)]
pub struct Context {
    native_display: Arc<X11Display>,
    context: BackendContext,
}

#[derive(Debug)]
pub enum BackendContext {
    Egl(egl::Context),
    //Glx(glx::Display),
}

impl BackendContext {
    #[inline]
    pub(crate) fn new(
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        match (&disp.display, &conf.config.config) {
            (BackendDisplay::Egl(disp), BackendConfig::Egl(config)) => egl::Context::new(
                disp,
                Context::inner_cb_egl(cb)?,
                conf.map_config(|_| config),
            )
            .map(BackendContext::Egl),
            //(BackendDisplay::Glx(disp), Config::Glx(config)) => {
            //    glx::Context::new(
            //        disp,
            //        Context::inner_cb_glx(cb)?,
            //        conf.map_config(|_| config),
            //    )
            //    .map(BackendContext::Glx)
            //},
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
        }
    }
}

impl Context {
    fn inner_cb_egl(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&egl::Context>, Error> {
        match cb.sharing {
            Some(Context {
                context: BackendContext::Egl(_),
                ..
            })
            | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a EGL context with a non-EGL context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context {
                context: BackendContext::Egl(ctx),
                ..
            } => ctx,
            _ => unreachable!(),
        }))
    }

    #[inline]
    pub(crate) fn new(
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        let context = BackendContext::new(disp, cb, conf)?;
        Ok(Context {
            context,
            native_display: Arc::clone(&disp.native_display),
        })
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        match &self.context {
            BackendContext::Egl(ref ctx) => ctx.make_current_surfaceless(),
            //Context::Glx(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        match (&self.context, &surf.surface) {
            (BackendContext::Egl(ref ctx), BackendSurface::Egl(ref surf)) => ctx.make_current(surf),
            //(Context::Glx(ref ctx), Surface::Glx(ref surf)) => ctx.make_current(surf),
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match &self.context {
            BackendContext::Egl(ref ctx) => ctx.make_not_current(),
            //Context::Glx(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match &self.context {
            BackendContext::Egl(ref ctx) => ctx.is_current(),
            //Context::Glx(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match &self.context {
            BackendContext::Egl(ref ctx) => ctx.get_api(),
            //Context::Glx(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const raw::c_void {
        match &self.context {
            BackendContext::Egl(ref ctx) => ctx.get_proc_address(addr),
            //Context::Glx(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match &self.context {
            BackendContext::Egl(ref ctx) => ctx.get_config().map_config(BackendConfig::Egl),
            //Context::Glx(ref ctx) => ctx.get_config().map_config(BackendConfig::Glx),
        }
        .map_config(|config| Config {
            config,
            native_display: Arc::clone(&self.native_display),
        })
    }
}

#[derive(Debug)]
pub struct Surface<T: SurfaceTypeTrait> {
    native_display: Arc<X11Display>,
    surface: BackendSurface<T>,
}

#[derive(Debug)]
pub enum BackendSurface<T: SurfaceTypeTrait> {
    Egl(egl::Surface<T>),
    //Glx(glx::Display),
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        match &self.surface {
            BackendSurface::Egl(ref surf) => surf.is_current(),
            //Surface::Glx(ref surf) => surf.is_current(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match &self.surface {
            BackendSurface::Egl(ref ctx) => ctx.get_config().map_config(BackendConfig::Egl),
            //Context::Glx(ref ctx) => ctx.get_config().map_config(BackendConfig::Glx),
        }
        .map_config(|config| Config {
            config,
            native_display: Arc::clone(&self.native_display),
        })
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match &self.surface {
            BackendSurface::Egl(ref surf) => surf.make_not_current(),
            //Surface::Glx(ref surf) => surf.make_not_current(),
        }
    }
}

impl BackendSurface<PBuffer> {
    #[inline]
    pub fn new(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        match (&disp.display, &conf.config.config) {
            (BackendDisplay::Egl(disp), BackendConfig::Egl(config)) => {
                egl::Surface::<PBuffer>::new(disp, conf.map_config(|_| config), size)
                    .map(BackendSurface::Egl)
            }
            //(BackendDisplay::Glx(disp), Config::Glx(config)) => {
            //    glx::Surface::<PBuffer>::new(
            //        disp,
            //        conf.map_config(|_| config),
            //        size,
            //    )
            //    .map(Surface::Glx)
            //},
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
        }
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub(crate) fn new(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        let surface = BackendSurface::<PBuffer>::new(disp, conf, size)?;
        Ok(Surface {
            surface,
            native_display: Arc::clone(&disp.native_display),
        })
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub fn new<NPB: NativePixmapBuilder>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        npb: NPB,
    ) -> Result<(NPB::Pixmap, Self), Error> {
        unimplemented!()
        //match (disp, conf.config) {
        //    (Display::Egl(disp), Config::Egl(config)) => {
        //        egl::Surface::<Pixmap>::new(
        //            disp,
        //            conf.map_config(|_| config),
        //            npb,
        //        )
        //        .map(|(pix, surf)| (pix, Surface::Egl(surf)))
        //    },
        //    (Display::Glx(disp), Config::Glx(config)) => {
        //        glx::Surface::<Pixmap>::new(
        //            disp,
        //            conf.map_config(|_| config),
        //            npb,
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
        disp: &Display,
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

impl BackendSurface<Window> {
    #[inline]
    pub fn new<NWB: NativeWindowBuilder>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nwb: NWB,
    ) -> Result<(NWB::Window, Self), Error> {
        let xlib = syms!(XLIB);
        // Get the screen_id for the window being built.
        let screen = disp
            .screen
            .unwrap_or(unsafe { (xlib.XDefaultScreen)(**disp.native_display) });
        let visual_info = conf.config.get_visual_info();

        let win = nwb.build_x11(&mut visual_info as *mut _ as *mut _, screen)?;
        match (&disp.display, &conf.config.config) {
            (BackendDisplay::Egl(disp), BackendConfig::Egl(config)) => {
                egl::Surface::<Window>::new(disp, conf.map_config(|_| config), nwb)
                    .map(|surf| (win, BackendSurface::Egl(surf)))
            }
            //(Display::Glx(disp), Config::Glx(config)) => {
            //    glx::Surface::<Window>::new(
            //        disp,
            //        conf.map_config(|_| config),
            //        nwb,
            //    )
            //    .map(|surf| (win, Surface::Glx(surf)))
            //},
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub fn new_existing<NW: NativeWindow>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        unimplemented!()
        //match (disp, conf.config) {
        //    (Display::Egl(disp), Config::Egl(config)) => {
        //        egl::Surface::<Window>::new_existing(
        //            disp,
        //            conf.map_config(|_| config),
        //            nw,
        //        )
        //        .map(Surface::Egl)
        //    },
        //    (Display::Glx(disp), Config::Glx(config)) => {
        //        glx::Surface::<Window>::new_existing(
        //            disp,
        //            conf.map_config(|_| config),
        //            nw,
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
    pub fn new<NWB: NativeWindowBuilder>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nwb: NWB,
    ) -> Result<(NWB::Window, Self), Error> {
        let (win, surface) = BackendSurface::<Window>::new(disp, conf, nwb)?;
        Ok((
            win,
            Surface {
                surface,
                native_display: Arc::clone(&disp.native_display),
            },
        ))
    }

    #[inline]
    pub fn new_existing<NW: NativeWindow>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        let surface = BackendSurface::<Window>::new_existing(disp, conf, nw)?;
        Ok(Surface {
            surface,
            native_display: Arc::clone(&disp.native_display),
        })
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        match &self.surface {
            BackendSurface::Egl(ref surf) => surf.swap_buffers(),
            //Surface::Glx(ref surf) => surf.swap_buffers(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        match &self.surface {
            BackendSurface::Egl(ref surf) => surf.swap_buffers_with_damage(rects),
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
pub fn select_config<'a, T, F>(
    native_disp: &Arc<X11Display>,
    target_transparency: Option<bool>,
    target_visual_xid: Option<raw::c_ulong>,
    conf_ids: Vec<T>,
    mut convert_to_xvisualinfo: F,
) -> Result<(T, ffi::XVisualInfo), Error>
where
    F: FnMut(&T) -> Result<ffi::XVisualInfo, Error>,
{
    use utils::Lacks;
    let mut chosen_conf_id = None;
    let mut lacks_what = None;

    let mut errors = make_oserror!(OsError::Misc(
        "Glutin failed to choose a config because none of them (if any) had a valid XVisualInfo."
            .to_string()
    ));

    for conf_id in conf_ids {
        let visual_infos = match convert_to_xvisualinfo(&conf_id) {
            Ok(vi) => vi,
            Err(err) => {
                errors = append_errors!(err, errors);
                continue;
            },
        };

        let this_lacks_what = utils::examine_visual_info(
            native_disp,
            visual_infos,
            target_transparency == Some(true),
            target_visual_xid,
        );

        match (lacks_what, &this_lacks_what) {
            (Some(Ok(())), _) => unreachable!(),

            // Found it.
            (_, Ok(())) => {
                chosen_conf_id = Some((conf_id, visual_infos));
                lacks_what = Some(this_lacks_what);
                break;
            }

            // Better have something than nothing.
            (None, _) => {
                chosen_conf_id = Some((conf_id, visual_infos));
                lacks_what = Some(this_lacks_what);
            }

            // Stick with the earlier.
            (Some(Err(Lacks::Transparency)), Err(Lacks::Transparency)) => (),
            (Some(Err(_)), Err(Lacks::XID)) => (),

            // Lacking transparency is better than lacking the xid.
            (Some(Err(Lacks::XID)), Err(Lacks::Transparency)) => {
                chosen_conf_id = Some((conf_id, visual_infos));
                lacks_what = Some(this_lacks_what);
            }
        }
    }

    match lacks_what {
        Some(Ok(())) => (),
        Some(Err(Lacks::Transparency)) => warn!(
            "[glutin] could not a find fb config with an alpha mask. Transparency may be broken."
        ),
        Some(Err(Lacks::XID)) => panic!(),
        None => warn!("[glutin] no configs were found. Period."),
    }

    chosen_conf_id.ok_or(errors)
}

//    Get the screen_id for the window being built.
//    let screen_id = unsafe { (xconn.xlib.XDefaultScreen)(xconn.display) };
//
//    let mut builder_glx_u = None;
//    let mut builder_egl_u = None;
//
//    finish creating the OpenGL context
//    let context = match context {
//    Prototype::Glx(ctx) => {
//    X11Context::Glx(ctx.finish_pbuffer(size)?)
//    }
//    Prototype::Egl(ctx) => {
//    X11Context::Egl(ctx.finish_pbuffer(size)?)
//    }
//    };
//
//    let context = Context::PBuffer(ContextInner {
//    xconn: Arc::clone(&xconn),
//    context,
//    });
//
//
//    Prototype::Glx(ctx) =>
//    X11Context::Glx(ctx.finish_surfaceless(xwin)?),
//    Prototype::Egl(ctx) => {
//    X11Context::Egl(ctx.finish_surfaceless()?)
//    }
//    _ => unimplemented!(),
//    };
//
//        let select_conf = |cs, display| {
//            select_conf(&xconn, transparent, cb, cs, |conf_id| {
//                let xid = egl::get_native_visual_id(display, *conf_id)
//                    as ffi::VisualID;
//                if xid == 0 {
//                    return None;
//                }
//                Some(x11_utils::get_visual_info_from_xid(xconn, xid))
//            })
//            .map(|(c, _)| c)
//        };
//        Ok(match cb.gl_attr.version {
//            GlRequest::Latest
//            | GlRequest::Specific(Api::OpenGl, _)
//            | GlRequest::GlThenGles { .. } => {
//                // GLX should be preferred over EGL, otherwise crashes may occur
//                // on X11 – issue #314
//                //
//                // However, with surfaceless, GLX isn't really there, so we
//                // should prefer EGL.
//                let glx = |builder_u: &'a mut Option<_>| {
//                    let builder = cb.clone();
//                    *builder_u =
//                        Some(builder.map_sharing(|c| match c.context {
//                            X11Context::Glx(ref c) => c,
//                            _ => panic!(),
//                        }));
//                    Ok(Prototype::Glx(GlxContext::new(
//                        Arc::clone(&xconn),
//                        builder_u.as_ref().unwrap(),
//                        screen_id,
//                        surface_type,
//                        transparent,
//                    )?))
//                };
//
//                let egl = |builder_u: &'a mut Option<_>| {
//                    let builder = cb.clone();
//                    *builder_u =
//                        Some(builder.map_sharing(|c| match c.context {
//                            X11Context::Egl(ref c) => c,
//                            _ => panic!(),
//                        }));
//                    let native_display =
//                        NativeDisplay::X11(Some(xconn.display as *const _));
//                    Ok(Prototype::Egl(EglContext::new(
//                        builder_u.as_ref().unwrap(),
//                        native_display,
//                        surface_type,
//                        select_conf,
//                    )?))
//                };
//
//
//
//        // Get the screen_id for the window being built.
//        let screen_id = unsafe { (syms!(XLIB).XDefaultScreen)(xconn.display) };
//
//
//        // getting the `visual_infos` (a struct that contains information about
//        // the visual to use)
//        let visual_infos = match context {
//            Prototype::Glx(ref p) => p.get_visual_infos().clone(),
//            Prototype::Egl(ref p) => utils::get_visual_info_from_xid(
//                &xconn,
//                p.get_native_visual_id() as ffi::VisualID,
//            ),
//        };
//
//        let win = wb
//            .with_x11_visual(&visual_infos as *const _)
//            .with_x11_screen(screen_id)
//            .build(el)?;
//
//        let xwin = win.xlib_window().unwrap();
//        // finish creating the OpenGL context
//        let context = match context {
//            Prototype::Glx(ctx) => X11Context::Glx(ctx.finish(xwin)?),
//            Prototype::Egl(ctx) => X11Context::Egl(ctx.finish(xwin as _)?),
//        };
//
//        let context = Context::Windowed(ContextInner {
//            xconn: Arc::clone(&xconn),
//            context,
//        });
//
//        Ok((win, context))
//    }
//
//    fn new_raw_context_impl(
//    xconn: &Arc<XConnection>,
//    xwin: raw::c_ulong,
//    pf_reqs: &PixelFormatRequirements,
//    gl_attr: &GlAttributes<&Context>,
//    plat_attr: &ContextPlatformAttributes,
//    fallback: bool,
//    ) -> Result<Self, CreationError> {
//    let attrs = unsafe {
//    let mut attrs = 0;
//    (xconn.xlib.XGetWindowAttributes)(xconn.display, xwin, &mut attrs);
//    attrs
//    };
//
//    // Not particularly efficient, but it's the only method I can find.
//    let mut screen_id = 0;
//    unsafe {
//    while attrs.screen
//    != (xconn.xlib.XScreenOfDisplay)(xconn.display, screen_id)
//    {
//    screen_id += 1;
//    }
//    }
//
//    let attrs = {
//    let mut attrs = 0;
//    unsafe {
//    (xconn.xlib.XGetWindowAttributes)(
//    xconn.display,
//    xwin,
//    &mut attrs,
//    );
//    }
//    attrs
//    };
//
//    let visual_xid =
//    unsafe { (xconn.xlib.XVisualIDFromVisual)(attrs.visual) };
//    let mut pf_reqs = pf_reqs.clone();
//    let mut plat_attr = plat_attr.clone();
//    plat_attr.x11_visual_xid = Some(visual_xid);
//    pf_reqs.depth_bits = Some(attrs.depth as _);
//
