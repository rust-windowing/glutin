//! Everything related to `EGLDisplay`.

use std::collections::HashSet;
use std::ffi::{self, CStr};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use glutin_egl_sys::egl;
use glutin_egl_sys::egl::types::{EGLAttrib, EGLDisplay, EGLint};

use once_cell::sync::OnceCell;

use raw_window_handle::RawDisplayHandle;

use crate::config::ConfigTemplate;
use crate::context::Version;
use crate::display::{AsRawDisplay, DisplayFeatures, GetDisplayExtensions, RawDisplay};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{PbufferSurface, PixmapSurface, SurfaceAttributes, WindowSurface};

use super::config::Config;
use super::context::NotCurrentContext;
use super::surface::Surface;

use super::{Egl, EGL};

/// Extensions that don't require any display.
static NO_DISPLAY_EXTENSIONS: OnceCell<HashSet<&'static str>> = OnceCell::new();

/// A wrapper for the `EGLDisplay` and its supported extensions.
#[derive(Debug, Clone)]
pub struct Display {
    // Inner display to simplify passing it around.
    pub(crate) inner: Arc<DisplayInner>,
}

impl Display {
    /// Create EGL display with the native display.
    ///
    /// # Safety
    ///
    /// `raw_display` must point to a valid system display. Using zero or
    /// `[std::ptr::null]` for the display will result in using
    /// `EGL_DEFAULT_DISPLAY`, which is not recommended or will
    /// work on a platform with a concept of native display, like Wayland.
    pub unsafe fn new(raw_display: RawDisplayHandle) -> Result<Self> {
        let egl = match EGL.as_ref() {
            Some(egl) => egl,
            None => return Err(ErrorKind::NotFound.into()),
        };

        NO_DISPLAY_EXTENSIONS.get_or_init(|| get_extensions(egl, egl::NO_DISPLAY));

        // Create a EGL display by chaining all display creation functions aborting on
        // `EGL_BAD_ATTRIBUTE`.
        let display = Self::get_platform_display(egl, raw_display)
            .or_else(|err| {
                if err.error_kind() == ErrorKind::BadAttribute {
                    Err(err)
                } else {
                    Self::get_platform_display_ext(egl, raw_display)
                }
            })
            .or_else(|err| {
                if err.error_kind() == ErrorKind::BadAttribute {
                    Err(err)
                } else {
                    Self::get_display(egl, raw_display)
                }
            })?;

        let version = unsafe {
            let (mut major, mut minor) = (0, 0);
            if egl.Initialize(display, &mut major, &mut minor) == egl::FALSE {
                return Err(super::check_error().err().unwrap());
            }

            Version::new(major as u8, minor as u8)
        };

        // Load extensions.
        let client_extensions = get_extensions(egl, display);
        let features = Self::extract_display_features(&client_extensions, version);

        let inner = Arc::new(DisplayInner {
            egl,
            raw: EglDisplay(display),
            _native_display: NativeDisplay(raw_display),
            version,
            features,
            client_extensions,
        });
        Ok(Self { inner })
    }

    fn get_platform_display(egl: &Egl, display: RawDisplayHandle) -> Result<EGLDisplay> {
        if !egl.GetPlatformDisplay.is_loaded() {
            return Err(ErrorKind::NotSupported("eglGetPlatformDisplay is not supported").into());
        }

        let extensions = NO_DISPLAY_EXTENSIONS.get().unwrap();

        let mut attrs = Vec::<EGLAttrib>::new();
        let (platform, mut display) = match display {
            #[cfg(wayland_platform)]
            RawDisplayHandle::Wayland(handle)
                if extensions.contains("EGL_KHR_platform_wayland") =>
            {
                (egl::PLATFORM_WAYLAND_KHR, handle.display)
            },
            #[cfg(x11_platform)]
            RawDisplayHandle::Xlib(handle) if extensions.contains("EGL_KHR_platform_x11") => {
                attrs.push(egl::PLATFORM_X11_SCREEN_KHR as EGLAttrib);
                attrs.push(handle.screen as EGLAttrib);
                (egl::PLATFORM_X11_KHR, handle.display)
            },
            RawDisplayHandle::Gbm(handle) if extensions.contains("EGL_KHR_platform_gbm") => {
                (egl::PLATFORM_GBM_KHR, handle.gbm_device)
            },
            RawDisplayHandle::Android(_) if extensions.contains("EGL_KHR_platform_android") => {
                (egl::PLATFORM_ANDROID_KHR, egl::DEFAULT_DISPLAY as *mut _)
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided display handle is not supported").into()
                )
            },
        };

        // Be explicit here.
        if display.is_null() {
            display = egl::DEFAULT_DISPLAY as *mut _;
        }

        // Push `egl::NONE` to terminate the list.
        attrs.push(egl::NONE as EGLAttrib);

        let display =
            unsafe { egl.GetPlatformDisplay(platform, display as *mut _, attrs.as_ptr()) };

        Self::check_display_error(display)
    }

    fn get_platform_display_ext(egl: &Egl, display: RawDisplayHandle) -> Result<EGLDisplay> {
        if !egl.GetPlatformDisplayEXT.is_loaded() {
            return Err(ErrorKind::NotSupported("eglGetPlatformDisplayEXT is not supported").into());
        }

        let extensions = NO_DISPLAY_EXTENSIONS.get().unwrap();

        let mut attrs = Vec::<EGLint>::new();
        let (platform, mut display) = match display {
            #[cfg(wayland_platform)]
            RawDisplayHandle::Wayland(handle)
                if extensions.contains("EGL_EXT_platform_wayland") =>
            {
                (egl::PLATFORM_WAYLAND_EXT, handle.display)
            },
            #[cfg(x11_platform)]
            RawDisplayHandle::Xlib(handle) if extensions.contains("EGL_EXT_platform_x11") => {
                attrs.push(egl::PLATFORM_X11_SCREEN_EXT as EGLint);
                attrs.push(handle.screen as EGLint);
                (egl::PLATFORM_X11_EXT, handle.display)
            },
            #[cfg(x11_platform)]
            RawDisplayHandle::Xcb(handle)
                if extensions.contains("EGL_MESA_platform_xcb")
                    || extensions.contains("EGL_EXT_platform_xcb") =>
            {
                attrs.push(egl::PLATFORM_XCB_EXT as EGLint);
                attrs.push(handle.screen as EGLint);
                (egl::PLATFORM_XCB_EXT, handle.connection)
            },
            RawDisplayHandle::Gbm(handle) if extensions.contains("EGL_MESA_platform_gbm") => {
                (egl::PLATFORM_GBM_MESA, handle.gbm_device)
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided display handle is not supported").into()
                )
            },
        };

        // Be explicit here.
        if display.is_null() {
            display = egl::DEFAULT_DISPLAY as *mut _;
        }

        // Push `egl::NONE` to terminate the list.
        attrs.push(egl::NONE as EGLint);

        let display =
            unsafe { egl.GetPlatformDisplayEXT(platform, display as *mut _, attrs.as_ptr()) };

        Self::check_display_error(display)
    }

    fn get_display(egl: &Egl, display: RawDisplayHandle) -> Result<EGLDisplay> {
        let mut display = match display {
            RawDisplayHandle::Gbm(handle) => handle.gbm_device,
            #[cfg(x11_platform)]
            RawDisplayHandle::Xlib(handle) => handle.display,
            RawDisplayHandle::Android(_) => egl::DEFAULT_DISPLAY as *mut _,
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided display handle is not supported").into()
                )
            },
        };

        if display.is_null() {
            display = egl::DEFAULT_DISPLAY as *mut _;
        }

        let display = unsafe { egl.GetDisplay(display) };
        Self::check_display_error(display)
    }

    fn extract_display_features(
        extensions: &HashSet<&'static str>,
        version: Version,
    ) -> DisplayFeatures {
        // Extract features.
        let mut supported_features = DisplayFeatures::CREATE_ES_CONTEXT
            | DisplayFeatures::MULTISAMPLING_PIXEL_FORMATS
            | DisplayFeatures::SWAP_CONTROL;

        supported_features.set(
            DisplayFeatures::FLOAT_PIXEL_FORMAT,
            extensions.contains("EGL_EXT_pixel_format_float"),
        );

        supported_features
            .set(DisplayFeatures::SRGB_FRAMEBUFFERS, extensions.contains("EGL_KHR_gl_colorspace"));

        let is_one_five = version >= Version::new(1, 5);

        supported_features.set(
            DisplayFeatures::CONTEXT_ROBUSTNESS,
            is_one_five || extensions.contains("EGL_EXT_create_context_robustness"),
        );

        supported_features.set(
            DisplayFeatures::CONTEXT_NO_ERROR,
            extensions.contains("EGL_KHR_create_context_no_error"),
        );

        supported_features
    }

    fn check_display_error(display: EGLDisplay) -> Result<EGLDisplay> {
        if display == egl::NO_DISPLAY {
            Err(super::check_error().err().unwrap())
        } else {
            Ok(display)
        }
    }
}

impl GlDisplay for Display {
    type Config = Config;
    type NotCurrentContext = NotCurrentContext;
    type PbufferSurface = Surface<PbufferSurface>;
    type PixmapSurface = Surface<PixmapSurface>;
    type WindowSurface = Surface<WindowSurface>;

    unsafe fn find_configs(
        &self,
        template: ConfigTemplate,
    ) -> Result<Box<dyn Iterator<Item = Self::Config> + '_>> {
        unsafe { Self::find_configs(self, template) }
    }

    unsafe fn create_window_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Self::WindowSurface> {
        unsafe { Self::create_window_surface(self, config, surface_attributes) }
    }

    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Self::PbufferSurface> {
        unsafe { Self::create_pbuffer_surface(self, config, surface_attributes) }
    }

    unsafe fn create_context(
        &self,
        config: &Self::Config,
        context_attributes: &crate::context::ContextAttributes,
    ) -> Result<Self::NotCurrentContext> {
        unsafe { Self::create_context(self, config, context_attributes) }
    }

    unsafe fn create_pixmap_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Self::PixmapSurface> {
        unsafe { Self::create_pixmap_surface(self, config, surface_attributes) }
    }

    fn get_proc_address(&self, addr: &CStr) -> *const ffi::c_void {
        unsafe { self.inner.egl.GetProcAddress(addr.as_ptr()) as *const _ }
    }

    fn version_string(&self) -> String {
        format!("EGL {}.{}", self.inner.version.major, self.inner.version.minor)
    }

    fn supported_features(&self) -> DisplayFeatures {
        self.inner.features
    }
}

impl GetDisplayExtensions for Display {
    fn extensions(&self) -> &HashSet<&'static str> {
        &self.inner.client_extensions
    }
}

impl AsRawDisplay for Display {
    fn raw_display(&self) -> RawDisplay {
        RawDisplay::Egl(*self.inner.raw)
    }
}

impl Sealed for Display {}

pub(crate) struct DisplayInner {
    /// Pointer to the EGL handler to simplify API calls.
    pub(crate) egl: &'static Egl,

    /// Pointer to the egl display.
    pub(crate) raw: EglDisplay,

    /// The version of the egl library.
    pub(crate) version: Version,

    /// Client EGL extensions.
    pub(crate) client_extensions: HashSet<&'static str>,

    /// The features supported by the display.
    pub(crate) features: DisplayFeatures,

    /// The raw display used to create EGL display.
    pub(crate) _native_display: NativeDisplay,
}

impl fmt::Debug for DisplayInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Display")
            .field("raw", &self.raw)
            .field("version", &self.version)
            .field("features", &self.features)
            .field("extensions", &self.client_extensions)
            .finish()
    }
}

impl Drop for DisplayInner {
    fn drop(&mut self) {
        // We cannot call safely call `eglTerminate`.
        //
        // This may sound confusing, but this is a result of how EGL works:
        //
        // From the documentation of `eglGetDisplay`:
        // > Multiple calls made to eglGetDisplay with the same display_id will
        // > return the same EGLDisplay handle.
        //
        // And from the documentation of `eglGetPlatformDisplay`:
        // > Multiple calls made to eglGetPlatformDisplay with the same
        // > parameters will return the same
        // > EGLDisplay handle.
        //
        // Furthermore the following is done when a display is initialized:
        // > Initializing an already initialized EGL display connection has no
        // > effect besides returning the
        // > version numbers.
        //
        // Terminating a display connection and then creating a new display
        // connection will reference the same display. This effectively
        // makes an EGLDisplay a singleton for the specified display_id or
        // platform and native display.
        //
        // Because EGLDisplay is a singleton, this makes the following sequence
        // problematic:
        //
        // 1. A display is created for a platform
        // 2. A second display is created for the same platform
        // 3. The first display is dropped, resulting in eglTerminate being
        // called.
        // 4. A context created from the second display is made
        // current, but it has been terminated and returns an EGL_BAD_DISPLAY
        // error.
        //
        // But wait? This causes a memory leak!
        //
        // Yes it does indeed result in a memory leak since we do not terminate
        // displays on drop. For most applications there is only ever a
        // single EGLDisplay for the lifetime of the application. The cost
        // of not dropping the display is negligible because the display will
        // probably be destroyed on app termination and we can let the
        // operating system deal with tearing down EGL instead.
        //
        // # Possible future work:
        //
        // For platform displays, we could track the use of individual raw
        // window handles and display attributes (recall the "with the
        // same parameters" line) and use that to determine if it is safe to
        // terminate the display, but that increases maintenance burden and is
        // possibly flaky to implement.

        // unsafe { self.egl.Terminate(self.raw) };
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeDisplay(RawDisplayHandle);

unsafe impl Send for NativeDisplay {}
unsafe impl Sync for NativeDisplay {}

impl Deref for NativeDisplay {
    type Target = RawDisplayHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EglDisplay(EGLDisplay);

// The EGL display could be shared between threads.
unsafe impl Send for EglDisplay {}
unsafe impl Sync for EglDisplay {}

impl Deref for EglDisplay {
    type Target = EGLDisplay;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Collect EGL extensions for the given `display`.
fn get_extensions(egl: &Egl, display: EGLDisplay) -> HashSet<&'static str> {
    unsafe {
        let extensions = egl.QueryString(display, egl::EXTENSIONS as i32);
        if extensions.is_null() {
            return HashSet::new();
        }

        if let Ok(extensions) = CStr::from_ptr(extensions).to_str() {
            extensions.split(' ').collect::<HashSet<&'static str>>()
        } else {
            HashSet::new()
        }
    }
}
