#![cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#![allow(unused_variables)]

#[cfg(not(target_os = "android"))]
mod egl {
    use super::ffi;
    use crate::api::dlloader::{SymTrait, SymWrapper};
    use libloading;
    use std::sync::Arc;
    use parking_lot::Mutex;

    #[cfg(unix)]
    use libloading::os::unix as libloading_os;
    #[cfg(windows)]
    use libloading::os::windows as libloading_os;

    #[derive(Clone)]
    pub struct Egl(pub SymWrapper<ffi::egl::Egl>);

    /// Because `*const raw::c_void` doesn't implement `Sync`.
    unsafe impl Sync for Egl {}

    type EglGetProcAddressType = libloading_os::Symbol<
        unsafe extern "C" fn(
            *const std::os::raw::c_void,
        ) -> *const std::os::raw::c_void,
    >;

    lazy_static! {
        static ref EGL_GET_PROC_ADDRESS: Arc<Mutex<Option<EglGetProcAddressType>>> =
            Arc::new(Mutex::new(None));
    }

    impl SymTrait for ffi::egl::Egl {
        fn load_with(lib: &libloading::Library) -> Self {
            let f = move |s: &'static str| -> *const std::os::raw::c_void {
                // Check if the symbol is available in the library directly. If
                // it is, just return it.
                match unsafe {
                    lib.get(
                        std::ffi::CString::new(s.as_bytes())
                            .unwrap()
                            .as_bytes_with_nul(),
                    )
                } {
                    Ok(sym) => return *sym,
                    Err(_) => (),
                };

                let mut egl_get_proc_address = (*EGL_GET_PROC_ADDRESS).lock();
                if egl_get_proc_address.is_none() {
                    unsafe {
                        let sym: libloading::Symbol<
                            unsafe extern "C" fn(
                                *const std::os::raw::c_void,
                            )
                                -> *const std::os::raw::c_void,
                        > = lib.get(b"eglGetProcAddress").unwrap();
                        *egl_get_proc_address = Some(sym.into_raw());
                    }
                }

                // The symbol was not available in the library, so ask
                // eglGetProcAddress for it. Note that eglGetProcAddress was
                // only able to look up extension functions prior to EGL 1.5,
                // hence this two-part dance.
                unsafe {
                    (egl_get_proc_address.as_ref().unwrap())(
                        std::ffi::CString::new(s.as_bytes())
                            .unwrap()
                            .as_bytes_with_nul()
                            .as_ptr()
                            as *const std::os::raw::c_void,
                    )
                }
            };

            Self::load_with(f)
        }
    }

    impl Egl {
        pub fn new() -> Result<Self, ()> {
            #[cfg(target_os = "windows")]
            let paths = vec!["libEGL.dll", "atioglxx.dll"];

            #[cfg(not(target_os = "windows"))]
            let paths = vec!["libEGL.so.1", "libEGL.so"];

            SymWrapper::new(paths).map(|i| Egl(i))
        }
    }
}

#[cfg(target_os = "android")]
mod egl {
    use super::ffi;

    #[derive(Clone)]
    pub struct Egl(pub ffi::egl::Egl);

    impl Egl {
        pub fn new() -> Result<Self, ()> {
            Ok(Egl(ffi::egl::Egl))
        }
    }
}

mod make_current_guard;

pub use self::egl::Egl;
use self::make_current_guard::MakeCurrentGuard;

use crate::{
    Api, ContextBuilderWrapper, ContextError, CreationError,
    GlVersion, GlRequest, ConfigBuilder, ConfigWrapper,
    ReleaseBehavior, Robustness, Rect, ConfigAttribs,
};

use glutin_egl_sys as ffi;
use parking_lot::Mutex;
#[cfg(any(
    target_os = "android",
    target_os = "windows",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;

use std::ffi::{c_void, CStr, CString};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::os::raw;
use std::sync::Arc;

impl Deref for Egl {
    type Target = ffi::egl::Egl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Egl {
    fn deref_mut(&mut self) -> &mut ffi::egl::Egl {
        &mut self.0
    }
}

lazy_static! {
    pub static ref EGL: Option<Egl> = Egl::new().ok();
}

/// Specifies the type of display passed as `native_display`.
#[derive(Debug)]
#[allow(dead_code)]
pub enum NativeDisplay {
    /// `None` means `EGL_DEFAULT_DISPLAY`.
    X11(Option<ffi::EGLNativeDisplayType>),
    /// `None` means `EGL_DEFAULT_DISPLAY`.
    Gbm(Option<ffi::EGLNativeDisplayType>),
    /// `None` means `EGL_DEFAULT_DISPLAY`.
    Wayland(Option<ffi::EGLNativeDisplayType>),
    /// `EGL_DEFAULT_DISPLAY` is mandatory for Android.
    Android,
    // TODO: should be `EGLDeviceEXT`
    Device(ffi::EGLNativeDisplayType),
    /// Don't specify any display type. Useful on windows. `None` means
    /// `EGL_DEFAULT_DISPLAY`.
    Other(Option<ffi::EGLNativeDisplayType>),
}

#[derive(Debug)]
pub struct DisplayInternal {
    display: ffi::egl::types::EGLDisplay,
    egl_version: EGLVersion,
    extensions: Vec<String>,
}

#[derive(Debug)]
pub struct Display(Arc<DisplayInternal>);

impl Display {
    pub fn new<TE>(
        el: &EventLoopWindowTarget<TE>,
        ndisp: NativeDisplay,
    ) -> Result<Self, CreationError> {
        let egl = EGL.as_ref().unwrap();
        // calling `eglGetDisplay` or equivalent
        let disp = get_native_display(&ndisp);

        if disp.is_null() {
            return Err(CreationError::OsError(
                "Could not create EGL display object".to_string(),
            ));
        }

        let egl_version = get_egl_version(disp)?;

        // the list of extensions supported by the client once initialized is
        // different from the list of extensions obtained earlier
        let extensions = if egl_version >= (1, 2) {
            let p = unsafe {
                CStr::from_ptr(
                    egl.QueryString(disp, ffi::egl::EXTENSIONS as i32),
                )
            };
            let list = String::from_utf8(p.to_bytes().to_vec())
                .unwrap_or_else(|_| format!(""));
            list.split(' ').map(|e| e.to_string()).collect::<Vec<_>>()
        } else {
            vec![]
        };

        Ok(Display(Arc::new(DisplayInternal {
            display: disp,
            extensions,
            egl_version,
        })))
    }
}

impl Deref for DisplayInternal {
    type Target = ffi::egl::types::EGLDisplay;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl Deref for Display {
    type Target = DisplayInternal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Context {
    display: Arc<DisplayInternal>,
    context: ffi::egl::types::EGLContext,
    config: ConfigWrapper<Config>,
}

#[derive(Debug, Clone)]
struct Config {
    display: Arc<DisplayInternal>,
    config_id: ffi::egl::types::EGLConfig,
    version: Option<GlVersion>,
    api: Api,
}

impl Config {
    #[inline]
    pub fn build<F>(
        disp: &Display,
        cb: ConfigBuilder,
        config_selector: F,
    ) -> Result<(ConfigAttribs, Config), CreationError>
    where
        F: FnMut(
            Vec<ffi::egl::types::EGLConfig>,
            ffi::egl::types::EGLDisplay,
        ) -> Result<ffi::egl::types::EGLConfig, ()>,
    {
        let egl = EGL.as_ref().unwrap();
        // binding the right API and choosing the version
        let (version, api) =
            unsafe { bind_and_get_api(&cb.version, disp.egl_version)? };

        let (config_id, attribs) = unsafe {
            choose_fbconfig(
                disp,
                api,
                version,
                cb,
                config_selector,
            )?
        };

        Ok((
            attribs,
            Config {
                display: Arc::clone(&disp.0),
                api: api,
                version,
                config_id,
            },
        ))
    }
}

#[cfg(target_os = "android")]
#[inline]
fn get_native_display(ndisp: &NativeDisplay) -> *const raw::c_void {
    let egl = EGL.as_ref().unwrap();
    unsafe { egl.GetDisplay(ffi::egl::DEFAULT_DISPLAY as *mut _) }
}

fn get_egl_version(
    disp: ffi::egl::types::EGLDisplay,
) -> Result<EGLVersion, CreationError> {
    unsafe {
        let egl = EGL.as_ref().unwrap();
        let mut major: ffi::egl::types::EGLint = 0;
        let mut minor: ffi::egl::types::EGLint = 0;

        if egl.Initialize(disp, &mut major, &mut minor) == 0 {
            return Err(CreationError::OsError(
                "eglInitialize failed".to_string(),
            ));
        }

        Ok((major, minor))
    }
}

type EGLVersion = (ffi::egl::types::EGLint, ffi::egl::types::EGLint);

unsafe fn rebind_api(
    api: Api,
    egl_version: EGLVersion,
) -> Result<(), CreationError> {
    let egl = EGL.as_ref().unwrap();
    if egl_version >= (1, 2) {
        if match api {
            Api::OpenGl if egl_version >= (1, 4) => egl.BindAPI(ffi::egl::OPENGL_API),
            Api::OpenGlEs => egl.BindAPI(ffi::egl::OPENGL_ES_API),
        } == 0 {
            return Err(CreationError::OpenGlVersionNotSupported);
        }
    }

    Ok(())
}

unsafe fn bind_and_get_api(
    version: &GlRequest,
    egl_version: EGLVersion,
) -> Result<(Option<GlVersion>, Api), CreationError> {
    let egl = EGL.as_ref().unwrap();
    match version {
        GlRequest::Latest => {
            if egl_version >= (1, 2) {
                if egl_version >= (1, 4) && egl.BindAPI(ffi::egl::OPENGL_API) != 0 {
                    Ok((None, Api::OpenGl))
                } else if egl.BindAPI(ffi::egl::OPENGL_ES_API) != 0 {
                    Ok((None, Api::OpenGlEs))
                } else {
                    Err(CreationError::OpenGlVersionNotSupported)
                }
            } else {
                Ok((None, Api::OpenGlEs))
            }
        }
        GlRequest::Specific(Api::OpenGlEs, version) => {
            if egl_version < (1, 2) {
                return Err(CreationError::OpenGlVersionNotSupported);
            }
            if egl.BindAPI(ffi::egl::OPENGL_ES_API) == 0 {
                return Err(CreationError::OpenGlVersionNotSupported);
            }
            Ok((Some(*version), Api::OpenGlEs))
        }
        GlRequest::Specific(Api::OpenGl, version) => {
            if egl_version < (1, 4) {
                return Err(CreationError::OpenGlVersionNotSupported);
            }
            if egl.BindAPI(ffi::egl::OPENGL_API) == 0 {
                return Err(CreationError::OpenGlVersionNotSupported);
            }
            Ok((Some(*version), Api::OpenGl))
        }
        GlRequest::Specific(_, _) => {
            Err(CreationError::OpenGlVersionNotSupported)
        }
        GlRequest::GlThenGles {
            opengles_version,
            opengl_version,
        } => {
            if egl_version >= (1, 2) {
                if egl_version >= (1, 4) && egl.BindAPI(ffi::egl::OPENGL_API) != 0 {
                    Ok((Some(*opengl_version), Api::OpenGl))
                } else if egl.BindAPI(ffi::egl::OPENGL_ES_API) != 0 {
                    Ok((Some(*opengles_version), Api::OpenGlEs))
                } else {
                    Err(CreationError::OpenGlVersionNotSupported)
                }
            } else {
                Ok((Some(*opengles_version), Api::OpenGlEs))
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
fn get_native_display(ndisp: &NativeDisplay) -> *const raw::c_void {
    let egl = EGL.as_ref().unwrap();
    // the first step is to query the list of extensions without any display, if
    // supported
    let dp_extensions = unsafe {
        let p =
            egl.QueryString(ffi::egl::NO_DISPLAY, ffi::egl::EXTENSIONS as i32);

        // this possibility is available only with EGL 1.5 or
        // EGL_EXT_platform_base, otherwise `eglQueryString` returns an
        // error
        if p.is_null() {
            vec![]
        } else {
            let p = CStr::from_ptr(p);
            let list = String::from_utf8(p.to_bytes().to_vec())
                .unwrap_or_else(|_| format!(""));
            list.split(' ').map(|e| e.to_string()).collect::<Vec<_>>()
        }
    };

    let has_dp_extension =
        |e: &str| dp_extensions.iter().find(|s| s == &e).is_some();

    match *ndisp {
        // Note: Some EGL implementations are missing the
        // `eglGetPlatformDisplay(EXT)` symbol       despite reporting
        // `EGL_EXT_platform_base`. I'm pretty sure this is a bug.
        //       Therefore we detect whether the symbol is loaded in addition to
        // checking for       extensions.
        NativeDisplay::X11(display)
            if has_dp_extension("EGL_KHR_platform_x11")
                && egl.GetPlatformDisplay.is_loaded() =>
        {
            let d = display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *const _);
            // TODO: `PLATFORM_X11_SCREEN_KHR`
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_X11_KHR,
                    d as *mut _,
                    std::ptr::null(),
                )
            }
        }

        NativeDisplay::X11(display)
            if has_dp_extension("EGL_EXT_platform_x11")
                && egl.GetPlatformDisplayEXT.is_loaded() =>
        {
            let d = display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *const _);
            // TODO: `PLATFORM_X11_SCREEN_EXT`
            unsafe {
                egl.GetPlatformDisplayEXT(
                    ffi::egl::PLATFORM_X11_EXT,
                    d as *mut _,
                    std::ptr::null(),
                )
            }
        }

        NativeDisplay::Gbm(display)
            if has_dp_extension("EGL_KHR_platform_gbm")
                && egl.GetPlatformDisplay.is_loaded() =>
        {
            let d = display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *const _);
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_GBM_KHR,
                    d as *mut _,
                    std::ptr::null(),
                )
            }
        }

        NativeDisplay::Gbm(display)
            if has_dp_extension("EGL_MESA_platform_gbm")
                && egl.GetPlatformDisplayEXT.is_loaded() =>
        {
            let d = display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *const _);
            unsafe {
                egl.GetPlatformDisplayEXT(
                    ffi::egl::PLATFORM_GBM_KHR,
                    d as *mut _,
                    std::ptr::null(),
                )
            }
        }

        NativeDisplay::Wayland(display)
            if has_dp_extension("EGL_KHR_platform_wayland")
                && egl.GetPlatformDisplay.is_loaded() =>
        {
            let d = display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *const _);
            unsafe {
                egl.GetPlatformDisplay(
                    ffi::egl::PLATFORM_WAYLAND_KHR,
                    d as *mut _,
                    std::ptr::null(),
                )
            }
        }

        NativeDisplay::Wayland(display)
            if has_dp_extension("EGL_EXT_platform_wayland")
                && egl.GetPlatformDisplayEXT.is_loaded() =>
        {
            let d = display.unwrap_or(ffi::egl::DEFAULT_DISPLAY as *const _);
            unsafe {
                egl.GetPlatformDisplayEXT(
                    ffi::egl::PLATFORM_WAYLAND_EXT,
                    d as *mut _,
                    std::ptr::null(),
                )
            }
        }

        // TODO: This will never be reached right now, as the android egl
        // bindings use the static generator, so can't rely on
        // GetPlatformDisplay(EXT).
        NativeDisplay::Android
            if has_dp_extension("EGL_KHR_platform_android")
                && egl.GetPlatformDisplay.is_loaded() =>
        unsafe {
            egl.GetPlatformDisplay(
                ffi::egl::PLATFORM_ANDROID_KHR,
                ffi::egl::DEFAULT_DISPLAY as *mut _,
                std::ptr::null(),
            )
        }

        NativeDisplay::Device(display)
            if has_dp_extension("EGL_EXT_platform_device")
                && egl.GetPlatformDisplay.is_loaded() =>
        unsafe {
            egl.GetPlatformDisplay(
                ffi::egl::PLATFORM_DEVICE_EXT,
                display as *mut _,
                std::ptr::null(),
            )
        }

        NativeDisplay::X11(Some(display))
        | NativeDisplay::Gbm(Some(display))
        | NativeDisplay::Wayland(Some(display))
        | NativeDisplay::Device(display)
        | NativeDisplay::Other(Some(display)) => unsafe {
            egl.GetDisplay(display as *mut _)
        },

        NativeDisplay::X11(None)
        | NativeDisplay::Gbm(None)
        | NativeDisplay::Wayland(None)
        | NativeDisplay::Android
        | NativeDisplay::Other(None) => unsafe {
            egl.GetDisplay(ffi::egl::DEFAULT_DISPLAY as *mut _)
        },
    }
}

impl Context {
    /// Start building an EGL context.
    ///
    /// This function initializes some things and chooses the pixel format.
    ///
    /// To finish the process, you must call `.finish(window)` on the
    /// `ContextPrototype`.
    pub(crate) fn new<'a>(
        disp: &Display,
        cb: &'a ContextBuilderWrapper<&'a Context>,
        supports_surfaceless: bool,
        conf: ConfigWrapper<&Config>,
    ) -> Result<Context, CreationError>
    {
        let egl = EGL.as_ref().unwrap();

        // FIXME: Support mixing apis
        rebind_api(conf.config.api, disp.egl_version);

        // FIXME: Also check for the GL_OES_surfaceless_context *CONTEXT*
        // extension
        if supports_surfaceless
            && disp.extensions
                .iter()
                .find(|s| s == &"EGL_KHR_surfaceless_context")
                .is_none()
        {
            return Err(CreationError::NotSupported(
                "EGL surfaceless not supported".to_string(),
            ));
        }

        let share = match cb.sharing {
            Some(ctx) => ctx.context,
            None => std::ptr::null(),
        };

        let context = unsafe {
            if let Some(version) = conf.attribs.version {
                create_context(
                    disp,
                    cb,
                    conf,
                    version,
                    share,
                )?
            } else if conf.attribs.api == Api::OpenGlEs {
                if let Ok(ctx) = create_context(
                    disp,
                    cb,
                    conf,
                    GlVersion(2, 0),
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    disp,
                    cb,
                    conf,
                    GlVersion(1, 0),
                    share,
                ) {
                    ctx
                } else {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
            } else {
                if let Ok(ctx) = create_context(
                    disp,
                    cb,
                    conf,
                    GlVersion(3, 2),
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    disp,
                    cb,
                    conf,
                    GlVersion(3, 1),
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    disp,
                    cb,
                    conf,
                    GlVersion(1, 0),
                    share,
                ) {
                    ctx
                } else {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
            }
        };

        Ok(Context {
            display: Arc::clone(&disp.0),
            context,
            config: conf.with_config(conf.config.clone()),
        })
    }

    unsafe fn make_current<T: SurfaceTypeTrait>(
        &self,
        surf: &EGLSurface<T>,
        is_pbuffer: bool,
    ) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();

        {
            let has_been_current = surf.has_been_current.lock();

            if !*has_been_current {
                // VSync defaults to enabled; disable it if it was not requested.
                if !is_pbuffer && !surf.config.attribs.vsync {
                    let _guard = MakeCurrentGuard::new(
                        **surf.display,
                        surf.surface,
                        surf.surface,
                        self.context,
                    )
                    .map_err(|err| ContextError::OsError(err))?;

                    unsafe {
                        if egl.SwapInterval(**surf.display, 0) == ffi::egl::FALSE {
                            panic!("finish_impl: eglSwapInterval failed: 0x{:x}", egl.GetError());
                        }
                    }
                }
                *has_been_current = true;
            }
        }

        let ret =
            egl.MakeCurrent(**self.display, surf.surface, surf.surface, self.context);

        check_make_current(Some(ret))
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();
        let ret = egl.MakeCurrent(
            **self.display,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_SURFACE,
            self.context,
        );

        check_make_current(Some(ret))
    }

    #[inline]
    pub unsafe fn make_current_surface(
        &self,
        surf: &WindowSurface,
    ) -> Result<(), ContextError> {
        self.make_current(surf, false)
    }

    #[inline]
    pub unsafe fn make_current_pbuffer(
        &self,
        pbuffer: &PBuffer,
    ) -> Result<(), ContextError> {
        self.make_current(pbuffer, true)
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();

        if egl.GetCurrentContext() == self.context {
            let ret = egl.MakeCurrent(
                **self.display,
                ffi::egl::NO_SURFACE,
                ffi::egl::NO_SURFACE,
                ffi::egl::NO_CONTEXT,
            );

            check_make_current(Some(ret))
        } else {
            check_make_current(None)
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        let egl = EGL.as_ref().unwrap();
        unsafe { egl.GetCurrentContext() == self.context }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config> {
        self.config.clone()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.config.config.api
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::egl::types::EGLContext {
        self.context
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> ffi::egl::types::EGLDisplay {
        **self.display
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
    //         self.config_id,
    //         nwin,
    //         std::ptr::null(),
    //     );
    //     if surface.is_null() {
    //         panic!(
    //             "on_surface_created: eglCreateWindowSurface failed with
    // 0x{:x}",             egl.GetError()
    //         )
    //     }
    //     let ret =
    //         egl.MakeCurrent(**self.display, *surface, *surface,
    // self.context);     if ret == 0 {
    //         panic!(
    //             "on_surface_created: eglMakeCurrent failed with 0x{:x}",
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
    //             "on_surface_destroyed: eglMakeCurrent failed with 0x{:x}",
    //             egl.GetError()
    //         )
    //     }
    //
    //     egl.DestroySurface(**self.display, *surface);
    //     *surface = ffi::egl::NO_SURFACE;
    // }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const c_void {
        let egl = EGL.as_ref().unwrap();
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        unsafe { egl.GetProcAddress(addr) as *const _ }
    }

    #[inline]
    pub fn get_native_visual_id(&self) -> ffi::egl::types::EGLint {
        get_native_visual_id(**self.display, self.config.config.config_id)
    }
}

unsafe fn check_make_current(ret: Option<u32>) -> Result<(), ContextError> {
    let egl = EGL.as_ref().unwrap();
    if ret == Some(0) {
        match egl.GetError() as u32 {
            ffi::egl::CONTEXT_LOST => Err(ContextError::ContextLost),
            err => panic!(
                "make_current: eglMakeCurrent failed (eglGetError returned 0x{:x})",
                err
            ),
        }
    } else {
        Ok(())
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

unsafe impl Send for WindowSurface {}
unsafe impl Sync for WindowSurface {}

unsafe impl Send for PBuffer {}
unsafe impl Sync for PBuffer {}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
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
            // implementation does not follow the docs,  or maybe I'm misreading
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
            // the same EGLDisplay that they'd at least do
            // some ref counting, but they don't.
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
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            let egl = EGL.as_ref().unwrap();

            let mut guard = MakeCurrentGuard::new_keep(**self.display);
            guard.if_any_same_then_invalidate(
                ffi::egl::NO_SURFACE,
                ffi::egl::NO_SURFACE,
                self.context,
            );
            std::mem::drop(guard);

            egl.DestroyContext(**self.display, self.context);
            self.context = ffi::egl::NO_CONTEXT;
        }
    }
}

#[inline]
pub fn get_native_visual_id(
    disp: ffi::egl::types::EGLDisplay,
    config_id: ffi::egl::types::EGLConfig,
) -> ffi::egl::types::EGLint {
    let egl = EGL.as_ref().unwrap();
    let mut value = 0;
    let ret = unsafe {
        egl.GetConfigAttrib(
            disp,
            config_id,
            ffi::egl::NATIVE_VISUAL_ID as ffi::egl::types::EGLint,
            &mut value,
        )
    };
    if ret == 0 {
        panic!(
            "get_native_visual_id: eglGetConfigAttrib failed with 0x{:x}",
            unsafe { egl.GetError() }
        )
    };
    value
}

pub trait SurfaceTypeTrait {}

#[derive(Debug)]
pub enum WindowSurfaceType {}
#[derive(Debug)]
pub enum PBufferSurfaceType {}

impl SurfaceTypeTrait for WindowSurfaceType {}
impl SurfaceTypeTrait for PBufferSurfaceType {}

pub type WindowSurface = EGLSurface<WindowSurfaceType>;
pub type PBuffer = EGLSurface<PBufferSurfaceType>;

#[derive(Debug)]
pub struct EGLSurface<T: SurfaceTypeTrait> {
    display: Arc<DisplayInternal>,
    surface: ffi::egl::types::EGLSurface,
    config: ConfigWrapper<Config>,
    phantom: PhantomData<T>,
    has_been_current: Mutex<bool>,
}

impl WindowSurface {
    #[inline]
    pub fn new_window_surface<T>(
        disp: &Display,
        conf: ConfigWrapper<&Config>,
        nwin: ffi::EGLNativeWindowType,
    ) -> Result<Self, CreationError> {
        let egl = EGL.as_ref().unwrap();
        let surf = unsafe {
            let surf = egl.CreateWindowSurface(
                ***disp,
                conf.config.config_id,
                nwin,
                std::ptr::null(),
            );
            if surf.is_null() {
                return Err(CreationError::OsError(format!(
                    "eglCreateWindowSurface failed with 0x{:x}",
                    egl.GetError()
                )));
            }
            surf
        };

        Ok(WindowSurface {
            display: Arc::clone(&disp.0),
            config: conf.with_config(conf.config.clone()),
            surface: surf,
            phantom: PhantomData,
            has_been_current: Mutex::new(false),
        })
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();
        if self.surface == ffi::egl::NO_SURFACE {
            return Err(ContextError::ContextLost);
        }

        let ret = unsafe { egl.SwapBuffers(**self.display, self.surface) };

        if ret == 0 {
            match unsafe { egl.GetError() } as u32 {
                ffi::egl::CONTEXT_LOST => {
                    return Err(ContextError::ContextLost)
                }
                err => panic!(
                    "swap_buffers: eglSwapBuffers failed (eglGetError returned 0x{:x})",
                    err
                ),
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();

        if !egl.SwapBuffersWithDamageKHR.is_loaded() {
            return Err(ContextError::OsError("buffer damage not suported".to_string()));
        }

        if self.surface == ffi::egl::NO_SURFACE {
            return Err(ContextError::ContextLost);
        }

        let mut ffirects: Vec<ffi::egl::types::EGLint> =
            Vec::with_capacity(rects.len() * 4);

        for rect in rects {
            ffirects.push(rect.x as ffi::egl::types::EGLint);
            ffirects.push(rect.y as ffi::egl::types::EGLint);
            ffirects.push(rect.width as ffi::egl::types::EGLint);
            ffirects.push(rect.height as ffi::egl::types::EGLint);
        }

        let ret = unsafe {
            egl.SwapBuffersWithDamageKHR(
                **self.display,
                self.surface,
                ffirects.as_mut_ptr(),
                rects.len() as ffi::egl::types::EGLint,
            )
        };

        if ret == ffi::egl::FALSE {
            match unsafe { egl.GetError() } as u32 {
                ffi::egl::CONTEXT_LOST => {
                    return Err(ContextError::ContextLost)
                }
                err => panic!(
                    "swap_buffers: eglSwapBuffers failed (eglGetError returned 0x{:x})",
                    err
                ),
            }
        } else {
            Ok(())
        }
    }
}

impl PBuffer {
    #[inline]
    pub fn new_pbuffer<T>(
        disp: &Display,
        conf: ConfigWrapper<&Config>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        let size: (u32, u32) = size.into();

        let egl = EGL.as_ref().unwrap();

        let tex_fmt = if conf.attribs.alpha_bits > 0 {
            ffi::egl::TEXTURE_RGBA
        } else {
            ffi::egl::TEXTURE_RGB
        };

        let attrs = &[
            ffi::egl::WIDTH as raw::c_int,
            size.0 as raw::c_int,
            ffi::egl::HEIGHT as raw::c_int,
            size.1 as raw::c_int,
            ffi::egl::NONE as raw::c_int,
        ];

        let surf = unsafe {
            let pbuffer = egl.CreatePbufferSurface(
                ***disp,
                conf.config.config_id,
                attrs.as_ptr(),
            );
            if pbuffer.is_null() || pbuffer == ffi::egl::NO_SURFACE {
                return Err(CreationError::OsError(
                    "eglCreatePbufferSurface failed".to_string(),
                ));
            }
            pbuffer
        };

        Ok(PBuffer {
            display: Arc::clone(&disp.0),
            config: conf.with_config(conf.config.clone()),
            surface: surf,
            phantom: PhantomData,
            has_been_current: Mutex::new(false),
        })
    }
}

impl<T: SurfaceTypeTrait> EGLSurface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        let egl = EGL.as_ref().unwrap();
        unsafe {
            egl.GetCurrentSurface(ffi::egl::DRAW as i32) == self.surface
                || egl.GetCurrentSurface(ffi::egl::READ as i32) == self.surface
        }
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config> {
        self.config.clone()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();

        if egl.GetCurrentSurface(ffi::egl::DRAW as i32) == self.surface
            || egl.GetCurrentSurface(ffi::egl::READ as i32) == self.surface
        {
            let ret = egl.MakeCurrent(
                **self.display,
                ffi::egl::NO_SURFACE,
                ffi::egl::NO_SURFACE,
                ffi::egl::NO_CONTEXT,
            );

            check_make_current(Some(ret))
        } else {
            check_make_current(None)
        }
    }
}

impl<T: SurfaceTypeTrait> Drop for EGLSurface<T> {
    fn drop(&mut self) {
        unsafe {
            let egl = EGL.as_ref().unwrap();

            let mut guard = MakeCurrentGuard::new_keep(**self.display);
            guard.if_any_same_then_invalidate(
                self.surface,
                self.surface,
                ffi::egl::NO_CONTEXT,
            );
            std::mem::drop(guard);

            egl.DestroySurface(**self.display, self.surface);
            self.surface = ffi::egl::NO_SURFACE;
        }
    }
}

unsafe fn choose_fbconfig<F>(
    disp: &Display,
    api: Api,
    version: Option<GlVersion>,
    cb: ConfigBuilder,
    mut config_selector: F,
) -> Result<(ffi::egl::types::EGLConfig, ConfigAttribs), CreationError>
where
    F: FnMut(
        Vec<ffi::egl::types::EGLConfig>,
        ffi::egl::types::EGLDisplay,
    ) -> Result<ffi::egl::types::EGLConfig, ()>,
{
    let egl = EGL.as_ref().unwrap();

    let descriptor = {
        let mut out: Vec<raw::c_int> = Vec::with_capacity(37);

        if disp.egl_version >= (1, 2) {
            out.push(ffi::egl::COLOR_BUFFER_TYPE as raw::c_int);
            out.push(ffi::egl::RGB_BUFFER as raw::c_int);
        }

        out.push(ffi::egl::SURFACE_TYPE as raw::c_int);
        let mut surface_type = 0;
        if cb.window_surface_support {
            surface_type = surface_type | ffi::egl::WINDOW_BIT;
        }
        if cb.pbuffer_support {
            surface_type = surface_type | ffi::egl::PBUFFER_BIT;
        }
        out.push(surface_type as raw::c_int);

        match (api, version) {
            (Api::OpenGlEs, Some(GlVersion(3, _))) => {
                if disp.egl_version < (1, 3) {
                    return Err(CreationError::NoAvailableConfig);
                }
                out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                out.push(ffi::egl::OPENGL_ES3_BIT as raw::c_int);
                out.push(ffi::egl::CONFORMANT as raw::c_int);
                out.push(ffi::egl::OPENGL_ES3_BIT as raw::c_int);
            }
            (Api::OpenGlEs, Some(GlVersion(2, _))) => {
                if disp.egl_version < (1, 3) {
                    return Err(CreationError::NoAvailableConfig);
                }
                out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                out.push(ffi::egl::OPENGL_ES2_BIT as raw::c_int);
                out.push(ffi::egl::CONFORMANT as raw::c_int);
                out.push(ffi::egl::OPENGL_ES2_BIT as raw::c_int);
            }
            (Api::OpenGlEs, Some(GlVersion(1, _))) => {
                if disp.egl_version >= (1, 3) {
                    out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                    out.push(ffi::egl::OPENGL_ES_BIT as raw::c_int);
                    out.push(ffi::egl::CONFORMANT as raw::c_int);
                    out.push(ffi::egl::OPENGL_ES_BIT as raw::c_int);
                }
            }
            (Api::OpenGlEs, _) => unimplemented!(),
            (Api::OpenGl, _) => {
                if disp.egl_version < (1, 3) {
                    return Err(CreationError::NoAvailableConfig);
                }
                out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                out.push(ffi::egl::OPENGL_BIT as raw::c_int);
                out.push(ffi::egl::CONFORMANT as raw::c_int);
                out.push(ffi::egl::OPENGL_BIT as raw::c_int);
            }
            (_, _) => unimplemented!(),
        };

        if let Some(hardware_accelerated) = cb.hardware_accelerated {
            out.push(ffi::egl::CONFIG_CAVEAT as raw::c_int);
            out.push(if hardware_accelerated {
                ffi::egl::NONE as raw::c_int
            } else {
                ffi::egl::SLOW_CONFIG as raw::c_int
            });
        }

        if let Some(color) = cb.color_bits {
            out.push(ffi::egl::RED_SIZE as raw::c_int);
            out.push((color / 3) as raw::c_int);
            out.push(ffi::egl::GREEN_SIZE as raw::c_int);
            out.push(
                (color / 3 + if color % 3 != 0 { 1 } else { 0 }) as raw::c_int,
            );
            out.push(ffi::egl::BLUE_SIZE as raw::c_int);
            out.push(
                (color / 3 + if color % 3 == 2 { 1 } else { 0 }) as raw::c_int,
            );
        }

        if let Some(alpha) = cb.alpha_bits {
            out.push(ffi::egl::ALPHA_SIZE as raw::c_int);
            out.push(alpha as raw::c_int);
        }

        if let Some(depth) = cb.depth_bits {
            out.push(ffi::egl::DEPTH_SIZE as raw::c_int);
            out.push(depth as raw::c_int);
        }

        if let Some(stencil) = cb.stencil_bits {
            out.push(ffi::egl::STENCIL_SIZE as raw::c_int);
            out.push(stencil as raw::c_int);
        }

        if let Some(true) = cb.double_buffer {
            return Err(CreationError::NoAvailableConfig);
        }

        if let Some(multisampling) = cb.multisampling {
            out.push(ffi::egl::SAMPLES as raw::c_int);
            out.push(multisampling as raw::c_int);
        }

        if cb.stereoscopy {
            return Err(CreationError::NoAvailableConfig);
        }

        if let Some(xid) = cb.plat_attr.x11_visual_xid {
            out.push(ffi::egl::NATIVE_VISUAL_ID as raw::c_int);
            out.push(xid as raw::c_int);
        }

        // FIXME: srgb is not taken into account

        match cb.release_behavior {
            ReleaseBehavior::Flush => (),
            ReleaseBehavior::None => {
                // TODO: with EGL you need to manually set the behavior
                unimplemented!()
            }
        }

        out.push(ffi::egl::NONE as raw::c_int);
        out
    };

    // calling `eglChooseConfig`
    let mut num_configs = 0;
    if egl.ChooseConfig(
        ***disp,
        descriptor.as_ptr(),
        std::ptr::null_mut(),
        0,
        &mut num_configs,
    ) == 0
    {
        return Err(CreationError::OsError(
            "eglChooseConfig failed".to_string(),
        ));
    }

    if num_configs == 0 {
        return Err(CreationError::NoAvailableConfig);
    }

    let mut config_ids = Vec::with_capacity(num_configs as usize);
    config_ids.resize_with(num_configs as usize, || std::mem::zeroed());
    if egl.ChooseConfig(
        ***disp,
        descriptor.as_ptr(),
        config_ids.as_mut_ptr(),
        num_configs,
        &mut num_configs,
    ) == 0
    {
        return Err(CreationError::OsError(
            "eglChooseConfig failed".to_string(),
        ));
    }

    // analyzing each config
    macro_rules! attrib {
        ($egl:expr, $display:expr, $config:expr, $attr:expr $(,)?) => {{
            let mut value = 0;
            let res = $egl.GetConfigAttrib(
                ***$display,
                $config,
                $attr as ffi::egl::types::EGLint,
                &mut value,
            );
            if res == 0 {
                Err(CreationError::OsError(
                    "eglGetConfigAttrib failed".to_string(),
                ))
            } else {
                Ok(value)
            }
        }};
    };

    let config_ids = if let Some(vsync) = cb.vsync {
        // We're interested in those configs which allow our desired VSync.
        let desired_swap_interval = if vsync {
            1
        } else {
            0
        };

        config_ids.into_iter().filter_map(|config_id| {
            let mut min_swap_interval = attrib!(
                egl,
                disp,
                config_id,
                ffi::egl::MIN_SWAP_INTERVAL,
            );

            if let Err(min_swap_interval) = min_swap_interval {
                return Some(Err(min_swap_interval));
            }

            if desired_swap_interval < min_swap_interval.unwrap() {
                return None;
            }

            let mut max_swap_interval = attrib!(
                egl,
                disp,
                config_id,
                ffi::egl::MAX_SWAP_INTERVAL,
            );

            if let Err(max_swap_interval) = max_swap_interval {
                return Some(Err(max_swap_interval));
            }

            if desired_swap_interval > max_swap_interval.unwrap() {
                return None;
            }

            Some(Ok(config_id))
        }).collect::<Result<Vec<_>, _>>()?
    } else {
        config_ids
    };

    if config_ids.is_empty() {
        return Err(CreationError::NoAvailableConfig);
    }

    let config_id = config_selector(config_ids, ***disp)
        .map_err(|_| CreationError::NoAvailableConfig)?;

    let mut min_swap_interval = attrib!(
        egl,
        disp,
        config_id,
        ffi::egl::MIN_SWAP_INTERVAL,
    )?;

    let mut max_swap_interval = attrib!(
        egl,
        disp,
        config_id,
        ffi::egl::MAX_SWAP_INTERVAL,
    )?;

    assert!(min_swap_interval >= 0);

    let desc = ConfigAttribs {
        api,
        version,
        vsync: min_swap_interval <= 1 && max_swap_interval >= 1,
        window_surface_support: cb.window_surface_support,
        pbuffer_support: cb.pbuffer_support,
        hardware_accelerated: attrib!(
            egl,
            disp,
            config_id,
            ffi::egl::CONFIG_CAVEAT,
        )? != ffi::egl::SLOW_CONFIG as i32,
        color_bits: attrib!(egl, disp, config_id, ffi::egl::RED_SIZE)? as u8
            + attrib!(egl, disp, config_id, ffi::egl::BLUE_SIZE)? as u8
            + attrib!(egl, disp, config_id, ffi::egl::GREEN_SIZE)? as u8,
        alpha_bits: attrib!(egl, disp, config_id, ffi::egl::ALPHA_SIZE)?
            as u8,
        depth_bits: attrib!(egl, disp, config_id, ffi::egl::DEPTH_SIZE)?
            as u8,
        stencil_bits: attrib!(egl, disp, config_id, ffi::egl::STENCIL_SIZE)?
            as u8,
        stereoscopy: false,
        double_buffer: true,
        multisampling: match attrib!(egl, disp, config_id, ffi::egl::SAMPLES)?
        {
            0 | 1 => None,
            a => Some(a as u16),
        },
        srgb: false, // TODO: use EGL_KHR_gl_colorspace to know that
    };

    Ok((config_id, desc))
}

unsafe fn create_context<'a>(
    disp: &Display,
    cb: &'a ContextBuilderWrapper<&'a Context>,
    conf: ConfigWrapper<&Config>,
    version: GlVersion,
    share: ffi::EGLContext,
) -> Result<ffi::egl::types::EGLContext, CreationError> {
    let egl = EGL.as_ref().unwrap();

    let mut context_attributes = Vec::with_capacity(10);
    let mut flags = 0;

    if disp.egl_version >= (1, 5)
        || disp.extensions
            .iter()
            .find(|s| s == &"EGL_KHR_create_context")
            .is_some()
    {
        context_attributes.push(ffi::egl::CONTEXT_MAJOR_VERSION as i32);
        context_attributes.push(version.0 as i32);
        context_attributes.push(ffi::egl::CONTEXT_MINOR_VERSION as i32);
        context_attributes.push(version.1 as i32);

        // handling robustness
        let supports_robustness = disp.egl_version >= (1, 5)
            || disp.extensions
                .iter()
                .find(|s| s == &"EGL_EXT_create_context_robustness")
                .is_some();

        match cb.robustness {
            Robustness::NotRobust => (),

            Robustness::NoError => {
                if disp.extensions
                    .iter()
                    .find(|s| s == &"EGL_KHR_create_context_no_error")
                    .is_some()
                {
                    context_attributes.push(
                        ffi::egl::CONTEXT_OPENGL_NO_ERROR_KHR as raw::c_int,
                    );
                    context_attributes.push(1);
                }
            }

            Robustness::RobustNoResetNotification => {
                if supports_robustness {
                    context_attributes.push(
                        ffi::egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY
                            as raw::c_int,
                    );
                    context_attributes
                        .push(ffi::egl::NO_RESET_NOTIFICATION as raw::c_int);
                    flags = flags
                        | ffi::egl::CONTEXT_OPENGL_ROBUST_ACCESS as raw::c_int;
                } else {
                    return Err(CreationError::RobustnessNotSupported);
                }
            }

            Robustness::TryRobustNoResetNotification => {
                if supports_robustness {
                    context_attributes.push(
                        ffi::egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY
                            as raw::c_int,
                    );
                    context_attributes
                        .push(ffi::egl::NO_RESET_NOTIFICATION as raw::c_int);
                    flags = flags
                        | ffi::egl::CONTEXT_OPENGL_ROBUST_ACCESS as raw::c_int;
                }
            }

            Robustness::RobustLoseContextOnReset => {
                if supports_robustness {
                    context_attributes.push(
                        ffi::egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY
                            as raw::c_int,
                    );
                    context_attributes
                        .push(ffi::egl::LOSE_CONTEXT_ON_RESET as raw::c_int);
                    flags = flags
                        | ffi::egl::CONTEXT_OPENGL_ROBUST_ACCESS as raw::c_int;
                } else {
                    return Err(CreationError::RobustnessNotSupported);
                }
            }

            Robustness::TryRobustLoseContextOnReset => {
                if supports_robustness {
                    context_attributes.push(
                        ffi::egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY
                            as raw::c_int,
                    );
                    context_attributes
                        .push(ffi::egl::LOSE_CONTEXT_ON_RESET as raw::c_int);
                    flags = flags
                        | ffi::egl::CONTEXT_OPENGL_ROBUST_ACCESS as raw::c_int;
                }
            }
        }

        if cb.debug {
            if disp.egl_version >= (1, 5) {
                context_attributes.push(ffi::egl::CONTEXT_OPENGL_DEBUG as i32);
                context_attributes.push(ffi::egl::TRUE as i32);
            }

            // TODO: using this flag sometimes generates an error there was a
            // change in the specs that added this flag, so it may not be
            // supported everywhere; however it is not possible to know whether
            // it is supported or not
            //
            // flags = flags | ffi::egl::CONTEXT_OPENGL_DEBUG_BIT_KHR as i32;
        }

        // In at least some configurations, the Android emulator’s GL
        // implementation advertises support for the
        // EGL_KHR_create_context extension but returns BAD_ATTRIBUTE
        // when CONTEXT_FLAGS_KHR is used.
        if flags != 0 {
            context_attributes.push(ffi::egl::CONTEXT_FLAGS_KHR as i32);
            context_attributes.push(flags);
        }
    } else if disp.egl_version >= (1, 3) && conf.config.api == Api::OpenGlEs {
        // robustness is not supported
        match cb.robustness {
            Robustness::RobustNoResetNotification
            | Robustness::RobustLoseContextOnReset => {
                return Err(CreationError::RobustnessNotSupported);
            }
            _ => (),
        }

        context_attributes.push(ffi::egl::CONTEXT_CLIENT_VERSION as i32);
        context_attributes.push(version.0 as i32);
    }

    context_attributes.push(ffi::egl::NONE as i32);

    let context = egl.CreateContext(
        ***disp,
        conf.config.config_id,
        share,
        context_attributes.as_ptr(),
    );

    if context.is_null() {
        match egl.GetError() as u32 {
            ffi::egl::BAD_MATCH | ffi::egl::BAD_ATTRIBUTE => {
                return Err(CreationError::OpenGlVersionNotSupported);
            }
            e => panic!("create_context: eglCreateContext failed: 0x{:x}", e),
        }
    }

    Ok(context)
}
