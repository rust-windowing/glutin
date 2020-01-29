#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod ffi;
mod glx;

pub use self::glx::{Glx, GlxExtra};

use crate::config::{
    Api, ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapInterval, SwapIntervalRange,
};
use crate::context::{ContextBuilderWrapper, GlProfile, ReleaseBehaviour, Robustness};
use crate::surface::{PBuffer, Pixmap, SurfaceType, SurfaceTypeTrait, Window};
use crate::utils::NoCmp;

use glutin_x11_sym::Display as X11Display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw;
use std::slice;
use std::sync::Arc;

lazy_static! {
    pub static ref GLX: Result<Glx, Error> = Glx::new();
    pub static ref GLX_EXTRA: Result<GlxExtra, Error> = GLX
        .as_ref()
        .map(|glx| GlxExtra::new(glx))
        .map_err(|err| err.clone());
}

#[derive(Debug, PartialEq, Eq)]
pub struct Display {
    display: Arc<X11Display>,
    screen: raw::c_int,
    extensions: Vec<String>,
    version: (u8, u8),
}

impl Display {
    #[inline]
    pub fn new(screen: raw::c_int, display: &Arc<X11Display>) -> Result<Arc<Self>, Error> {
        let glx = GLX.as_ref().unwrap();
        // This is completely ridiculous, but VirtualBox's OpenGL driver needs
        // some call handled by *it* (i.e. not Mesa) to occur before
        // anything else can happen. That is because VirtualBox's OpenGL
        // driver is going to apply binary patches to Mesa in the DLL
        // constructor and until it's loaded it won't have a chance to do that.
        //
        // The easiest way to do this is to just call `glXQueryVersion()` before
        // doing anything else. See: https://www.virtualbox.org/ticket/8293
        let (mut major, mut minor) = (0, 0);
        unsafe {
            glx.QueryVersion(***display as *mut _, &mut major, &mut minor);
        }
        let version = (major as _, minor as _);

        if version < (1, 3) {
            return Err(make_error!(ErrorType::NotSupported(
                "Glutin does not support GLX versions older than 1.3. GLX 1.3 was released in 1997. You've had plenty time to upgrade :D".to_string(),
            )));
        }

        // loading the list of extensions
        let extensions = Self::load_extensions(display, screen)?
            .split(' ')
            .map(|e| e.to_string())
            .collect::<Vec<_>>();

        Ok(Arc::new(Display {
            display: Arc::clone(display),
            screen,
            extensions,
            version,
        }))
    }

    #[inline]
    fn load_extensions(disp: &Arc<X11Display>, screen: raw::c_int) -> Result<String, Error> {
        unsafe {
            let glx = GLX.as_ref().unwrap();
            let extensions = glx.QueryExtensionsString(***disp as *mut _, screen);
            if extensions.is_null() {
                return Err(make_oserror!(OsError::Misc(
                    "`glXQueryExtensionsString` found no glX extensions".to_string(),
                )));
            }
            let extensions = CStr::from_ptr(extensions).to_bytes().to_vec();
            Ok(String::from_utf8(extensions).unwrap())
        }
    }

    #[inline]
    fn has_extension(&self, e: &str) -> bool {
        self.extensions.iter().find(|s| s == &e).is_some()
    }
}

impl Deref for Display {
    type Target = Arc<X11Display>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

// FIXME why is this a macro again?
macro_rules! attrib {
    ($glx:expr, $disp:expr, $conf:expr, $attr:expr $(,)?) => {{
        let mut value = 0;
        let res = unsafe {
            $glx.GetFBConfigAttrib(****$disp as *mut _, $conf, $attr as raw::c_int, &mut value)
        };
        match res {
            0 => Ok(value),
            err => Err(make_oserror!(OsError::Misc(format!(
                "glxGetFBConfigAttrib failed for {:?} with 0x{:x}",
                $conf, err,
            )))),
        }
    }};
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    display: Arc<Display>,
    config: ffi::glx::types::GLXFBConfig,
    visual_info: NoCmp<ffi::XVisualInfo>,
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

impl Config {
    #[inline]
    pub fn new<F>(
        cf: &ConfigsFinder,
        screen: raw::c_int,
        disp: &Arc<X11Display>,
        mut conf_selector: F,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error>
    where
        F: FnMut(
            Vec<ffi::glx::types::GLXFBConfig>,
        ) -> Vec<Result<(ffi::glx::types::GLXFBConfig, ffi::XVisualInfo), Error>>,
    {
        let xlib = syms!(XLIB);
        let glx = GLX.as_ref().map_err(|e| e.clone())?;
        let disp = Display::new(screen, disp)?;
        let mut errors = make_error!(ErrorType::NoAvailableConfig);

        if cf.must_support_surfaceless {
            errors.append(make_error!(ErrorType::NotSupported(
                "EGL surfaceless not supported with GLX".to_string(),
            )));
            return Err(errors);
        }

        if cf.version.0 != Api::OpenGl {
            errors.append(make_error!(ErrorType::OpenGlVersionNotSupported));
            return Err(errors);
        }

        let descriptor = {
            let mut out: Vec<raw::c_int> = Vec::with_capacity(37);

            out.push(ffi::glx::X_RENDERABLE as raw::c_int);
            out.push(1);

            if let Some(xid) = cf.plat_attr.x11_visual_xid {
                // getting the visual infos
                let fvi = crate::platform_impl::x11::utils::get_visual_info_from_xid(&**disp, xid)?;

                out.push(ffi::glx::X_VISUAL_TYPE as raw::c_int);
                out.push(fvi.class as raw::c_int);

                out.push(ffi::glx::VISUAL_ID as raw::c_int);
                out.push(xid as raw::c_int);
            } else {
                out.push(ffi::glx::X_VISUAL_TYPE as raw::c_int);
                out.push(ffi::glx::TRUE_COLOR as raw::c_int);
            }

            out.push(ffi::glx::DRAWABLE_TYPE as raw::c_int);
            let mut surface_type = 0;
            if cf.must_support_windows {
                surface_type = surface_type | ffi::glx::WINDOW_BIT;
            }
            if cf.must_support_pbuffers {
                surface_type = surface_type | ffi::glx::PBUFFER_BIT;
            }
            if cf.must_support_pixmaps {
                surface_type = surface_type | ffi::glx::PIXMAP_BIT;
            }
            out.push(surface_type as raw::c_int);

            if let Some(color) = cf.color_bits {
                out.push(ffi::glx::RED_SIZE as raw::c_int);
                out.push((color / 3) as raw::c_int);
                out.push(ffi::glx::GREEN_SIZE as raw::c_int);
                out.push((color / 3 + if color % 3 != 0 { 1 } else { 0 }) as raw::c_int);
                out.push(ffi::glx::BLUE_SIZE as raw::c_int);
                out.push((color / 3 + if color % 3 == 2 { 1 } else { 0 }) as raw::c_int);
            }

            if let Some(alpha) = cf.alpha_bits {
                out.push(ffi::glx::ALPHA_SIZE as raw::c_int);
                out.push(alpha as raw::c_int);
            }

            match cf.float_color_buffer {
                Some(true) => {
                    if !disp.has_extension("GLX_ARB_fbconfig_float") {
                        errors.append(make_error!(ErrorType::FloatingPointSurfaceNotSupported));
                        return Err(errors);
                    }
                    out.push(ffi::glx::RENDER_TYPE as raw::c_int);
                    out.push(ffi::glx_extra::RGBA_FLOAT_BIT_ARB as raw::c_int);
                }
                Some(false) => {
                    out.push(ffi::glx::RENDER_TYPE as raw::c_int);
                    out.push(ffi::glx::RGBA_BIT as raw::c_int);
                }
                _ => (),
            }

            if let Some(depth) = cf.depth_bits {
                out.push(ffi::glx::DEPTH_SIZE as raw::c_int);
                out.push(depth as raw::c_int);
            }

            if let Some(stencil) = cf.stencil_bits {
                out.push(ffi::glx::STENCIL_SIZE as raw::c_int);
                out.push(stencil as raw::c_int);
            }

            let double_buffer = cf.double_buffer.unwrap_or(true);
            out.push(ffi::glx::DOUBLEBUFFER as raw::c_int);
            out.push(if double_buffer { 1 } else { 0 });

            if let Some(multisampling) = cf.multisampling {
                if disp.version >= (1, 4) {
                    out.push(ffi::glx::SAMPLE_BUFFERS as raw::c_int);
                    out.push(if multisampling == 0 { 0 } else { 1 });
                    out.push(ffi::glx::SAMPLES as raw::c_int);
                    out.push(multisampling as raw::c_int);
                } else if disp.has_extension("GLX_ARB_multisample") {
                    out.push(ffi::glx_extra::SAMPLE_BUFFERS_ARB as raw::c_int);
                    out.push(if multisampling == 0 { 0 } else { 1 });
                    out.push(ffi::glx_extra::SAMPLES_ARB as raw::c_int);
                    out.push(multisampling as raw::c_int);
                } else {
                    errors.append(make_error!(ErrorType::MultisamplingNotSupported));
                    return Err(errors);
                }
            }

            out.push(ffi::glx::STEREO as raw::c_int);
            out.push(if cf.stereoscopy { 1 } else { 0 });

            // The ARB ext says that if we don't pass GLX_FRAMEBUFFER_SRGB_CAPABLE_ARB
            // it is treated as don't care, which is what we want.
            //
            // The ARB ext was ammended to say so in
            // https://github.com/KhronosGroup/OpenGL-Registry/issues/199.
            //
            // The EXT ext doesn't specify, but given that they should both behave
            // (nearly) the same, it is safe to assume that this is also the case
            // for the EXT ext.
            if let Some(srgb) = cf.srgb {
                let srgb = if srgb { 1 } else { 0 };
                if disp.has_extension("GLX_ARB_framebuffer_sRGB") {
                    out.push(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as raw::c_int);
                    out.push(srgb);
                } else if disp.has_extension("GLX_EXT_framebuffer_sRGB") {
                    out.push(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as raw::c_int);
                    out.push(srgb);
                } else {
                    errors.append(make_error!(ErrorType::SrgbSurfaceNotSupported));
                    return Err(make_error!(ErrorType::NoAvailableConfig));
                }
            }

            out.push(ffi::glx::CONFIG_CAVEAT as raw::c_int);
            out.push(ffi::glx::DONT_CARE as raw::c_int);

            out.push(0);
            out
        };

        let swap_control_supported = disp.has_extension("GLX_EXT_swap_control")
            || disp.has_extension("GLX_MESA_swap_control")
            || disp.has_extension("GLX_SGI_swap_control");
        let swap_control_tear_supported = disp.has_extension("GLX_EXT_swap_control_tear");

        if !cf.desired_swap_interval_ranges.is_empty() {
            if !swap_control_supported {
                errors.append(make_error!(ErrorType::SwapControlRangeNotSupported));
                return Err(errors);
            }

            if !swap_control_tear_supported {
                for dsir in &cf.desired_swap_interval_ranges[..] {
                    match dsir {
                        SwapIntervalRange::AdaptiveWait(_) => {
                            errors.append(make_error!(ErrorType::AdaptiveSwapControlNotSupported));
                            errors.append(make_error!(ErrorType::SwapControlRangeNotSupported));
                            return Err(errors);
                        }
                        _ => (),
                    }
                }
            }
        }

        // calling glXChooseFBConfig
        let mut num_confs = 0;
        let confs_ptr = unsafe {
            glx.ChooseFBConfig(
                ****disp as *mut _,
                screen,
                descriptor.as_ptr(),
                &mut num_confs,
            )
        };

        if let Err(err) = disp.check_errors() {
            errors.append(err);
            return Err(errors);
        }

        if confs_ptr.is_null() {
            return Err(errors);
        }

        if num_confs == 0 {
            return Err(errors);
        }

        let confs: Vec<ffi::glx::types::GLXFBConfig> = unsafe {
            let confs = slice::from_raw_parts(confs_ptr, num_confs as usize)
                .iter()
                .cloned()
                .collect();
            (xlib.XFree)(confs_ptr as *mut _);
            confs
        };

        let confs: Vec<_> = conf_selector(confs)
            .into_iter()
            .filter_map(|conf| match conf {
                Err(err) => {
                    errors.append(err);
                    None
                }
                Ok(conf) => Some(conf),
            })
            .map(|(conf, visual_info)| {
                // There is no way for us to know the SwapIntervalRange's max until the
                // drawable is made.
                //
                // 1000 is reasonable.
                let mut swap_interval_ranges = if swap_control_supported {
                    vec![
                        SwapIntervalRange::DontWait,
                        SwapIntervalRange::Wait(1..1000),
                    ]
                } else {
                    vec![]
                };
                if swap_control_tear_supported {
                    swap_interval_ranges.push(SwapIntervalRange::AdaptiveWait(1..1000));
                }

                let surf_type = attrib!(glx, disp, conf, ffi::glx::DRAWABLE_TYPE)? as u32;
                let attribs = ConfigAttribs {
                    version: cf.version,
                    supports_windows: (surf_type & ffi::glx::WINDOW_BIT) != 0,
                    supports_pixmaps: (surf_type & ffi::glx::PIXMAP_BIT) != 0,
                    supports_pbuffers: (surf_type & ffi::glx::PBUFFER_BIT) != 0,
                    supports_surfaceless: false,
                    hardware_accelerated: attrib!(glx, disp, conf, ffi::glx::CONFIG_CAVEAT)?
                        != ffi::glx::SLOW_CONFIG as raw::c_int,
                    color_bits: attrib!(glx, disp, conf, ffi::glx::RED_SIZE)? as u8
                        + attrib!(glx, disp, conf, ffi::glx::BLUE_SIZE)? as u8
                        + attrib!(glx, disp, conf, ffi::glx::GREEN_SIZE)? as u8,
                    alpha_bits: attrib!(glx, disp, conf, ffi::glx::ALPHA_SIZE)? as u8,
                    depth_bits: attrib!(glx, disp, conf, ffi::glx::DEPTH_SIZE)? as u8,
                    stencil_bits: attrib!(glx, disp, conf, ffi::glx::STENCIL_SIZE)? as u8,
                    stereoscopy: attrib!(glx, disp, conf, ffi::glx::STEREO)? != 0,
                    double_buffer: attrib!(glx, disp, conf, ffi::glx::DOUBLEBUFFER)? != 0,
                    // Gets populated later.
                    float_color_buffer: false,

                    multisampling: match disp.has_extension("GLX_ARB_multisample") {
                        false => None,
                        true => match attrib!(glx, disp, conf, ffi::glx::SAMPLE_BUFFERS)? {
                            0 => None,
                            _ => Some(attrib!(glx, disp, conf, ffi::glx::SAMPLES)? as u16),
                        },
                    },
                    srgb: match (
                        disp.has_extension("GLX_ARB_framebuffer_sRGB"),
                        disp.has_extension("GLX_EXT_framebuffer_sRGB"),
                    ) {
                        (true, _) => {
                            attrib!(
                                glx,
                                disp,
                                conf,
                                ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB
                            )? != 0
                        }
                        (_, true) => {
                            attrib!(
                                glx,
                                disp,
                                conf,
                                ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT
                            )? != 0
                        }
                        // Mesa prior to 2017 did not support sRGB contexts. It is sane to assume
                        // that if neither ext is implmented the config is most likely not sRGB.
                        //
                        // Of course, this might not be the case, but without either ext
                        // implemented there is no way for us to tell.
                        (_, _) => false,
                    },
                    swap_interval_ranges,
                };

                Ok((attribs, conf, visual_info))
            })
            // FIXME: Pleasing borrowck. Lokathor demands unrolling this loop.
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|conf| {
                if let Err(err) = conf {
                    errors.append(err);
                    return None;
                }
                let (attribs, conf, visual_info) = conf.unwrap();

                let render_type = match attrib!(glx, disp, conf, ffi::glx::RENDER_TYPE) {
                    Ok(rt) => rt,
                    Err(err) => {
                        errors.append(err);
                        return None;
                    }
                } as u32;

                let wants_floating_point = cf.float_color_buffer != Some(false);
                let wants_standard = cf.float_color_buffer != Some(true);

                let is_floating_point = disp.has_extension("GLX_ARB_fbconfig_float")
                    && (render_type & ffi::glx_extra::RGBA_FLOAT_BIT_ARB) != 0;
                let is_standard = (render_type & ffi::glx::RGBA_BIT) != 0;

                let mut confs = vec![];

                if wants_floating_point && is_floating_point {
                    let mut attribs = attribs.clone();
                    attribs.float_color_buffer = true;
                    confs.push((attribs, conf, visual_info));
                }

                if wants_standard && is_standard {
                    confs.push((attribs, conf, visual_info));
                }

                if confs.is_empty() {
                    errors.append(make_error!(ErrorType::FloatingPointSurfaceNotSupported));
                    return None;
                }

                Some(confs)
            })
            .flat_map(|conf| conf)
            .collect();

        if let Err(err) = disp.check_errors() {
            errors.append(err);
            return Err(errors);
        }

        if confs.is_empty() {
            return Err(errors);
        }

        Ok(confs
            .into_iter()
            .map(|(attribs, config, visual)| {
                (
                    attribs,
                    Config {
                        display: Arc::clone(&disp),
                        config,
                        visual_info: NoCmp(visual),
                    },
                )
            })
            .collect())
    }

    #[inline]
    pub fn display(&self) -> &Arc<X11Display> {
        &*self.display
    }

    #[inline]
    pub fn screen(&self) -> raw::c_int {
        self.display.screen
    }

    #[inline]
    pub fn get_visual_info(&self) -> ffi::XVisualInfo {
        *self.visual_info
    }

    #[inline]
    pub fn raw_config(&self) -> *mut raw::c_void {
        self.config as *mut _
    }
}

#[inline]
pub fn get_native_visual_id(
    disp: &Arc<X11Display>,
    conf: ffi::glx::types::GLXFBConfig,
) -> Result<ffi::VisualID, Error> {
    let glx = GLX.as_ref().unwrap();
    Ok(attrib!(glx, &disp, conf, ffi::glx::VISUAL_ID)? as ffi::VisualID)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Context {
    display: Arc<Display>,
    context: ffi::glx::types::GLXContext,
    config: ConfigWrapper<Config, ConfigAttribs>,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    #[inline]
    pub(crate) fn new(
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Context, Error> {
        let glx = GLX.as_ref().unwrap();
        let glx_extra = GLX_EXTRA.as_ref().unwrap();
        let disp = &conf.config.display;
        let sharing = cb
            .sharing
            .map(|ctx| ctx.context)
            .unwrap_or(std::ptr::null());
        let context = if disp.has_extension("GLX_ARB_create_context") {
            let mut attributes = Vec::with_capacity(9);

            let version = conf.attribs.version.1;
            attributes.push(ffi::glx_extra::CONTEXT_MAJOR_VERSION_ARB as raw::c_int);
            attributes.push(version.0 as raw::c_int);
            attributes.push(ffi::glx_extra::CONTEXT_MINOR_VERSION_ARB as raw::c_int);
            attributes.push(version.1 as raw::c_int);

            if let Some(profile) = cb.profile {
                let flag = match profile {
                    GlProfile::Compatibility => {
                        ffi::glx_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB
                    }
                    GlProfile::Core => ffi::glx_extra::CONTEXT_CORE_PROFILE_BIT_ARB,
                };

                attributes.push(ffi::glx_extra::CONTEXT_PROFILE_MASK_ARB as raw::c_int);
                attributes.push(flag as raw::c_int);
            }

            let flags = {
                let mut flags = 0;

                // robustness
                if disp.has_extension("GLX_ARB_create_context_robustness") {
                    match cb.robustness {
                        Robustness::RobustNoResetNotification => {
                            attributes.push(
                                ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB
                                    as raw::c_int,
                            );
                            attributes
                                .push(ffi::glx_extra::NO_RESET_NOTIFICATION_ARB as raw::c_int);
                            flags =
                                flags | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as raw::c_int;
                        }
                        Robustness::RobustLoseContextOnReset => {
                            attributes.push(
                                ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB
                                    as raw::c_int,
                            );
                            attributes
                                .push(ffi::glx_extra::LOSE_CONTEXT_ON_RESET_ARB as raw::c_int);
                            flags =
                                flags | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as raw::c_int;
                        }
                        Robustness::NoError => {
                            return Err(make_error!(ErrorType::RobustnessNotSupported));
                        }
                        _ => (),
                    }
                } else {
                    match cb.robustness {
                        Robustness::RobustNoResetNotification
                        | Robustness::RobustLoseContextOnReset
                        | Robustness::NoError => {
                            return Err(make_error!(ErrorType::RobustnessNotSupported));
                        }
                        _ => (),
                    }
                }

                if cb.debug {
                    flags = flags | ffi::glx_extra::CONTEXT_DEBUG_BIT_ARB as raw::c_int;
                }

                flags
            };

            attributes.push(ffi::glx_extra::CONTEXT_FLAGS_ARB as raw::c_int);
            attributes.push(flags);

            match cb.release_behavior {
                ReleaseBehaviour::Flush => {
                    if disp.has_extension("GLX_ARB_context_flush_control") {
                        // With how shitty drivers are, never hurts to be explicit
                        attributes.push(ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_ARB as raw::c_int);
                        attributes
                            .push(ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_FLUSH_ARB as raw::c_int);
                    }
                }
                ReleaseBehaviour::None => {
                    if !disp.has_extension("GLX_ARB_context_flush_control") {
                        return Err(make_error!(ErrorType::FlushControlNotSupported));
                    }
                    attributes.push(ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_ARB as raw::c_int);
                    attributes
                        .push(ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_NONE_ARB as raw::c_int);
                }
            }

            attributes.push(0);

            unsafe {
                glx_extra.CreateContextAttribsARB(
                    *****disp as *mut _,
                    conf.config.config as *mut _,
                    sharing,
                    1,
                    attributes.as_ptr(),
                )
            }
        } else {
            match cb.robustness {
                Robustness::RobustNoResetNotification
                | Robustness::RobustLoseContextOnReset
                | Robustness::NoError => {
                    return Err(make_error!(ErrorType::RobustnessNotSupported));
                }
                _ => (),
            }

            unsafe {
                glx.CreateNewContext(
                    *****disp as *mut _,
                    conf.config.config as *mut _,
                    ffi::glx::RGBA_TYPE as _,
                    sharing,
                    1,
                )
            }
        };

        // TODO: If BadMatch, it was either an unsupported sharing or version.
        disp.check_errors()?;

        if context.is_null() {
            return Err(make_oserror!(OsError::Misc(
                "GL context creation failed, no errors generated though".to_string(),
            )));
        }

        Ok(Context {
            display: Arc::clone(&disp),
            config: conf.clone_inner(),
            context,
        })
    }

    #[inline]
    pub(crate) unsafe fn make_current<T: SurfaceTypeTrait>(
        &self,
        surf: &Surface<T>,
    ) -> Result<(), Error> {
        let glx = GLX.as_ref().unwrap();
        let res = glx.MakeCurrent(****self.display as *mut _, surf.surface, self.context);
        Self::check_errors(&self.display, res)
    }

    #[inline]
    pub(crate) unsafe fn make_current_rw<TR: SurfaceTypeTrait, TW: SurfaceTypeTrait>(
        &self,
        read_surf: &Surface<TR>,
        write_surf: &Surface<TW>,
    ) -> Result<(), Error> {
        let glx = GLX.as_ref().unwrap();
        // This is a newer variant of glxMakeCurrent introduced in GLX 1.3 as the older variant was
        // not enough.
        let res = glx.MakeContextCurrent(
            ****self.display as *mut _,
            write_surf.surface,
            read_surf.surface,
            self.context,
        );
        Self::check_errors(&self.display, res)
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        // Should have been caught in src/surface.rs
        unreachable!()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        let glx = GLX.as_ref().unwrap();
        let res = glx.MakeCurrent(****self.display as *mut _, 0, std::ptr::null());
        Self::check_errors(&self.display, res)
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        let glx = GLX.as_ref().unwrap();
        unsafe { glx.GetCurrentContext() == self.context }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.config.clone()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        let glx = GLX.as_ref().unwrap();
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        if self.display.version < (1, 4) {
            return Err(make_error!(ErrorType::NotSupported(
                "Glx does not support glxGetProcAddress on GLX versions older than 1.4. GLX 1.4 was released in 2005. You've had plenty time to upgrade :D".to_string(),
            )));
        }
        let ret = unsafe { glx.GetProcAddress(addr as *const _) as *const _ };
        self.display.check_errors()?;
        Ok(ret)
    }

    #[inline]
    fn check_errors(disp: &Arc<Display>, ret: i32) -> Result<(), Error> {
        disp.check_errors()?;
        if ret == ffi::False {
            return Err(make_oserror!(OsError::Misc(
                "Function failed without generating error.".to_string(),
            )));
        }
        Ok(())
    }

    #[inline]
    pub fn raw_context(&self) -> *mut raw::c_void {
        self.context as *mut _
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            glx.DestroyContext(****self.display as *mut _, self.context);
        }

        self.display.check_errors().unwrap();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Surface<T: SurfaceTypeTrait> {
    display: Arc<Display>,
    surface: ffi::Drawable,
    config: ConfigWrapper<Config, ConfigAttribs>,
    phantom: PhantomData<T>,
}

unsafe impl<T: SurfaceTypeTrait> Send for Surface<T> {}
unsafe impl<T: SurfaceTypeTrait> Sync for Surface<T> {}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            glx.GetCurrentDrawable() == self.surface || glx.GetCurrentReadDrawable() == self.surface
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.config.clone()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        let glx = GLX.as_ref().unwrap();
        let res = glx.MakeCurrent(****self.display as *mut _, 0, std::ptr::null());
        Context::check_errors(&self.display, res)
    }

    #[inline]
    pub fn raw_surface(&self) -> *mut raw::c_void {
        self.surface as *mut _
    }
}

impl Surface<Window> {
    #[inline]
    pub fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nwin: ffi::Window,
    ) -> Result<Self, Error> {
        let glx = GLX.as_ref().unwrap();
        let disp = &conf.config.display;

        let window = unsafe {
            glx.CreateWindow(
                *****disp as *mut _,
                conf.config.config,
                nwin,
                std::ptr::null_mut(),
            )
        };

        disp.check_errors()?;

        Ok(Surface {
            display: Arc::clone(&disp),
            surface: window,
            config: conf.clone_inner(),
            phantom: PhantomData,
        })
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            glx.SwapBuffers(****self.display as *mut _, self.surface);
        }
        self.display.check_errors()
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, _rects: &[dpi::Rect]) -> Result<(), Error> {
        Err(make_error!(ErrorType::NotSupported(
            "Glx does not support swap buffers with damage.".to_string(),
        )))
    }

    #[inline]
    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        let glx_extra = GLX_EXTRA.as_ref().map_err(|e| e.clone())?;
        let glx = GLX.as_ref().unwrap();
        let desired_swap_interval = match swap_interval {
            SwapInterval::DontWait => 0,
            SwapInterval::Wait(n) => n as i32,
            SwapInterval::AdaptiveWait(n) => {
                if !self.display.has_extension("GLX_EXT_swap_control_tear") {
                    return Err(make_error!(ErrorType::AdaptiveSwapControlNotSupported));
                }

                // Note: GLX_EXT_swap_control_tear implies and requires GLX_EXT_swap_control.
                //
                // We just need to pass in the swap_interval as negative
                -(n as i32)
            }
        };

        if self.display.has_extension("GLX_EXT_swap_control")
            && glx_extra.SwapIntervalEXT.is_loaded()
        {
            // this should be the most common extension
            unsafe {
                glx_extra.SwapIntervalEXT(
                    ****self.display as *mut _,
                    self.surface,
                    desired_swap_interval,
                );
            }

            let mut swap = 0;
            unsafe {
                glx.QueryDrawable(
                    ****self.display as *mut _,
                    self.surface,
                    ffi::glx_extra::SWAP_INTERVAL_EXT as i32,
                    &mut swap,
                );
            }

            if swap != (desired_swap_interval.abs()) as u32 {
                return Err(make_oserror!(OsError::Misc(format!(
                    "Couldn't setup vsync: expected interval `{}` but got `{}`",
                    desired_swap_interval, swap
                ))));
            }
        } else if self.display.has_extension("GLX_MESA_swap_control")
            && glx_extra.SwapIntervalMESA.is_loaded()
        {
            unsafe {
                glx_extra.SwapIntervalMESA(desired_swap_interval as u32);
            }
        } else if self.display.has_extension("GLX_SGI_swap_control")
            && glx_extra.SwapIntervalSGI.is_loaded()
        {
            unsafe {
                glx_extra.SwapIntervalSGI(desired_swap_interval);
            }
        } else {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Couldn't find any available swap control extension. This means the config did not support any swap interval ranges.".to_string(),
            )));
        }

        self.display.check_errors()?;

        Ok(())
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: &dpi::PhysicalSize<u32>,
    ) -> Result<Self, Error> {
        let glx = GLX.as_ref().unwrap();
        let size: (u32, u32) = (*size).into();
        let disp = &conf.config.display;

        let attributes: Vec<raw::c_int> = vec![
            ffi::glx::PBUFFER_WIDTH as raw::c_int,
            size.0 as raw::c_int,
            ffi::glx::PBUFFER_HEIGHT as raw::c_int,
            size.1 as raw::c_int,
            0,
        ];

        let pbuffer = unsafe {
            glx.CreatePbuffer(*****disp as *mut _, conf.config.config, attributes.as_ptr())
        };

        disp.check_errors()?;

        Ok(Surface {
            display: Arc::clone(&disp),
            surface: pbuffer,
            config: conf.clone_inner(),
            phantom: PhantomData,
        })
    }
}

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    fn drop(&mut self) {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            match T::surface_type() {
                SurfaceType::Window => glx.DestroyWindow(****self.display as *mut _, self.surface),
                SurfaceType::PBuffer => {
                    glx.DestroyPbuffer(****self.display as *mut _, self.surface)
                }
                SurfaceType::Pixmap => glx.DestroyPixmap(****self.display as *mut _, self.surface),
            }
        }
        self.display.check_errors().unwrap();
    }
}
