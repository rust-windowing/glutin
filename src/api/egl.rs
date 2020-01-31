#![cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod egl;
pub mod ffi;

pub use self::egl::Egl;

use crate::config::{
    Api, ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapInterval, SwapIntervalRange, Version,
};
use crate::context::{ContextBuilderWrapper, ReleaseBehaviour, Robustness};
use crate::surface::{PBuffer, Pixmap, SurfaceType, SurfaceTypeTrait, Window};

use glutin_interface::{NativeDisplay, RawDisplay};
#[cfg(any(
    target_os = "android",
    target_os = "windows",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::convert::TryInto;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw;
use std::sync::Arc;

lazy_static! {
    pub static ref EGL: Result<Egl, Error> = Egl::new();
}

type EglVersion = (ffi::EGLint, ffi::EGLint);

#[derive(Debug, PartialEq, Eq)]
pub struct Display {
    display: ffi::EGLDisplay,
    egl_version: EglVersion,
    extensions: Vec<String>,
    client_extensions: Vec<String>,
}

impl Display {
    #[inline]
    fn has_extension(&self, e: &str) -> bool {
        self.extensions.iter().find(|s| s == &e).is_some()
    }

    #[inline]
    fn get_native_display(
        client_extensions: &[String],
        ndisp: &RawDisplay,
    ) -> Result<*const raw::c_void, Error> {
        let egl = EGL.as_ref().unwrap();

        let has_client_extension = |e: &str| client_extensions.iter().find(|s| s == &e).is_some();

        let disp = match *ndisp {
            // Note: Some EGL implementations are missing the
            // `eglGetPlatformDisplay(EXT)` symbol despite reporting
            // `EGL_EXT_platform_base`. I'm pretty sure this is a bug. Therefore we
            // detect whether the symbol is loaded in addition to checking for
            // extensions.
            RawDisplay::Xlib {
                display, screen, ..
            } if has_client_extension("EGL_KHR_platform_x11")
                && egl.GetPlatformDisplay.is_loaded() =>
            {
                let attrib_list = screen.map(|screen| {
                    [
                        ffi::egl::PLATFORM_X11_SCREEN_KHR as ffi::EGLAttrib,
                        screen as ffi::EGLAttrib,
                        ffi::egl::NONE as ffi::EGLAttrib,
                    ]
                });
                unsafe {
                    egl.GetPlatformDisplay(
                        ffi::egl::PLATFORM_X11_KHR,
                        display as *mut _,
                        attrib_list
                            .as_ref()
                            .map(|list| list.as_ptr())
                            .unwrap_or(std::ptr::null()),
                    )
                }
            }

            RawDisplay::Xlib {
                display, screen, ..
            } if has_client_extension("EGL_EXT_platform_x11")
                && egl.GetPlatformDisplayEXT.is_loaded() =>
            {
                let attrib_list = screen.map(|screen| {
                    [
                        ffi::egl::PLATFORM_X11_SCREEN_EXT as ffi::EGLint,
                        screen as ffi::EGLint,
                        ffi::egl::NONE as ffi::EGLint,
                    ]
                });
                unsafe {
                    egl.GetPlatformDisplayEXT(
                        ffi::egl::PLATFORM_X11_EXT,
                        display as *mut _,
                        attrib_list
                            .as_ref()
                            .map(|list| list.as_ptr())
                            .unwrap_or(std::ptr::null()),
                    )
                }
            }

            RawDisplay::Gbm { gbm_device, .. }
                if has_client_extension("EGL_KHR_platform_gbm")
                    && egl.GetPlatformDisplay.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_GBM_KHR,
                    gbm_device.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *mut _) as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::Gbm { gbm_device, .. }
                if has_client_extension("EGL_MESA_platform_gbm")
                    && egl.GetPlatformDisplayEXT.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplayEXT(
                    ffi::egl::PLATFORM_GBM_KHR,
                    gbm_device.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *mut _) as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::Wayland { wl_display, .. }
                if has_client_extension("EGL_KHR_platform_wayland")
                    && egl.GetPlatformDisplay.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_WAYLAND_KHR,
                    wl_display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *mut _) as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::Wayland { wl_display, .. }
                if has_client_extension("EGL_EXT_platform_wayland")
                    && egl.GetPlatformDisplayEXT.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplayEXT(
                    ffi::egl::PLATFORM_WAYLAND_EXT,
                    wl_display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *mut _) as *mut _,
                    std::ptr::null(),
                )
            }

            // TODO: This will never be reached right now, as the android egl
            // bindings use the static generator, so can't rely on
            // GetPlatformDisplay(EXT).
            RawDisplay::Android { .. }
                if has_client_extension("EGL_KHR_platform_android")
                    && egl.GetPlatformDisplay.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_ANDROID_KHR,
                    ffi::egl::DEFAULT_DISPLAY as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::EglExtDevice { egl_device_ext, .. }
                if has_client_extension("EGL_EXT_platform_device")
                    && egl.GetPlatformDisplay.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_DEVICE_EXT,
                    egl_device_ext as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::EglExtDevice { egl_device_ext, .. }
                if has_client_extension("EGL_EXT_platform_device")
                    && egl.GetPlatformDisplayEXT.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplayEXT(
                    ffi::egl::PLATFORM_DEVICE_EXT,
                    egl_device_ext as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::EglMesaSurfaceless { .. }
                if has_client_extension("EGL_MESA_platform_surfaceless")
                    && egl.GetPlatformDisplay.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_SURFACELESS_MESA,
                    ffi::egl::DEFAULT_DISPLAY as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::EglMesaSurfaceless { .. }
                if has_client_extension("EGL_MESA_platform_surfaceless")
                    && egl.GetPlatformDisplayEXT.is_loaded() =>
            unsafe {
                egl.GetPlatformDisplayEXT(
                    ffi::egl::PLATFORM_SURFACELESS_MESA,
                    ffi::egl::DEFAULT_DISPLAY as *mut _,
                    std::ptr::null(),
                )
            }

            RawDisplay::Gbm {
                gbm_device: Some(display),
                ..
            }
            | RawDisplay::Xlib {
                display,
                screen: None,
                ..
            }
            | RawDisplay::Windows {
                hwnd: Some(display),
                ..
            } => unsafe { egl.GetDisplay(display as *mut _) },

            RawDisplay::Android { .. } | RawDisplay::Windows { hwnd: None, .. } => unsafe {
                egl.GetDisplay(ffi::egl::DEFAULT_DISPLAY as *mut _)
            },

            _ => {
                return Err(make_error!(ErrorType::NotSupported(
                    "Display type unsupported by glutin.".to_string(),
                )));
            }
        };

        match disp {
            ffi::egl::NO_DISPLAY => Err(make_oserror!(OsError::Misc(format!(
                "Creating EGL display failed with 0x{:x}",
                unsafe { egl.GetError() },
            )))),
            disp => Ok(disp),
        }
    }

    #[inline]
    fn get_egl_version(disp: ffi::EGLDisplay) -> Result<EglVersion, Error> {
        unsafe {
            let egl = EGL.as_ref().unwrap();
            let mut major: ffi::EGLint = 0;
            let mut minor: ffi::EGLint = 0;

            if egl.Initialize(disp, &mut major, &mut minor) == ffi::egl::FALSE {
                return Err(make_oserror!(OsError::Misc(format!(
                    "eglInitialize failed with 0x{:x}",
                    egl.GetError()
                ))));
            }

            Ok((major, minor))
        }
    }

    #[inline]
    pub fn new<ND: NativeDisplay>(nd: &ND) -> Result<Arc<Self>, Error> {
        let egl = EGL.as_ref().map_err(|err| err.clone())?;

        // the first step is to query the list of extensions without any display, if
        // supported
        let client_extensions = unsafe {
            let p = egl.QueryString(ffi::egl::NO_DISPLAY, ffi::egl::EXTENSIONS as i32);

            // this possibility is available only with EGL 1.5 or
            // EGL_EXT_platform_base, otherwise `eglQueryString` returns an
            // error
            if p.is_null() {
                vec![]
            } else {
                let p = CStr::from_ptr(p);
                let list = String::from_utf8(p.to_bytes().to_vec()).unwrap_or_else(|_| format!(""));
                list.split(' ').map(|e| e.to_string()).collect::<Vec<_>>()
            }
        };

        // calling `eglGetDisplay` or equivalent
        let disp = Self::get_native_display(&client_extensions, &nd.raw_display())?;

        let egl_version = Self::get_egl_version(disp)?;

        // the list of extensions supported by the client once initialized is
        // different from the list of extensions obtained earlier
        let extensions = if egl_version >= (1, 2) {
            let p = unsafe { egl.QueryString(disp, ffi::egl::EXTENSIONS as i32) };
            if p.is_null() {
                return Err(make_oserror!(OsError::Misc(format!(
                    "Querying for EGL extensions failed with 0x{:x}",
                    unsafe { egl.GetError() },
                ))));
            }

            let p = unsafe { CStr::from_ptr(p) };
            let list = String::from_utf8(p.to_bytes().to_vec()).unwrap_or_else(|_| format!(""));
            list.split(' ').map(|e| e.to_string()).collect::<Vec<_>>()
        } else {
            vec![]
        };

        Ok(Arc::new(Display {
            display: disp,
            extensions,
            client_extensions,
            egl_version,
        }))
    }

    #[inline]
    unsafe fn bind_api(api: Api, egl_version: EglVersion) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();
        if egl_version >= (1, 2) {
            if match api {
                Api::OpenGl if egl_version >= (1, 4) => egl.BindAPI(ffi::egl::OPENGL_API),
                Api::OpenGl => ffi::egl::FALSE,
                Api::OpenGlEs if egl_version >= (1, 2) => egl.BindAPI(ffi::egl::OPENGL_ES_API),
                Api::OpenGlEs => ffi::egl::TRUE,
                _ => ffi::egl::FALSE,
            } == ffi::egl::FALSE
            {
                return Err(make_error!(ErrorType::OpenGlVersionNotSupported));
            }
        }

        Ok(())
    }
}

impl Deref for Display {
    type Target = ffi::EGLDisplay;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl Drop for Display {
    #[inline]
    fn drop(&mut self) {
        // In a reasonable world, we could uncomment the line bellow.
        //
        // This is no such world. Lets talk about something.
        //
        // You see, every call to `get_native_display` returns the exact
        // same display, just look at the docs:
        //
        // "Multiple calls made to eglGetDisplay with the same display_id
        // will return the same EGLDisplay handle."
        //
        // My EGL implementation does not do any ref counting, nor do the
        // EGL docs mention ref counting anywhere. In fact, the docs state
        // that there will be *no effect*, which, in a way, implies no ref
        // counting:
        //
        // "Initializing an already initialized EGL display connection has
        // no effect besides returning the version numbers."
        //
        // So, if we terminate the display, other people who are using it
        // won't be so happy.
        //
        // Well, how did I stumble on this issue you might ask...
        //
        // In this case, the "other people" was us, for it appears my EGL
        // implementation does not follow the docs, or maybe I'm misreading
        // them. You see, according to the egl docs:
        //
        // "To release the current context without assigning a new one, set
        // context to EGL_NO_CONTEXT and set draw and read to
        // EGL_NO_SURFACE.  [...] ******This is the only case where an
        // uninitialized display may be passed to eglMakeCurrent.******"
        // (Emphasis mine).
        //
        // Well, my computer returns EGL_BAD_DISPLAY if the display passed
        // to eglMakeCurrent is uninitialized, which allowed to me to spot
        // this issue.
        //
        // I would have assumed that if EGL was going to provide us with
        // the same EGLDisplay that they'd at least do some ref counting,
        // but they don't.
        //
        // FIXME: Technically we are leaking resources, not much we can do.
        // Yeah, we could have a global static that does ref counting
        // ourselves, but what if some other library is using the display.
        //
        // On unix operating systems, we could preload a little lib that
        // does ref counting on that level, but:
        //      A) What about other platforms?
        //      B) Do you *really* want all glutin programs to preload a
        //      library?
        //      C) Who the hell is going to maintain that?
        //
        // egl.Terminate(**self.display);
    }
}

// FIXME why is this a macro again?
macro_rules! attrib {
    ($egl:expr, $disp:expr, $conf:expr, $attr:expr $(,)?) => {{
        let mut value = 0;
        let res = unsafe { $egl.GetConfigAttrib(**$disp, $conf, $attr as ffi::EGLint, &mut value) };
        match res {
            0 => Err(make_oserror!(OsError::Misc(format!(
                "eglGetConfigAttrib failed for {:?} with 0x{:x}",
                $conf,
                unsafe { $egl.GetError() },
            )))),
            _ => Ok(value),
        }
    }};
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    display: Arc<Display>,
    config: ffi::EGLConfig,
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

impl Config {
    #[inline]
    pub fn new<F, NB: NativeDisplay>(
        cf: &ConfigsFinder,
        nb: &NB,
        mut conf_selector: F,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error>
    where
        F: FnMut(Vec<ffi::EGLConfig>, &Arc<Display>) -> Vec<Result<ffi::EGLConfig, Error>>,
    {
        let egl = EGL.as_ref().map_err(|e| e.clone())?;
        let display = Display::new(nb)?;
        let mut errors = make_error!(ErrorType::NoAvailableConfig);

        // TODO: Alternatively, allow EGL_MESA_platform_surfaceless.
        // FIXME: Also check for the GL_OES_surfaceless_context *CONTEXT*
        // extension
        let supports_surfaceless = display.has_extension("EGL_KHR_surfaceless_context");
        if cf.must_support_surfaceless && !supports_surfaceless {
            return Err(make_error!(ErrorType::SurfaceTypesNotSupported {
                change_surfaceless: true,
                change_window: false,
                change_pixmap: false,
                change_pbuffer: false,
            }));
        }

        let floating_ext_present = display.has_extension("EGL_EXT_pixel_format_float");
        if cf.float_color_buffer == Some(true) {
            if !floating_ext_present {
                errors.append(make_error!(ErrorType::FloatingPointSurfaceNotSupported));
                return Err(errors);
            }
        }

        if cf.stereoscopy == Some(true) {
            errors.append(make_error!(ErrorType::StereoscopyNotSupported));
            return Err(errors);
        }

        if let Some(srgb) = cf.srgb {
            if srgb && !display.has_extension("EGL_KHR_gl_colorspace") {
                errors.append(make_error!(ErrorType::SrgbSurfaceNotSupported));
                return Err(errors);
            }
        }

        match cf.version {
            (Api::OpenGl, _) | (Api::OpenGlEs, Version(2, _)) | (Api::OpenGlEs, Version(3, _)) => {
                if display.egl_version < (1, 3) {
                    errors.append(make_error!(ErrorType::OpenGlVersionNotSupported));
                    return Err(errors);
                }
            }
            (Api::OpenGlEs, Version(1, _)) => (),
            (_, _) => unimplemented!(),
        };

        // binding the right API and choosing the version
        unsafe { Display::bind_api(cf.version.0, display.egl_version)? };

        if cf
            .desired_swap_interval_ranges
            .iter()
            .find(|si| match si {
                SwapIntervalRange::AdaptiveWait(_) => true,
                _ => false,
            })
            .is_some()
        {
            errors.append(make_error!(ErrorType::AdaptiveSwapControlNotSupported));
            errors.append(make_error!(ErrorType::SwapControlRangeNotSupported));
            return Err(errors);
        }

        let mut num_confs = 0;
        if unsafe { egl.GetConfigs(**display, std::ptr::null_mut(), 0, &mut num_confs) } == 0 {
            errors.append(make_oserror!(OsError::Misc(format!(
                "eglChooseConfig failed with 0x{:x}",
                unsafe { egl.GetError() },
            ))));
            return Err(errors);
        }

        if num_confs == 0 {
            return Err(errors);
        }

        let mut confs = Vec::with_capacity(num_confs as usize);
        confs.resize_with(num_confs as usize, || unsafe { std::mem::zeroed() });
        if unsafe { egl.GetConfigs(**display, confs.as_mut_ptr(), num_confs, &mut num_confs) } == 0
        {
            errors.append(make_oserror!(OsError::Misc(format!(
                "eglChooseConfig failed with 0x{:x}",
                unsafe { egl.GetError() },
            ))));
            return Err(errors);
        }

        let conv_range = |sir: &_| match sir {
            SwapIntervalRange::DontWait => 0..1,
            SwapIntervalRange::Wait(r) => r.clone(),
            SwapIntervalRange::AdaptiveWait(_) => unreachable!(),
        };

        let dsir = if cf.desired_swap_interval_ranges.is_empty() {
            None
        } else {
            let mut dsir = conv_range(&cf.desired_swap_interval_ranges[0]);

            for ndsir in &cf.desired_swap_interval_ranges[1..] {
                let ndsir = conv_range(ndsir);
                dsir.start = u32::min(dsir.start, ndsir.start);
                dsir.end = u32::max(dsir.end, ndsir.end);
            }

            Some(dsir)
        };

        let confs: Vec<_> = conf_selector(confs, &display)
            .into_iter()
            .filter_map(|conf| match conf {
                Err(err) => {
                    errors.append(err);
                    None
                }
                Ok(conf) => Some(conf),
            })
            .map(|conf| {
                if display.egl_version >= (1, 2) {
                    let cbt = attrib!(egl, display, conf, ffi::egl::COLOR_BUFFER_TYPE)?;
                    if cbt as u32 != ffi::egl::RGB_BUFFER {
                        return Err(make_oserror!(OsError::Misc(format!(
                            "Got color buffer type of {} for {:?}",
                            cbt, conf,
                        ))));
                    }
                }

                #[cfg(any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd",
                ))]
                {
                    if let Some(xid) = cf.plat_attr.x11_visual_xid {
                        let avid = attrib!(egl, display, conf, ffi::egl::NATIVE_VISUAL_ID)?;
                        if avid != xid.try_into().unwrap() {
                            return Err(make_oserror!(OsError::Misc(format!(
                                "Xid of {} doesn't match requested {} for {:?}",
                                avid, xid, conf,
                            ))));
                        }
                    }
                }

                if let Some(vbit) = match cf.version {
                    (Api::OpenGlEs, Version(3, _)) => Some(ffi::egl::OPENGL_ES3_BIT),
                    (Api::OpenGlEs, Version(2, _)) => Some(ffi::egl::OPENGL_ES2_BIT),
                    (Api::OpenGlEs, Version(1, _)) if display.egl_version >= (1, 3) => {
                        Some(ffi::egl::OPENGL_ES_BIT)
                    }
                    (Api::OpenGlEs, Version(1, _)) => None,
                    (Api::OpenGl, _) => Some(ffi::egl::OPENGL_BIT),
                    (_, _) => unreachable!(),
                } {
                    if attrib!(egl, display, conf, ffi::egl::RENDERABLE_TYPE)? as u32 & vbit != vbit
                        || attrib!(egl, display, conf, ffi::egl::CONFORMANT)? as u32 & vbit != vbit
                    {
                        return Err(make_error!(ErrorType::OpenGlVersionNotSupported));
                    }
                }

                // Try into to panic if value is negative. Never trust the driver.
                let min_swap_interval: u32 =
                    attrib!(egl, display, conf, ffi::egl::MIN_SWAP_INTERVAL)?
                        .try_into()
                        .unwrap();
                let max_swap_interval: u32 =
                    attrib!(egl, display, conf, ffi::egl::MAX_SWAP_INTERVAL)?
                        .try_into()
                        .unwrap();
                // Inclusive to exclusive range
                let max_swap_interval = max_swap_interval + 1;

                assert!(max_swap_interval != min_swap_interval);
                let swap_interval_ranges = match (min_swap_interval, max_swap_interval) {
                    (0, 1) => vec![SwapIntervalRange::DontWait],
                    (0, _) => vec![
                        SwapIntervalRange::Wait(1..max_swap_interval),
                        SwapIntervalRange::DontWait,
                    ],
                    (_, _) => vec![SwapIntervalRange::Wait(
                        min_swap_interval..max_swap_interval,
                    )],
                };

                if let Some(ref dsir) = dsir {
                    if dsir.start < min_swap_interval || dsir.end > max_swap_interval {
                        return Err(make_error!(ErrorType::SwapControlRangeNotSupported));
                    }
                }

                let surf_type = attrib!(egl, display, conf, ffi::egl::SURFACE_TYPE)? as u32;
                let attribs = ConfigAttribs {
                    version: cf.version,
                    supports_windows: (surf_type & ffi::egl::WINDOW_BIT) != 0,
                    supports_pixmaps: (surf_type & ffi::egl::PIXMAP_BIT) != 0,
                    supports_pbuffers: (surf_type & ffi::egl::PBUFFER_BIT) != 0,
                    supports_surfaceless,
                    hardware_accelerated: attrib!(egl, display, conf, ffi::egl::CONFIG_CAVEAT)?
                        != ffi::egl::SLOW_CONFIG as raw::c_int,

                    color_bits: attrib!(egl, display, conf, ffi::egl::RED_SIZE)? as u8
                        + attrib!(egl, display, conf, ffi::egl::BLUE_SIZE)? as u8
                        + attrib!(egl, display, conf, ffi::egl::GREEN_SIZE)? as u8,
                    alpha_bits: attrib!(egl, display, conf, ffi::egl::ALPHA_SIZE)? as u8,
                    depth_bits: attrib!(egl, display, conf, ffi::egl::DEPTH_SIZE)? as u8,
                    stencil_bits: attrib!(egl, display, conf, ffi::egl::STENCIL_SIZE)? as u8,
                    float_color_buffer: match floating_ext_present {
                        false => false,
                        true => {
                            match attrib!(egl, display, conf, ffi::egl::COLOR_COMPONENT_TYPE_EXT)?
                                as _
                            {
                                ffi::egl::COLOR_COMPONENT_TYPE_FIXED_EXT => false,
                                ffi::egl::COLOR_COMPONENT_TYPE_FLOAT_EXT => true,
                                _ => panic!(),
                            }
                        }
                    },
                    stereoscopy: false,
                    multisampling: match attrib!(egl, display, conf, ffi::egl::SAMPLE_BUFFERS)? {
                        0 => None,
                        _ => Some(attrib!(egl, display, conf, ffi::egl::SAMPLES)? as u16),
                    },
                    srgb: cf.srgb.unwrap_or(false),
                    double_buffer: cf.double_buffer.unwrap_or(true),
                    swap_interval_ranges,
                };

                crate::utils::common_attribs_match(&attribs, cf)?;

                if let Some(float_color_buffer) = cf.float_color_buffer {
                    if float_color_buffer != attribs.float_color_buffer {
                        return Err(make_error!(ErrorType::FloatingPointSurfaceNotSupported));
                    }
                }

                Ok((attribs, conf))
            })
            // FIXME: Pleasing borrowck. Lokathor demands unrolling this loop.
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|conf| {
                if let Err(err) = conf {
                    errors.append(err);
                    return None;
                }
                let (attribs, conf) = conf.unwrap();

                let mut confs = vec![(attribs.clone(), conf)];

                if cf.srgb.is_none() {
                    let mut attribs = attribs.clone();
                    attribs.srgb = true;
                    confs.push((attribs, conf));
                }

                if cf.double_buffer.is_none() {
                    let mut attribs = attribs.clone();
                    attribs.double_buffer = false;
                    confs.push((attribs, conf));
                }

                if cf.double_buffer.is_none() && cf.srgb.is_none() {
                    let mut attribs = attribs.clone();
                    attribs.srgb = true;
                    attribs.double_buffer = false;
                    confs.push((attribs, conf));
                }

                Some(confs)
            })
            .flat_map(|conf| conf)
            .collect();

        if confs.is_empty() {
            return Err(errors);
        }

        Ok(confs
            .into_iter()
            .map(|(attribs, config)| {
                (
                    attribs,
                    Config {
                        display: Arc::clone(&display),
                        config,
                    },
                )
            })
            .collect())
    }

    #[inline]
    pub fn get_native_visual_id(&self) -> Result<ffi::EGLint, Error> {
        get_native_visual_id(**self.display, self.config)
    }

    #[inline]
    pub fn raw_config(&self) -> *mut raw::c_void {
        self.config as *mut _
    }

    #[inline]
    pub fn raw_display(&self) -> *mut raw::c_void {
        **self.display as *mut _
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Context {
    display: Arc<Display>,
    context: ffi::EGLContext,
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
        let display = Arc::clone(&conf.config.display);
        let egl = EGL.as_ref().unwrap();
        let (api, version) = conf.attribs.version;

        unsafe {
            // FIXME: Support mixing apis
            Display::bind_api(api, display.egl_version)?;
        }

        let sharing = match cb.sharing {
            Some(ctx) => ctx.context,
            None => std::ptr::null(),
        };

        let mut context_attributes = Vec::with_capacity(10);
        let mut flags = 0;

        if display.egl_version >= (1, 5)
            || display
                .extensions
                .iter()
                .find(|s| s == &"EGL_KHR_create_context")
                .is_some()
        {
            context_attributes.push(ffi::egl::CONTEXT_MAJOR_VERSION as raw::c_int);
            context_attributes.push(version.0 as raw::c_int);
            context_attributes.push(ffi::egl::CONTEXT_MINOR_VERSION as raw::c_int);
            context_attributes.push(version.1 as raw::c_int);

            // handling robustness
            let supports_robustness = display.egl_version >= (1, 5)
                || display.has_extension("EGL_EXT_create_context_robustness");
            let supports_no_error = display.has_extension("EGL_KHR_create_context_no_error");

            if !match cb.robustness {
                Robustness::NoError => supports_no_error,
                Robustness::RobustLoseContextOnReset | Robustness::RobustNoResetNotification => {
                    supports_robustness
                }
                _ => true,
            } {
                return Err(make_error!(ErrorType::RobustnessNotSupported));
            }

            match cb.robustness {
                Robustness::NoError => {
                    context_attributes.push(ffi::egl::CONTEXT_OPENGL_NO_ERROR_KHR as raw::c_int);
                    context_attributes.push(1);
                }
                Robustness::RobustNoResetNotification => {
                    context_attributes
                        .push(ffi::egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY as raw::c_int);
                    context_attributes.push(ffi::egl::NO_RESET_NOTIFICATION as raw::c_int);
                    flags = flags | ffi::egl::CONTEXT_OPENGL_ROBUST_ACCESS as raw::c_int;
                }
                Robustness::RobustLoseContextOnReset => {
                    context_attributes
                        .push(ffi::egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY as raw::c_int);
                    context_attributes.push(ffi::egl::LOSE_CONTEXT_ON_RESET as raw::c_int);
                    flags = flags | ffi::egl::CONTEXT_OPENGL_ROBUST_ACCESS as raw::c_int;
                }
                _ => (),
            }

            if cb.debug {
                if display.egl_version >= (1, 5) {
                    context_attributes.push(ffi::egl::CONTEXT_OPENGL_DEBUG as raw::c_int);
                    context_attributes.push(ffi::egl::TRUE as raw::c_int);
                }

                // TODO: using this flag sometimes generates an error there was a
                // change in the specs that added this flag, so it may not be
                // supported everywhere; however it is not possible to know whether
                // it is supported or not
                //
                // flags = flags | ffi::egl::CONTEXT_OPENGL_DEBUG_BIT_KHR as raw::c_int;
            }

            // In at least some configurations, the Android emulator’s GL
            // implementation advertises support for the
            // EGL_KHR_create_context extension but returns BAD_ATTRIBUTE
            // when CONTEXT_FLAGS_KHR is used.
            if flags != 0 {
                context_attributes.push(ffi::egl::CONTEXT_FLAGS_KHR as raw::c_int);
                context_attributes.push(flags);
            }
        } else if display.egl_version >= (1, 3) && api == Api::OpenGlEs {
            // robustness is not supported
            match cb.robustness {
                Robustness::RobustNoResetNotification | Robustness::RobustLoseContextOnReset => {
                    return Err(make_error!(ErrorType::RobustnessNotSupported));
                }
                _ => (),
            }

            context_attributes.push(ffi::egl::CONTEXT_CLIENT_VERSION as raw::c_int);
            context_attributes.push(version.0 as raw::c_int);
        }

        match cb.release_behavior {
            ReleaseBehaviour::Flush => {
                // FIXME: This isn't a client extension, right?
                if display.has_extension("EGL_KHR_context_flush_control") {
                    // With how shitty drivers are, never hurts to be explicit
                    context_attributes.push(ffi::egl::CONTEXT_RELEASE_BEHAVIOR_KHR as raw::c_int);
                    context_attributes
                        .push(ffi::egl::CONTEXT_RELEASE_BEHAVIOR_FLUSH_KHR as raw::c_int);
                }
            }
            ReleaseBehaviour::None => {
                // FIXME: This isn't a client extension, right?
                if !display.has_extension("EGL_KHR_context_flush_control") {
                    return Err(make_error!(ErrorType::FlushControlNotSupported));
                }
                context_attributes.push(ffi::egl::CONTEXT_RELEASE_BEHAVIOR_KHR as raw::c_int);
                context_attributes.push(ffi::egl::CONTEXT_RELEASE_BEHAVIOR_NONE_KHR as raw::c_int);
            }
        }

        context_attributes.push(ffi::egl::NONE as raw::c_int);

        let context = unsafe {
            egl.CreateContext(
                **display,
                conf.config.config,
                sharing,
                context_attributes.as_ptr(),
            )
        };

        if context.is_null() {
            match unsafe { egl.GetError() } as u32 {
                ffi::egl::BAD_MATCH | ffi::egl::BAD_ATTRIBUTE => {
                    return Err(make_error!(ErrorType::OpenGlVersionNotSupported));
                }
                err => {
                    return Err(make_oserror!(OsError::Misc(format!(
                        "eglCreateContext failed with 0x{:x}",
                        err,
                    ))));
                }
            }
        }

        Ok(Context {
            display,
            context,
            config: conf.clone_inner(),
        })
    }

    #[inline]
    pub(crate) unsafe fn make_current<T: SurfaceTypeTrait>(
        &self,
        surf: &Surface<T>,
    ) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();

        let ret = egl.MakeCurrent(**self.display, surf.surface, surf.surface, self.context);
        Self::check_errors(Some(ret))
    }

    #[inline]
    pub(crate) unsafe fn make_current_rw<TR: SurfaceTypeTrait, TW: SurfaceTypeTrait>(
        &self,
        read_surf: &Surface<TR>,
        write_surf: &Surface<TW>,
    ) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();

        let ret = egl.MakeCurrent(
            **self.display,
            write_surf.surface,
            read_surf.surface,
            self.context,
        );
        Self::check_errors(Some(ret))
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();
        let ret = egl.MakeCurrent(
            **self.display,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_SURFACE,
            self.context,
        );

        Self::check_errors(Some(ret))
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();

        let ret = egl.MakeCurrent(
            **self.display,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_CONTEXT,
        );

        Self::check_errors(Some(ret))
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        let egl = EGL.as_ref().unwrap();
        unsafe { egl.GetCurrentContext() == self.context }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.config.clone()
    }

    // FIXME: Needed for android support.
    // winit doesn't have it, I'll add this back in when it does.
    //
    // // Handle Android Life Cycle.
    // // Android has started the activity or sent it to foreground.
    // // Create a new surface and attach it to the recreated ANativeWindow.
    // // Restore the EGLContext.
    // #[cfg(target_os = "android")]
    // pub unsafe fn on_surface_created(&self, nwin: ffi::EGLNativeWindowType) {
    //     let egl = EGL.as_ref().unwrap();
    //     let mut surface = self.surface.as_ref().unwrap().lock();
    //     if *surface != ffi::egl::NO_SURFACE {
    //         return;
    //     }
    //     *surface = egl.CreateWindowSurface(
    //         **self.display,
    //         self.config,
    //         nwin,
    //         std::ptr::null(),
    //     );
    //     if surface.is_null() {
    //         panic!(
    //             "[glutin] on_surface_created: eglCreateWindowSurface failed with
    // 0x{:x}",             egl.GetError()
    //         )
    //     }
    //     let ret =
    //         egl.MakeCurrent(**self.display, *surface, *surface,
    // self.context);     if ret == 0 {
    //         panic!(
    //             "[glutin] on_surface_created: eglMakeCurrent failed with 0x{:x}",
    //             egl.GetError()
    //         )
    //     }
    // }
    //
    // // Handle Android Life Cycle.
    // // Android has stopped the activity or sent it to background.
    // // Release the surface attached to the destroyed ANativeWindow.
    // // The EGLContext is not destroyed so it can be restored later.
    // #[cfg(target_os = "android")]
    // pub unsafe fn on_surface_destroyed(&self) {
    //     let egl = EGL.as_ref().unwrap();
    //     let mut surface = self.surface.as_ref().unwrap().lock();
    //     if *surface == ffi::egl::NO_SURFACE {
    //         return;
    //     }
    //     let ret = egl.MakeCurrent(
    //         **self.display,
    //         ffi::egl::NO_SURFACE,
    //         ffi::egl::NO_SURFACE,
    //         ffi::egl::NO_CONTEXT,
    //     );
    //     if ret == 0 {
    //         panic!(
    //             "[glutin] on_surface_destroyed: eglMakeCurrent failed with 0x{:x}",
    //             egl.GetError()
    //         )
    //     }
    //
    //     egl.DestroySurface(**self.display, *surface);
    //     *surface = ffi::egl::NO_SURFACE;
    // }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        let egl = EGL.as_ref().unwrap();
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        Ok(unsafe { egl.GetProcAddress(addr) as *const raw::c_void })
    }

    #[inline]
    fn check_errors(ret: Option<u32>) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();
        if ret == Some(ffi::egl::FALSE) || ret == None {
            match unsafe { egl.GetError() } as u32 {
                ffi::egl::SUCCESS if ret == None => Ok(()),
                ffi::egl::CONTEXT_LOST => Err(make_error!(ErrorType::ContextLost)),
                err => Err(make_oserror!(OsError::Misc(format!(
                    "failed (eglGetError returned 0x{:x})",
                    err,
                )))),
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn raw_context(&self) -> *mut raw::c_void {
        self.context as *mut _
    }
}

impl Drop for Context {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let egl = EGL.as_ref().unwrap();

            egl.DestroyContext(**self.display, self.context);
            self.context = ffi::egl::NO_CONTEXT;
        }
    }
}

#[inline]
pub fn get_native_visual_id(
    disp: ffi::EGLDisplay,
    conf: ffi::EGLConfig,
) -> Result<ffi::EGLint, Error> {
    let egl = EGL.as_ref().unwrap();
    attrib!(egl, &&disp, conf, ffi::egl::NATIVE_VISUAL_ID)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Surface<T: SurfaceTypeTrait> {
    display: Arc<Display>,
    surface: ffi::EGLSurface,
    config: ConfigWrapper<Config, ConfigAttribs>,
    phantom: PhantomData<T>,
}

unsafe impl<T: SurfaceTypeTrait> Send for Surface<T> {}
unsafe impl<T: SurfaceTypeTrait> Sync for Surface<T> {}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    fn assemble_desc(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: Option<(&dpi::PhysicalSize<u32>, bool)>,
    ) -> Vec<raw::c_int> {
        let mut out = Vec::new();
        match conf.attribs.srgb {
            false => {
                if conf.config.display.has_extension("EGL_KHR_gl_colorspace") {
                    // With how shitty drivers are, never hurts to be explicit
                    out.push(ffi::egl::GL_COLORSPACE_KHR as raw::c_int);
                    out.push(ffi::egl::GL_COLORSPACE_LINEAR_KHR as raw::c_int);
                }
            }
            true => {
                out.push(ffi::egl::GL_COLORSPACE_KHR as raw::c_int);
                out.push(ffi::egl::GL_COLORSPACE_SRGB_KHR as raw::c_int);
            }
        }

        if T::surface_type() == SurfaceType::Window {
            out.push(ffi::egl::RENDER_BUFFER as raw::c_int);
            match conf.attribs.double_buffer {
                false => {
                    out.push(ffi::egl::SINGLE_BUFFER as raw::c_int);
                }
                true => {
                    out.push(ffi::egl::BACK_BUFFER as raw::c_int);
                }
            }
        }

        if let Some((size, largest)) = size {
            let size: (u32, u32) = (*size).into();
            out.push(ffi::egl::TEXTURE_FORMAT as raw::c_int);
            out.push(match size {
                (0, _) | (_, 0) => ffi::egl::NO_TEXTURE,
                _ if conf.attribs.alpha_bits > 0 => ffi::egl::TEXTURE_RGBA,
                _ => ffi::egl::TEXTURE_RGB,
            } as raw::c_int);
            out.push(ffi::egl::TEXTURE_TARGET as raw::c_int);
            out.push(match size {
                (0, _) | (_, 0) => ffi::egl::NO_TEXTURE,
                _ => ffi::egl::TEXTURE_2D,
            } as raw::c_int);

            out.push(ffi::egl::WIDTH as raw::c_int);
            out.push(size.0 as raw::c_int);
            out.push(ffi::egl::HEIGHT as raw::c_int);
            out.push(size.1 as raw::c_int);

            out.push(ffi::egl::LARGEST_PBUFFER as raw::c_int);
            out.push(if largest {
                ffi::egl::TRUE
            } else {
                ffi::egl::FALSE
            } as raw::c_int)
        }

        out.push(ffi::egl::NONE as raw::c_int);
        out
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        let egl = EGL.as_ref().unwrap();
        unsafe {
            egl.GetCurrentSurface(ffi::egl::DRAW as i32) == self.surface
                || egl.GetCurrentSurface(ffi::egl::READ as i32) == self.surface
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.config.clone()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();

        let ret = egl.MakeCurrent(
            **self.display,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_CONTEXT,
        );

        Context::check_errors(Some(ret))
    }

    #[inline]
    pub fn raw_surface(&self) -> *mut raw::c_void {
        self.surface as *mut _
    }

    #[inline]
    pub fn size(&self) -> Result<dpi::PhysicalSize<u32>, Error> {
        let egl = EGL.as_ref().unwrap();
        let mut width = 0;
        let mut height = 0;

        unsafe {
            Context::check_errors(Some(egl.QuerySurface(
                **self.display,
                self.surface,
                ffi::egl::WIDTH as _,
                &mut width,
            )))?;
            Context::check_errors(Some(egl.QuerySurface(
                **self.display,
                self.surface,
                ffi::egl::HEIGHT as _,
                &mut height,
            )))?;
        }

        Ok(dpi::PhysicalSize::new(width as _, height as _))
    }
}

impl Surface<Window> {
    #[inline]
    pub fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nwin: ffi::EGLNativeWindowType,
    ) -> Result<Self, Error> {
        let display = Arc::clone(&conf.config.display);
        let egl = EGL.as_ref().unwrap();
        let desc = Self::assemble_desc(conf.clone(), None);
        let surface = unsafe {
            let surf = egl.CreateWindowSurface(**display, conf.config.config, nwin, desc.as_ptr());
            if surf.is_null() {
                return Err(make_oserror!(OsError::Misc(format!(
                    "eglCreateWindowSurface failed with 0x{:x}",
                    egl.GetError()
                ))));
            }
            surf
        };

        Context::check_errors(None)?;

        Ok(Surface {
            display,
            config: conf.clone_inner(),
            surface,
            phantom: PhantomData,
        })
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();
        if self.surface == ffi::egl::NO_SURFACE {
            return Err(make_error!(ErrorType::ContextLost));
        }

        let ret = unsafe { egl.SwapBuffers(**self.display, self.surface) };

        Context::check_errors(Some(ret))
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();

        if !egl.SwapBuffersWithDamageKHR.is_loaded()
            || !self
                .display
                .has_extension("EGL_KHR_swap_buffers_with_damage")
        {
            return Err(make_error!(ErrorType::NotSupported(
                "buffer damage not suported".to_string(),
            )));
        }

        if self.surface == ffi::egl::NO_SURFACE {
            return Err(make_error!(ErrorType::ContextLost));
        }

        let mut ffirects: Vec<ffi::EGLint> = Vec::with_capacity(rects.len() * 4);

        for rect in rects {
            ffirects.push(rect.pos.x as ffi::EGLint);
            ffirects.push(rect.pos.y as ffi::EGLint);
            ffirects.push(rect.size.width as ffi::EGLint);
            ffirects.push(rect.size.height as ffi::EGLint);
        }

        let ret = unsafe {
            egl.SwapBuffersWithDamageKHR(
                **self.display,
                self.surface,
                ffirects.as_mut_ptr(),
                rects.len() as ffi::EGLint,
            )
        };

        Context::check_errors(Some(ret))
    }

    #[inline]
    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        let egl = EGL.as_ref().unwrap();
        // Swap interval defaults to 1
        let n = match swap_interval {
            SwapInterval::Wait(n) => n,
            SwapInterval::DontWait => 0,
            SwapInterval::AdaptiveWait(_) => unreachable!(),
        };
        unsafe {
            if egl.SwapInterval(**self.display, n as i32) == ffi::egl::FALSE {
                return Err(make_oserror!(OsError::Misc(format!(
                    "eglSwapInterval failed with 0x{:x}",
                    egl.GetError()
                ))));
            }
        }

        Ok(())
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: &dpi::PhysicalSize<u32>,
        largest: bool,
    ) -> Result<Self, Error> {
        let display = Arc::clone(&conf.config.display);
        let egl = EGL.as_ref().unwrap();

        let desc = Self::assemble_desc(conf.clone(), Some((size, largest)));
        let surf = unsafe {
            let pbuffer = egl.CreatePbufferSurface(**display, conf.config.config, desc.as_ptr());
            if pbuffer.is_null() || pbuffer == ffi::egl::NO_SURFACE {
                return Err(make_oserror!(OsError::Misc(format!(
                    "eglCreatePbufferSurface failed with 0x{:x}",
                    egl.GetError(),
                ))));
            }
            pbuffer
        };

        Ok(Surface {
            display,
            config: conf.clone_inner(),
            surface: surf,
            phantom: PhantomData,
        })
    }
}

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let egl = EGL.as_ref().unwrap();

            egl.DestroySurface(**self.display, self.surface);
            self.surface = ffi::egl::NO_SURFACE;
        }
    }
}
