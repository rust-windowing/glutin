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

    #[derive(Clone)]
    pub struct Egl(pub SymWrapper<ffi::egl::Egl>);

    /// Because `*const raw::c_void` doesn't implement `Sync`.
    unsafe impl Sync for Egl {}

    impl SymTrait for ffi::egl::Egl {
        fn load_with<F>(loadfn: F) -> Self
        where
            F: FnMut(&'static str) -> *const std::os::raw::c_void,
        {
            Self::load_with(loadfn)
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
use crate::platform_impl::PlatformAttributes;
use crate::{
    Api, ContextBuilderWrapper, ContextError, ContextSupports, CreationError,
    GlAttributes, GlRequest, PixelFormat, PixelFormatRequirements,
    ReleaseBehavior, Robustness,
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

use std::ffi::{CStr, CString};
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
pub struct EGLDisplay(ffi::egl::types::EGLDisplay);

impl Deref for EGLDisplay {
    type Target = ffi::egl::types::EGLDisplay;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Context {
    display: Arc<EGLDisplay>,
    context: ffi::egl::types::EGLContext,
    api: Api,
    pixel_format: PixelFormat,
    config_id: ffi::egl::types::EGLConfig,
}

#[cfg(target_os = "android")]
#[inline]
fn get_native_display(native_display: &NativeDisplay) -> *const raw::c_void {
    let egl = EGL.as_ref().unwrap();
    unsafe { egl.GetDisplay(ffi::egl::DEFAULT_DISPLAY as *mut _) }
}

fn get_egl_version(
    display: ffi::egl::types::EGLDisplay,
) -> Result<(ffi::egl::types::EGLint, ffi::egl::types::EGLint), CreationError> {
    unsafe {
        let egl = EGL.as_ref().unwrap();
        let mut major: ffi::egl::types::EGLint = 0;
        let mut minor: ffi::egl::types::EGLint = 0;

        if egl.Initialize(display, &mut major, &mut minor) == 0 {
            return Err(CreationError::OsError(
                "eglInitialize failed".to_string(),
            ));
        }

        Ok((major, minor))
    }
}

unsafe fn bind_and_get_api<'a>(
    gl_attr: &'a GlAttributes<&'a Context>,
    egl_version: (ffi::egl::types::EGLint, ffi::egl::types::EGLint),
) -> Result<(Option<(u8, u8)>, Api), CreationError> {
    let egl = EGL.as_ref().unwrap();
    match gl_attr.version {
        GlRequest::Latest => {
            if egl_version >= (1, 4) {
                if egl.BindAPI(ffi::egl::OPENGL_API) != 0 {
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
            if egl_version >= (1, 2) {
                if egl.BindAPI(ffi::egl::OPENGL_ES_API) == 0 {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
            }
            Ok((Some(version), Api::OpenGlEs))
        }
        GlRequest::Specific(Api::OpenGl, version) => {
            if egl_version < (1, 4) {
                return Err(CreationError::OpenGlVersionNotSupported);
            }
            if egl.BindAPI(ffi::egl::OPENGL_API) == 0 {
                return Err(CreationError::OpenGlVersionNotSupported);
            }
            Ok((Some(version), Api::OpenGl))
        }
        GlRequest::Specific(_, _) => {
            Err(CreationError::OpenGlVersionNotSupported)
        }
        GlRequest::GlThenGles {
            opengles_version,
            opengl_version,
        } => {
            if egl_version >= (1, 4) {
                if egl.BindAPI(ffi::egl::OPENGL_API) != 0 {
                    Ok((Some(opengl_version), Api::OpenGl))
                } else if egl.BindAPI(ffi::egl::OPENGL_ES_API) != 0 {
                    Ok((Some(opengles_version), Api::OpenGlEs))
                } else {
                    Err(CreationError::OpenGlVersionNotSupported)
                }
            } else {
                Ok((Some(opengles_version), Api::OpenGlEs))
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
fn get_native_display(native_display: &NativeDisplay) -> *const raw::c_void {
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

    match *native_display {
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
    pub(crate) fn new<'a, F>(
        cb: &'a ContextBuilderWrapper<&'a Context>,
        native_display: NativeDisplay,
        ctx_supports: ContextSupports,
        config_selector: F,
    ) -> Result<Context, CreationError>
    where
        F: FnMut(
            Vec<ffi::egl::types::EGLConfig>,
            ffi::egl::types::EGLDisplay,
        ) -> Result<ffi::egl::types::EGLConfig, ()>,
    {
        let egl = EGL.as_ref().unwrap();
        // calling `eglGetDisplay` or equivalent
        let display = get_native_display(&native_display);

        if display.is_null() {
            return Err(CreationError::OsError(
                "Could not create EGL display object".to_string(),
            ));
        }

        let egl_version = get_egl_version(display)?;

        // the list of extensions supported by the client once initialized is
        // different from the list of extensions obtained earlier
        let extensions = if egl_version >= (1, 2) {
            let p = unsafe {
                CStr::from_ptr(
                    egl.QueryString(display, ffi::egl::EXTENSIONS as i32),
                )
            };
            let list = String::from_utf8(p.to_bytes().to_vec())
                .unwrap_or_else(|_| format!(""));
            list.split(' ').map(|e| e.to_string()).collect::<Vec<_>>()
        } else {
            vec![]
        };

        // FIXME: Also check for the GL_OES_surfaceless_context *CONTEXT*
        // extension
        if ctx_supports.contains(ContextSupports::SURFACELESS)
            && extensions
                .iter()
                .find(|s| s == &"EGL_KHR_surfaceless_context")
                .is_none()
        {
            return Err(CreationError::NotSupported(
                "EGL surfaceless not supported".to_string(),
            ));
        }

        // binding the right API and choosing the version
        let (version, api) =
            unsafe { bind_and_get_api(&cb.gl_attr, egl_version)? };

        let (config_id, pixel_format) = unsafe {
            choose_fbconfig(
                display,
                &egl_version,
                api,
                version,
                cb,
                ctx_supports,
                config_selector,
            )?
        };

        let share = match cb.gl_attr.sharing {
            Some(ctx) => ctx.context,
            None => std::ptr::null(),
        };

        let context = unsafe {
            if let Some(version) = version {
                create_context(
                    display,
                    &egl_version,
                    &extensions,
                    api,
                    version,
                    config_id,
                    cb.gl_attr.debug,
                    cb.gl_attr.robustness,
                    share,
                )?
            } else if api == Api::OpenGlEs {
                if let Ok(ctx) = create_context(
                    display,
                    &egl_version,
                    &extensions,
                    api,
                    (2, 0),
                    config_id,
                    cb.gl_attr.debug,
                    cb.gl_attr.robustness,
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    display,
                    &egl_version,
                    &extensions,
                    api,
                    (1, 0),
                    config_id,
                    cb.gl_attr.debug,
                    cb.gl_attr.robustness,
                    share,
                ) {
                    ctx
                } else {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
            } else {
                if let Ok(ctx) = create_context(
                    display,
                    &egl_version,
                    &extensions,
                    api,
                    (3, 2),
                    config_id,
                    cb.gl_attr.debug,
                    cb.gl_attr.robustness,
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    display,
                    &egl_version,
                    &extensions,
                    api,
                    (3, 1),
                    config_id,
                    cb.gl_attr.debug,
                    cb.gl_attr.robustness,
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    display,
                    &egl_version,
                    &extensions,
                    api,
                    (1, 0),
                    config_id,
                    cb.gl_attr.debug,
                    cb.gl_attr.robustness,
                    share,
                ) {
                    ctx
                } else {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
            }
        };

        Ok(Context {
            display: Arc::new(EGLDisplay(display)),
            context,
            api,
            pixel_format,
            config_id,
        })
    }

    unsafe fn make_current(
        &self,
        surface: ffi::egl::types::EGLSurface,
    ) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();
        let ret =
            egl.MakeCurrent(**self.display, surface, surface, self.context);

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
        surface: &WindowSurface,
    ) -> Result<(), ContextError> {
        self.make_current(surface.surface)
    }

    #[inline]
    pub unsafe fn make_current_pbuffer(
        &self,
        pbuffer: &PBuffer,
    ) -> Result<(), ContextError> {
        self.make_current(pbuffer.surface)
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
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pixel_format.clone()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.api
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
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let egl = EGL.as_ref().unwrap();
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        unsafe { egl.GetProcAddress(addr) as *const _ }
    }

    #[inline]
    pub fn get_native_visual_id(&self) -> ffi::egl::types::EGLint {
        get_native_visual_id(**self.display, self.config_id)
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

impl Drop for EGLDisplay {
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
    display: ffi::egl::types::EGLDisplay,
    config_id: ffi::egl::types::EGLConfig,
) -> ffi::egl::types::EGLint {
    let egl = EGL.as_ref().unwrap();
    let mut value = 0;
    let ret = unsafe {
        egl.GetConfigAttrib(
            display,
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
    display: Arc<EGLDisplay>,
    surface: ffi::egl::types::EGLSurface,
    pixel_format: PixelFormat,
    phantom: PhantomData<T>,
}

impl WindowSurface {
    #[inline]
    pub fn new_window_surface<T>(
        el: &EventLoopWindowTarget<T>,
        ctx: &Context,
        nwin: ffi::EGLNativeWindowType,
    ) -> Result<Self, CreationError> {
        let egl = EGL.as_ref().unwrap();
        let surface = unsafe {
            let surface = egl.CreateWindowSurface(
                **ctx.display,
                ctx.config_id,
                nwin,
                std::ptr::null(),
            );
            if surface.is_null() {
                return Err(CreationError::OsError(format!(
                    "eglCreateWindowSurface failed with 0x{:x}",
                    egl.GetError()
                )));
            }
            surface
        };

        Ok(WindowSurface {
            display: Arc::clone(&ctx.display),
            pixel_format: ctx.pixel_format.clone(),
            surface,
            phantom: PhantomData,
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
}

impl PBuffer {
    #[inline]
    pub fn new_pbuffer<T>(
        _el: &EventLoopWindowTarget<T>,
        ctx: &Context,
        size: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        let size: (u32, u32) = size.into();

        let egl = EGL.as_ref().unwrap();

        let tex_fmt = if ctx.pixel_format.alpha_bits > 0 {
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

        let surface = unsafe {
            let pbuffer = egl.CreatePbufferSurface(
                **ctx.display,
                ctx.config_id,
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
            display: Arc::clone(&ctx.display),
            pixel_format: ctx.pixel_format.clone(),
            surface,
            phantom: PhantomData,
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
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pixel_format.clone()
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
    display: ffi::egl::types::EGLDisplay,
    egl_version: &(ffi::egl::types::EGLint, ffi::egl::types::EGLint),
    api: Api,
    version: Option<(u8, u8)>,
    cb: &ContextBuilderWrapper<&Context>,
    ctx_supports: ContextSupports,
    mut config_selector: F,
) -> Result<(ffi::egl::types::EGLConfig, PixelFormat), CreationError>
where
    F: FnMut(
        Vec<ffi::egl::types::EGLConfig>,
        ffi::egl::types::EGLDisplay,
    ) -> Result<ffi::egl::types::EGLConfig, ()>,
{
    let egl = EGL.as_ref().unwrap();

    let descriptor = {
        let mut out: Vec<raw::c_int> = Vec::with_capacity(37);

        if egl_version >= &(1, 2) {
            out.push(ffi::egl::COLOR_BUFFER_TYPE as raw::c_int);
            out.push(ffi::egl::RGB_BUFFER as raw::c_int);
        }

        out.push(ffi::egl::SURFACE_TYPE as raw::c_int);
        let mut surface_type =
            if ctx_supports.contains(ContextSupports::WINDOW_SURFACES) {
                ffi::egl::WINDOW_BIT
            } else {
                0
            };
        if ctx_supports.contains(ContextSupports::PBUFFERS) {
            surface_type = surface_type | ffi::egl::PBUFFER_BIT
        }
        out.push(surface_type as raw::c_int);

        match (api, version) {
            (Api::OpenGlEs, Some((3, _))) => {
                if egl_version < &(1, 3) {
                    return Err(CreationError::NoAvailablePixelFormat);
                }
                out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                out.push(ffi::egl::OPENGL_ES3_BIT as raw::c_int);
                out.push(ffi::egl::CONFORMANT as raw::c_int);
                out.push(ffi::egl::OPENGL_ES3_BIT as raw::c_int);
            }
            (Api::OpenGlEs, Some((2, _))) => {
                if egl_version < &(1, 3) {
                    return Err(CreationError::NoAvailablePixelFormat);
                }
                out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                out.push(ffi::egl::OPENGL_ES2_BIT as raw::c_int);
                out.push(ffi::egl::CONFORMANT as raw::c_int);
                out.push(ffi::egl::OPENGL_ES2_BIT as raw::c_int);
            }
            (Api::OpenGlEs, Some((1, _))) => {
                if egl_version >= &(1, 3) {
                    out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                    out.push(ffi::egl::OPENGL_ES_BIT as raw::c_int);
                    out.push(ffi::egl::CONFORMANT as raw::c_int);
                    out.push(ffi::egl::OPENGL_ES_BIT as raw::c_int);
                }
            }
            (Api::OpenGlEs, _) => unimplemented!(),
            (Api::OpenGl, _) => {
                if egl_version < &(1, 3) {
                    return Err(CreationError::NoAvailablePixelFormat);
                }
                out.push(ffi::egl::RENDERABLE_TYPE as raw::c_int);
                out.push(ffi::egl::OPENGL_BIT as raw::c_int);
                out.push(ffi::egl::CONFORMANT as raw::c_int);
                out.push(ffi::egl::OPENGL_BIT as raw::c_int);
            }
            (_, _) => unimplemented!(),
        };

        if let Some(hardware_accelerated) = cb.pf_reqs.hardware_accelerated {
            out.push(ffi::egl::CONFIG_CAVEAT as raw::c_int);
            out.push(if hardware_accelerated {
                ffi::egl::NONE as raw::c_int
            } else {
                ffi::egl::SLOW_CONFIG as raw::c_int
            });
        }

        if let Some(color) = cb.pf_reqs.color_bits {
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

        if let Some(alpha) = cb.pf_reqs.alpha_bits {
            out.push(ffi::egl::ALPHA_SIZE as raw::c_int);
            out.push(alpha as raw::c_int);
        }

        if let Some(depth) = cb.pf_reqs.depth_bits {
            out.push(ffi::egl::DEPTH_SIZE as raw::c_int);
            out.push(depth as raw::c_int);
        }

        if let Some(stencil) = cb.pf_reqs.stencil_bits {
            out.push(ffi::egl::STENCIL_SIZE as raw::c_int);
            out.push(stencil as raw::c_int);
        }

        if let Some(true) = cb.pf_reqs.double_buffer {
            return Err(CreationError::NoAvailablePixelFormat);
        }

        if let Some(multisampling) = cb.pf_reqs.multisampling {
            out.push(ffi::egl::SAMPLES as raw::c_int);
            out.push(multisampling as raw::c_int);
        }

        if cb.pf_reqs.stereoscopy {
            return Err(CreationError::NoAvailablePixelFormat);
        }

        if let Some(xid) = cb.plat_attr.x11_visual_xid {
            out.push(ffi::egl::NATIVE_VISUAL_ID as raw::c_int);
            out.push(xid as raw::c_int);
        }

        // FIXME: srgb is not taken into account

        match cb.pf_reqs.release_behavior {
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
        display,
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
        return Err(CreationError::NoAvailablePixelFormat);
    }

    let mut config_ids = Vec::with_capacity(num_configs as usize);
    config_ids.resize_with(num_configs as usize, || std::mem::zeroed());
    if egl.ChooseConfig(
        display,
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

    if num_configs == 0 {
        return Err(CreationError::NoAvailablePixelFormat);
    }

    let config_id = config_selector(config_ids, display)
        .map_err(|_| CreationError::NoAvailablePixelFormat)?;

    // analyzing each config
    macro_rules! attrib {
        ($egl:expr, $display:expr, $config:expr, $attr:expr) => {{
            let mut value = 0;
            let res = $egl.GetConfigAttrib(
                $display,
                $config,
                $attr as ffi::egl::types::EGLint,
                &mut value,
            );
            if res == 0 {
                return Err(CreationError::OsError(
                    "eglGetConfigAttrib failed".to_string(),
                ));
            }
            value
        }};
    };

    let desc = PixelFormat {
        hardware_accelerated: attrib!(
            egl,
            display,
            config_id,
            ffi::egl::CONFIG_CAVEAT
        ) != ffi::egl::SLOW_CONFIG as i32,
        color_bits: attrib!(egl, display, config_id, ffi::egl::RED_SIZE) as u8
            + attrib!(egl, display, config_id, ffi::egl::BLUE_SIZE) as u8
            + attrib!(egl, display, config_id, ffi::egl::GREEN_SIZE) as u8,
        alpha_bits: attrib!(egl, display, config_id, ffi::egl::ALPHA_SIZE)
            as u8,
        depth_bits: attrib!(egl, display, config_id, ffi::egl::DEPTH_SIZE)
            as u8,
        stencil_bits: attrib!(egl, display, config_id, ffi::egl::STENCIL_SIZE)
            as u8,
        stereoscopy: false,
        double_buffer: true,
        multisampling: match attrib!(egl, display, config_id, ffi::egl::SAMPLES)
        {
            0 | 1 => None,
            a => Some(a as u16),
        },
        srgb: false, // TODO: use EGL_KHR_gl_colorspace to know that
    };

    Ok((config_id, desc))
}

unsafe fn create_context(
    display: ffi::egl::types::EGLDisplay,
    egl_version: &(ffi::egl::types::EGLint, ffi::egl::types::EGLint),
    extensions: &[String],
    api: Api,
    version: (u8, u8),
    config_id: ffi::egl::types::EGLConfig,
    gl_debug: bool,
    gl_robustness: Robustness,
    share: ffi::EGLContext,
) -> Result<ffi::egl::types::EGLContext, CreationError> {
    let egl = EGL.as_ref().unwrap();

    let mut context_attributes = Vec::with_capacity(10);
    let mut flags = 0;

    if egl_version >= &(1, 5)
        || extensions
            .iter()
            .find(|s| s == &"EGL_KHR_create_context")
            .is_some()
    {
        context_attributes.push(ffi::egl::CONTEXT_MAJOR_VERSION as i32);
        context_attributes.push(version.0 as i32);
        context_attributes.push(ffi::egl::CONTEXT_MINOR_VERSION as i32);
        context_attributes.push(version.1 as i32);

        // handling robustness
        let supports_robustness = egl_version >= &(1, 5)
            || extensions
                .iter()
                .find(|s| s == &"EGL_EXT_create_context_robustness")
                .is_some();

        match gl_robustness {
            Robustness::NotRobust => (),

            Robustness::NoError => {
                if extensions
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

        if gl_debug {
            if egl_version >= &(1, 5) {
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
    } else if egl_version >= &(1, 3) && api == Api::OpenGlEs {
        // robustness is not supported
        match gl_robustness {
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
        display,
        config_id,
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
