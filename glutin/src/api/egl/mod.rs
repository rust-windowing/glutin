#![cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#![allow(unused_variables)]

#[cfg(not(target_os = "android"))]
mod egl {
    use super::ffi;
    use crate::api::dlloader::{SymTrait, SymWrapper};

    #[derive(Clone)]
    pub struct Egl(pub SymWrapper<ffi::egl::Egl>);

    /// Because `*const libc::c_void` doesn't implement `Sync`.
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

pub use self::egl::Egl;
use crate::{
    Api, ContextError, CreationError, GlAttributes, GlRequest, PixelFormat,
    PixelFormatRequirements, ReleaseBehavior, Robustness,
};

use glutin_egl_sys as ffi;
#[cfg(any(target_os = "android", target_os = "windows"))]
use winit::dpi;

use std::cell::Cell;
use std::ffi::{CStr, CString};
use std::ops::{Deref, DerefMut};
use std::os::raw;

impl Deref for Egl {
    type Target = ffi::egl::Egl;

    fn deref(&self) -> &ffi::egl::Egl {
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

pub struct Context {
    display: ffi::egl::types::EGLDisplay,
    context: ffi::egl::types::EGLContext,
    surface: Cell<ffi::egl::types::EGLSurface>,
    api: Api,
    pixel_format: PixelFormat,
    #[cfg(target_os = "android")]
    config_id: ffi::egl::types::EGLConfig,
}

#[cfg(target_os = "android")]
#[inline]
fn get_native_display(egl: &Egl, ndisp: NativeDisplay) -> *const raw::c_void {
    unsafe { egl.GetDisplay(ffi::egl::DEFAULT_DISPLAY as *mut _) }
}

#[cfg(not(target_os = "android"))]
fn get_native_display(egl: &Egl, ndisp: NativeDisplay) -> *const raw::c_void {
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

    match ndisp {
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
    pub fn new<'a>(
        pf_reqs: &PixelFormatRequirements,
        opengl: &'a GlAttributes<&'a Context>,
        ndisp: NativeDisplay,
    ) -> Result<ContextPrototype<'a>, CreationError> {
        let egl = EGL.as_ref().unwrap();
        // calling `eglGetDisplay` or equivalent
        let display = get_native_display(egl, ndisp);

        if display.is_null() {
            return Err(CreationError::OsError(
                "Could not create EGL display object".to_string(),
            ));
        }

        let egl_version = unsafe {
            let mut major: ffi::egl::types::EGLint = std::mem::uninitialized();
            let mut minor: ffi::egl::types::EGLint = std::mem::uninitialized();

            if egl.Initialize(display, &mut major, &mut minor) == 0 {
                return Err(CreationError::OsError(format!(
                    "eglInitialize failed"
                )));
            }

            (major, minor)
        };

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

        // binding the right API and choosing the version
        let (version, api) = unsafe {
            match opengl.version {
                GlRequest::Latest => {
                    if egl_version >= (1, 4) {
                        if egl.BindAPI(ffi::egl::OPENGL_API) != 0 {
                            (None, Api::OpenGl)
                        } else if egl.BindAPI(ffi::egl::OPENGL_ES_API) != 0 {
                            (None, Api::OpenGlEs)
                        } else {
                            return Err(
                                CreationError::OpenGlVersionNotSupported,
                            );
                        }
                    } else {
                        (None, Api::OpenGlEs)
                    }
                }
                GlRequest::Specific(Api::OpenGlEs, version) => {
                    if egl_version >= (1, 2) {
                        if egl.BindAPI(ffi::egl::OPENGL_ES_API) == 0 {
                            return Err(
                                CreationError::OpenGlVersionNotSupported,
                            );
                        }
                    }
                    (Some(version), Api::OpenGlEs)
                }
                GlRequest::Specific(Api::OpenGl, version) => {
                    if egl_version < (1, 4) {
                        return Err(CreationError::OpenGlVersionNotSupported);
                    }
                    if egl.BindAPI(ffi::egl::OPENGL_API) == 0 {
                        return Err(CreationError::OpenGlVersionNotSupported);
                    }
                    (Some(version), Api::OpenGl)
                }
                GlRequest::Specific(_, _) => {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
                GlRequest::GlThenGles {
                    opengles_version,
                    opengl_version,
                } => {
                    if egl_version >= (1, 4) {
                        if egl.BindAPI(ffi::egl::OPENGL_API) != 0 {
                            (Some(opengl_version), Api::OpenGl)
                        } else if egl.BindAPI(ffi::egl::OPENGL_ES_API) != 0 {
                            (Some(opengles_version), Api::OpenGlEs)
                        } else {
                            return Err(
                                CreationError::OpenGlVersionNotSupported,
                            );
                        }
                    } else {
                        (Some(opengles_version), Api::OpenGlEs)
                    }
                }
            }
        };

        let (config_id, pixel_format) = unsafe {
            choose_fbconfig(egl, display, &egl_version, api, version, pf_reqs)?
        };

        Ok(ContextPrototype {
            opengl,
            display,
            egl_version,
            extensions,
            api,
            version,
            config_id,
            pixel_format,
        })
    }

    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();
        let ret = egl.MakeCurrent(
            self.display,
            self.surface.get(),
            self.surface.get(),
            self.context,
        );

        if ret == 0 {
            match egl.GetError() as u32 {
                ffi::egl::CONTEXT_LOST => {
                    return Err(ContextError::ContextLost)
                }
                err => panic!(
                    "make_current: eglMakeCurrent failed (eglGetError returned 0x{:x})",
                    err
                ),
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        let egl = EGL.as_ref().unwrap();
        unsafe { egl.GetCurrentContext() == self.context }
    }

    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let egl = EGL.as_ref().unwrap();
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        unsafe { egl.GetProcAddress(addr) as *const _ }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        let egl = EGL.as_ref().unwrap();
        if self.surface.get() == ffi::egl::NO_SURFACE {
            return Err(ContextError::ContextLost);
        }

        let ret = unsafe { egl.SwapBuffers(self.display, self.surface.get()) };

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
    pub fn get_api(&self) -> Api {
        self.api
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pixel_format.clone()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::egl::types::EGLContext {
        self.context
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> ffi::egl::types::EGLDisplay {
        self.display
    }

    // Handle Android Life Cycle.
    // Android has started the activity or sent it to foreground.
    // Create a new surface and attach it to the recreated ANativeWindow.
    // Restore the EGLContext.
    #[cfg(target_os = "android")]
    pub unsafe fn on_surface_created(&self, nwin: ffi::EGLNativeWindowType) {
        let egl = EGL.as_ref().unwrap();
        if self.surface.get() != ffi::egl::NO_SURFACE {
            return;
        }
        self.surface.set(egl.CreateWindowSurface(
            self.display,
            self.config_id,
            nwin,
            std::ptr::null(),
        ));
        if self.surface.get().is_null() {
            panic!(
                "on_surface_created: eglCreateWindowSurface failed with 0x{:x}",
                egl.GetError()
            )
        }
        let ret = egl.MakeCurrent(
            self.display,
            self.surface.get(),
            self.surface.get(),
            self.context,
        );
        if ret == 0 {
            panic!(
                "on_surface_created: eglMakeCurrent failed with 0x{:x}",
                egl.GetError()
            )
        }
    }

    // Handle Android Life Cycle.
    // Android has stopped the activity or sent it to background.
    // Release the surface attached to the destroyed ANativeWindow.
    // The EGLContext is not destroyed so it can be restored later.
    #[cfg(target_os = "android")]
    pub unsafe fn on_surface_destroyed(&self) {
        let egl = EGL.as_ref().unwrap();
        if self.surface.get() == ffi::egl::NO_SURFACE {
            return;
        }
        let ret = egl.MakeCurrent(
            self.display,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_SURFACE,
            ffi::egl::NO_CONTEXT,
        );
        if ret == 0 {
            panic!(
                "on_surface_destroyed: eglMakeCurrent failed with 0x{:x}",
                egl.GetError()
            )
        }

        egl.DestroySurface(self.display, self.surface.get());
        self.surface.set(ffi::egl::NO_SURFACE);
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Drop for Context {
    fn drop(&mut self) {
        // https://stackoverflow.com/questions/54402688/recreate-eglcreatewindowsurface-with-same-native-window
        let egl = EGL.as_ref().unwrap();
        unsafe {
            // Ok, so we got to call `glFinish` before destroying the context to
            // insure it actually gets destroyed. This requires making the this
            // context current.
            //
            // Now, if the user has multiple contexts, and they drop this one
            // unintentionally between calls to the other context, this could
            // result in a !FUN! time debuging.
            //
            // Then again, if they're **unintentionally** dropping contexts, I
            // think they got bigger problems.
            self.make_current().unwrap();

            let gl_finish_fn = self.get_proc_address("glFinish");
            assert!(gl_finish_fn != std::ptr::null());
            let gl_finish_fn =
                std::mem::transmute::<_, extern "system" fn()>(gl_finish_fn);
            gl_finish_fn();

            let ret = egl.MakeCurrent(
                self.display,
                ffi::egl::NO_SURFACE,
                ffi::egl::NO_SURFACE,
                ffi::egl::NO_CONTEXT,
            );
            if ret == 0 {
                panic!(
                    "drop: eglMakeCurrent failed with 0x{:x}",
                    egl.GetError()
                )
            }
            egl.DestroyContext(self.display, self.context);
            egl.DestroySurface(self.display, self.surface.get());
            egl.Terminate(self.display);
        }
    }
}

pub struct ContextPrototype<'a> {
    opengl: &'a GlAttributes<&'a Context>,
    display: ffi::egl::types::EGLDisplay,
    egl_version: (ffi::egl::types::EGLint, ffi::egl::types::EGLint),
    extensions: Vec<String>,
    api: Api,
    version: Option<(u8, u8)>,
    config_id: ffi::egl::types::EGLConfig,
    pixel_format: PixelFormat,
}

impl<'a> ContextPrototype<'a> {
    pub fn get_native_visual_id(&self) -> ffi::egl::types::EGLint {
        let egl = EGL.as_ref().unwrap();
        let mut value = unsafe { std::mem::uninitialized() };
        let ret = unsafe {
            egl.GetConfigAttrib(
                self.display,
                self.config_id,
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

    pub fn finish(
        self,
        nwin: ffi::EGLNativeWindowType,
    ) -> Result<Context, CreationError> {
        let egl = EGL.as_ref().unwrap();
        let surface = unsafe {
            let surface = egl.CreateWindowSurface(
                self.display,
                self.config_id,
                nwin,
                std::ptr::null(),
            );
            if surface.is_null() {
                return Err(CreationError::OsError(format!(
                    "eglCreateWindowSurface failed"
                )));
            }
            surface
        };

        self.finish_impl(surface)
    }

    #[cfg(any(target_os = "android", target_os = "windows"))]
    pub fn finish_pbuffer(
        self,
        dims: dpi::PhysicalSize,
    ) -> Result<Context, CreationError> {
        let dims: (u32, u32) = dims.into();

        let egl = EGL.as_ref().unwrap();
        let attrs = &[
            ffi::egl::WIDTH as raw::c_int,
            dims.0 as raw::c_int,
            ffi::egl::HEIGHT as raw::c_int,
            dims.1 as raw::c_int,
            ffi::egl::NONE as raw::c_int,
        ];

        let surface = unsafe {
            let surface = egl.CreatePbufferSurface(
                self.display,
                self.config_id,
                attrs.as_ptr(),
            );
            if surface.is_null() {
                return Err(CreationError::OsError(format!(
                    "eglCreatePbufferSurface failed"
                )));
            }
            surface
        };

        self.finish_impl(surface)
    }

    fn finish_impl(
        self,
        surface: ffi::egl::types::EGLSurface,
    ) -> Result<Context, CreationError> {
        let share = match self.opengl.sharing {
            Some(ctx) => ctx.context,
            None => std::ptr::null(),
        };

        let context = unsafe {
            if let Some(version) = self.version {
                create_context(
                    self.display,
                    &self.egl_version,
                    &self.extensions,
                    self.api,
                    version,
                    self.config_id,
                    self.opengl.debug,
                    self.opengl.robustness,
                    share,
                )?
            } else if self.api == Api::OpenGlEs {
                if let Ok(ctx) = create_context(
                    self.display,
                    &self.egl_version,
                    &self.extensions,
                    self.api,
                    (2, 0),
                    self.config_id,
                    self.opengl.debug,
                    self.opengl.robustness,
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    self.display,
                    &self.egl_version,
                    &self.extensions,
                    self.api,
                    (1, 0),
                    self.config_id,
                    self.opengl.debug,
                    self.opengl.robustness,
                    share,
                ) {
                    ctx
                } else {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
            } else {
                if let Ok(ctx) = create_context(
                    self.display,
                    &self.egl_version,
                    &self.extensions,
                    self.api,
                    (3, 2),
                    self.config_id,
                    self.opengl.debug,
                    self.opengl.robustness,
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    self.display,
                    &self.egl_version,
                    &self.extensions,
                    self.api,
                    (3, 1),
                    self.config_id,
                    self.opengl.debug,
                    self.opengl.robustness,
                    share,
                ) {
                    ctx
                } else if let Ok(ctx) = create_context(
                    self.display,
                    &self.egl_version,
                    &self.extensions,
                    self.api,
                    (1, 0),
                    self.config_id,
                    self.opengl.debug,
                    self.opengl.robustness,
                    share,
                ) {
                    ctx
                } else {
                    return Err(CreationError::OpenGlVersionNotSupported);
                }
            }
        };

        Ok(Context {
            display: self.display,
            context,
            surface: Cell::new(surface),
            api: self.api,
            pixel_format: self.pixel_format,
            #[cfg(target_os = "android")]
            config_id: self.config_id,
        })
    }
}

unsafe fn choose_fbconfig(
    egl: &Egl,
    display: ffi::egl::types::EGLDisplay,
    egl_version: &(ffi::egl::types::EGLint, ffi::egl::types::EGLint),
    api: Api,
    version: Option<(u8, u8)>,
    reqs: &PixelFormatRequirements,
) -> Result<(ffi::egl::types::EGLConfig, PixelFormat), CreationError> {
    let descriptor = {
        let mut out: Vec<raw::c_int> = Vec::with_capacity(37);

        if egl_version >= &(1, 2) {
            out.push(ffi::egl::COLOR_BUFFER_TYPE as raw::c_int);
            out.push(ffi::egl::RGB_BUFFER as raw::c_int);
        }

        out.push(ffi::egl::SURFACE_TYPE as raw::c_int);
        // TODO: Some versions of Mesa report a BAD_ATTRIBUTE error
        // if we ask for PBUFFER_BIT as well as WINDOW_BIT
        out.push((ffi::egl::WINDOW_BIT) as raw::c_int);

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

        if let Some(hardware_accelerated) = reqs.hardware_accelerated {
            out.push(ffi::egl::CONFIG_CAVEAT as raw::c_int);
            out.push(if hardware_accelerated {
                ffi::egl::NONE as raw::c_int
            } else {
                ffi::egl::SLOW_CONFIG as raw::c_int
            });
        }

        if let Some(color) = reqs.color_bits {
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

        if let Some(alpha) = reqs.alpha_bits {
            out.push(ffi::egl::ALPHA_SIZE as raw::c_int);
            out.push(alpha as raw::c_int);
        }

        if let Some(depth) = reqs.depth_bits {
            out.push(ffi::egl::DEPTH_SIZE as raw::c_int);
            out.push(depth as raw::c_int);
        }

        if let Some(stencil) = reqs.stencil_bits {
            out.push(ffi::egl::STENCIL_SIZE as raw::c_int);
            out.push(stencil as raw::c_int);
        }

        if let Some(true) = reqs.double_buffer {
            return Err(CreationError::NoAvailablePixelFormat);
        }

        if let Some(multisampling) = reqs.multisampling {
            out.push(ffi::egl::SAMPLES as raw::c_int);
            out.push(multisampling as raw::c_int);
        }

        if reqs.stereoscopy {
            return Err(CreationError::NoAvailablePixelFormat);
        }

        if let Some(xid) = reqs.x11_visual_xid {
            out.push(ffi::egl::NATIVE_VISUAL_ID as raw::c_int);
            out.push(xid as raw::c_int);
        }

        // FIXME: srgb is not taken into account

        match reqs.release_behavior {
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
    let mut config_id = std::mem::uninitialized();
    let mut num_configs = std::mem::uninitialized();
    if egl.ChooseConfig(
        display,
        descriptor.as_ptr(),
        &mut config_id,
        1,
        &mut num_configs,
    ) == 0
    {
        return Err(CreationError::OsError(format!("eglChooseConfig failed")));
    }
    if num_configs == 0 {
        return Err(CreationError::NoAvailablePixelFormat);
    }

    // analyzing each config
    macro_rules! attrib {
        ($egl:expr, $display:expr, $config:expr, $attr:expr) => {{
            let mut value = std::mem::uninitialized();
            let res = $egl.GetConfigAttrib(
                $display,
                $config,
                $attr as ffi::egl::types::EGLint,
                &mut value,
            );
            if res == 0 {
                return Err(CreationError::OsError(format!(
                    "eglGetConfigAttrib failed"
                )));
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

            // TODO: using this flag sometimes generates an error
            //       there was a change in the specs that added this flag, so it
            // may not be       supported everywhere ; however it is
            // not possible to know whether it is       supported or
            // not flags = flags |
            // ffi::egl::CONTEXT_OPENGL_DEBUG_BIT_KHR as i32;
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
