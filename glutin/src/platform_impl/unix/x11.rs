use crate::api::egl::{self, EGL};
use crate::api::glx::{self, ffi, GLX};
use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::ContextBuilderWrapper;
use crate::display::DisplayBuilder;
use crate::platform_impl::BackingApi;
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};

use glutin_interface::inputs::{
    NativeDisplay, NativePixmap, NativePixmapBuilder, NativeWindow, NativeWindowBuilder,
    RawDisplay, RawWindow,
};
use glutin_x11_sym::Display as X11Display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::fmt::Debug;
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
            (Err(mut err1), Err(err2)) => Err({ err1.append(err2); err1 }),
        }
    }
}

#[derive(Debug)]
pub enum Config {
    Egl(egl::Config),
    //Glx(glx::Display),
}

impl Config {
    #[inline]
    pub fn new(disp: &Display, cb: ConfigBuilder) -> Result<Vec<(ConfigAttribs, Self)>, Error> {
        let plat_attr = cb.plat_attr.clone();
        Ok(match disp.display {
            BackendDisplay::Egl(ref bdisp) => {
                let configs = egl::Config::new(bdisp, cb, |confs| {
                    select_configs(
                        &disp.native_display,
                        plat_attr.x11_transparency,
                        plat_attr.x11_visual_xid,
                        confs,
                        |config_id| {
                            let xid =
                                egl::get_native_visual_id(***bdisp, *config_id) as ffi::VisualID;
                            utils::get_visual_info_from_xid(&disp.native_display, xid)
                                .map(|vis| (vis, xid))
                        },
                    )
                    .into_iter()
                    .map(|config| config.map(|(conf, _)| conf))
                    .collect()
                })?;
                configs
                    .into_iter()
                    .map(|(attribs, config)| (attribs, Config::Egl(config)))
                    .collect()
            }
        })
    }

    #[inline]
    pub fn get_visual_info(
        &self,
        native_display: &Arc<X11Display>,
    ) -> Result<ffi::XVisualInfo, Error> {
        match self {
            Config::Egl(conf) => utils::get_visual_info_from_xid(
                native_display,
                conf.get_native_visual_id() as ffi::VisualID,
            ),
        }
    }
}

#[derive(Debug)]
pub enum Context {
    Egl(egl::Context),
    //Glx(glx::Display),
}

impl Context {
    #[inline]
    pub(crate) fn new(
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        match (&disp.display, &conf.config) {
            (BackendDisplay::Egl(disp), Config::Egl(config)) => egl::Context::new(
                disp,
                Context::inner_cb_egl(cb)?,
                conf.map_config(|_| config),
            )
            .map(Context::Egl),
            //(BackendDisplay::Glx(disp), Config::Glx(config)) => {
            //    glx::Context::new(
            //        disp,
            //        Context::inner_cb_glx(cb)?,
            //        conf.map_config(|_| config),
            //    )
            //    .map(Context::Glx)
            //},
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible display and config backends.".to_string()
            ))),
        }
    }

    fn inner_cb_egl(
        cb: ContextBuilderWrapper<&Context>,
    ) -> Result<ContextBuilderWrapper<&egl::Context>, Error> {
        match cb.sharing {
            Some(Context::Egl(_)) | None => (),
            _ => {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "Cannot share a EGL context with a non-EGL context".to_string()
                )))
            }
        }

        Ok(cb.map_sharing(|ctx| match ctx {
            Context::Egl(ctx) => ctx,
            _ => unreachable!(),
        }))
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        match self {
            Context::Egl(ref ctx) => ctx.make_current_surfaceless(),
            //Context::Glx(ref ctx) => ctx.make_current_surfaceless(),
        }
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        match (self, surf) {
            (Context::Egl(ref ctx), Surface::Egl(ref surf)) => ctx.make_current(surf),
            //(Context::Glx(ref ctx), Surface::Glx(ref surf)) => ctx.make_current(surf),
            (_, _) => Err(make_error!(ErrorType::BadApiUsage(
                "Incompatible context and surface backends.".to_string()
            ))),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Context::Egl(ref ctx) => ctx.make_not_current(),
            //Context::Glx(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Context::Egl(ref ctx) => ctx.is_current(),
            //Context::Glx(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self {
            Context::Egl(ref ctx) => ctx.get_api(),
            //Context::Glx(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const raw::c_void {
        match self {
            Context::Egl(ref ctx) => ctx.get_proc_address(addr),
            //Context::Glx(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Context::Egl(ref ctx) => ctx.get_config().map_config(Config::Egl),
            //Context::Glx(ref ctx) => ctx.get_config().map_config(Config::Glx),
        }
    }
}

#[derive(Debug)]
pub enum Surface<T: SurfaceTypeTrait> {
    Egl(egl::Surface<T>),
    //Glx(glx::Display),
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            Surface::Egl(ref surf) => surf.is_current(),
            //Surface::Glx(ref surf) => surf.is_current(),
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        match self {
            Surface::Egl(ref ctx) => ctx.get_config().map_config(Config::Egl),
            //Context::Glx(ref ctx) => ctx.get_config().map_config(Config::Glx),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        match self {
            Surface::Egl(ref surf) => surf.make_not_current(),
            //Surface::Glx(ref surf) => surf.make_not_current(),
        }
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub fn new(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        match (&disp.display, conf.config) {
            (BackendDisplay::Egl(disp), Config::Egl(config)) => {
                egl::Surface::<PBuffer>::new(disp, conf.map_config(|_| config), size)
                    .map(Surface::Egl)
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

impl Surface<Window> {
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
        let visual_info = conf.config.get_visual_info(&disp.native_display);
        let nw = nwb.build_x11(&visual_info as *const _ as *const _, screen)?;
        Self::new_existing(disp, conf, &nw).map(|surf| (nw, surf))
    }

    #[inline]
    pub fn new_existing<NW: NativeWindow>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        let xlib = syms!(XLIB);
        let surface = nw.raw_window();
        let surface = match surface {
            RawWindow::Xlib { window, .. } => window,
            _ => unreachable!(),
        };

        let visual_info = conf.config.get_visual_info(&disp.native_display)?;
        let window_attrs = {
            let mut window_attrs = unsafe { std::mem::zeroed() };
            let window_attr_error = make_oserror!(OsError::Misc(
                "Glutin failed to query for a window's window attributes.".to_string()
            ));
            disp.native_display
                .check_errors()
                .map_err(|mut err| { err.append(window_attr_error.clone()); err})?;
            if unsafe {
                (xlib.XGetWindowAttributes)(**disp.native_display, surface, &mut window_attrs)
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

        match (&disp.display, &conf.config) {
            (BackendDisplay::Egl(disp), Config::Egl(config)) => {
                egl::Surface::<Window>::new(disp, conf.map_config(|_| config), surface as *const _)
                    .map(|surf| Surface::Egl(surf))
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
    pub fn swap_buffers(&self) -> Result<(), Error> {
        match self {
            Surface::Egl(ref surf) => surf.swap_buffers(),
            //Surface::Glx(ref surf) => surf.swap_buffers(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        match self {
            Surface::Egl(ref surf) => surf.swap_buffers_with_damage(rects),
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
    native_disp: &Arc<X11Display>,
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
                native_disp,
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
//        let select_configs = |cs, display| {
//            select_configs(&xconn, transparent, cb, cs, |conf_id| {
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
//                        select_configs,
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
