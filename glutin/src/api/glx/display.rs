//! GLX object creation.

use std::collections::HashSet;
use std::ffi::{self, CStr};
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use glutin_glx_sys::glx;
use glutin_glx_sys::glx::types::Display as GLXDisplay;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};

use crate::config::ConfigTemplate;
use crate::context::Version;
use crate::display::{
    AsRawDisplay, DisplayFeatures, DisplayResult, GetDisplayExtensions, RawDisplay,
};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{PbufferSurface, PixmapSurface, SurfaceAttributes, WindowSurface};

use super::config::Config;
use super::context::NotCurrentContext;
use super::surface::Surface;
use super::{Glx, GlxExtra, XlibErrorHookRegistrar, GLX, GLX_BASE_ERROR, GLX_EXTRA};

/// A wrapper for the `GLXDisplay`, which is basically an `XDisplay`.
#[derive(Debug)]
pub struct Display<D> {
    pub(crate) inner: Arc<DisplayInner<D>>,
}

impl<D> Clone for Display<D> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<D: HasDisplayHandle> AsRef<D> for Display<D> {
    fn as_ref(&self) -> &D {
        self.display()
    }
}

impl<D: HasDisplayHandle> HasDisplayHandle for Display<D> {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        self.display().display_handle()
    }
}

impl<D: HasDisplayHandle> Display<D> {
    /// Create GLX display.
    pub fn new(display_handle: D, error_hook_registrar: XlibErrorHookRegistrar) -> Result<Self> {
        Self::new_with_display(display_handle, error_hook_registrar).map_err(Into::into)
    }

    /// Get a reference to the inner display.
    pub fn display(&self) -> &D {
        &self.inner.display
    }

    pub(crate) fn new_with_display(
        display_handle: D,
        error_hook_registrar: XlibErrorHookRegistrar,
    ) -> DisplayResult<Self, D> {
        // Don't load GLX when unsupported platform was requested.
        let (display, screen) = match display_handle.display_handle().map(|w| w.as_raw()) {
            Ok(RawDisplayHandle::Xlib(handle)) => {
                let display = match handle.display {
                    Some(display) => display.as_ptr(),
                    None => return Err((ErrorKind::BadDisplay, display_handle).into()),
                };

                (GlxDisplay(display as *mut _), handle.screen as i32)
            },
            Ok(_) => {
                return Err((
                    ErrorKind::NotSupported("provided native display isn't supported"),
                    display_handle,
                )
                    .into())
            },
            Err(e) => return Err((crate::error::Error::from(e), display_handle).into()),
        };

        let glx = match GLX.as_ref() {
            Some(glx) => glx,
            None => return Err((ErrorKind::NotFound, display_handle).into()),
        };

        // Set the base for errors coming from GLX.
        unsafe {
            let mut error_base = 0;
            let mut event_base = 0;
            if glx.QueryExtension(display.0, &mut error_base, &mut event_base) == 0 {
                // The glx extension isn't present.
                return Err((ErrorKind::InitializationFailed, display_handle).into());
            }
            GLX_BASE_ERROR.store(error_base, Ordering::Relaxed);
        }

        // This is completely ridiculous, but VirtualBox's OpenGL driver needs
        // some call handled by *it* (i.e. not Mesa) to occur before
        // anything else can happen. That is because VirtualBox's OpenGL
        // driver is going to apply binary patches to Mesa in the DLL
        // constructor and until it's loaded it won't have a chance to do that.
        //
        // The easiest way to do this is to just call `glXQueryVersion()` before
        // doing anything else. See: https://www.virtualbox.org/ticket/8293
        let version = unsafe {
            let (mut major, mut minor) = (0, 0);
            if glx.QueryVersion(display.0, &mut major, &mut minor) == 0 {
                return Err((ErrorKind::InitializationFailed, display_handle).into());
            }
            Version::new(major as u8, minor as u8)
        };

        if version < Version::new(1, 3) {
            return Err((
                ErrorKind::NotSupported("the glx below 1.3 isn't supported"),
                display_handle,
            )
                .into());
        }

        // Register the error handling hook.
        error_hook_registrar(Box::new(super::glx_error_hook));

        let client_extensions = get_extensions(glx, display);
        let features = Self::extract_display_features(&client_extensions, version);

        let inner = Arc::new(DisplayInner {
            raw: display,
            glx,
            glx_extra: GLX_EXTRA.as_ref(),
            version,
            screen,
            features,
            client_extensions,
            display: display_handle,
        });

        Ok(Self { inner })
    }

    fn extract_display_features(
        extensions: &HashSet<&'static str>,
        version: Version,
    ) -> DisplayFeatures {
        let mut features = DisplayFeatures::empty();

        features.set(
            DisplayFeatures::MULTISAMPLING_PIXEL_FORMATS,
            version >= Version::new(1, 4) || extensions.contains("GLX_ARB_multisample"),
        );

        features.set(
            DisplayFeatures::FLOAT_PIXEL_FORMAT,
            extensions.contains("GLX_ARB_fbconfig_float"),
        );

        features.set(
            DisplayFeatures::SRGB_FRAMEBUFFERS,
            extensions.contains("GLX_ARB_framebuffer_sRGB")
                || extensions.contains("GLX_EXT_framebuffer_sRGB"),
        );

        features.set(
            DisplayFeatures::CREATE_ES_CONTEXT,
            extensions.contains("GLX_EXT_create_context_es2_profile")
                || extensions.contains("GLX_EXT_create_context_es_profile"),
        );

        features.set(
            DisplayFeatures::SWAP_CONTROL,
            extensions.contains("GLX_EXT_swap_control")
                || extensions.contains("GLX_SGI_swap_control")
                || extensions.contains("GLX_MESA_swap_control"),
        );

        features.set(
            DisplayFeatures::CONTEXT_ROBUSTNESS,
            extensions.contains("GLX_ARB_create_context_robustness"),
        );

        features.set(
            DisplayFeatures::CONTEXT_RELEASE_BEHAVIOR,
            extensions.contains("GLX_ARB_context_flush_control"),
        );

        features.set(
            DisplayFeatures::CONTEXT_NO_ERROR,
            extensions.contains("GLX_ARB_create_context_no_error"),
        );

        features
    }
}

impl<D: HasDisplayHandle> GlDisplay for Display<D> {
    type Config = Config<D>;
    type NotCurrentContext = NotCurrentContext<D>;
    type PbufferSurface = Surface<D, PbufferSurface>;
    type PixmapSurface = Surface<D, PixmapSurface>;
    type WindowSurface<W: HasWindowHandle> = Surface<D, WindowSurface<W>>;

    fn find_configs<W: HasWindowHandle>(
        &self,
        template: ConfigTemplate<W>,
    ) -> Result<Box<dyn Iterator<Item = Self::Config> + '_>> {
        Self::find_configs(self, template)
    }

    fn create_window_surface<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<WindowSurface<W>>,
    ) -> Result<Self::WindowSurface<W>> {
        Self::create_window_surface(self, config, surface_attributes)
    }

    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<PbufferSurface>,
    ) -> Result<Self::PbufferSurface> {
        unsafe { Self::create_pbuffer_surface(self, config, surface_attributes) }
    }

    fn create_context<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        context_attributes: &crate::context::ContextAttributes<W>,
    ) -> Result<Self::NotCurrentContext> {
        Self::create_context(self, config, context_attributes)
    }

    unsafe fn create_pixmap_surface(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<PixmapSurface>,
    ) -> Result<Self::PixmapSurface> {
        unsafe { Self::create_pixmap_surface(self, config, surface_attributes) }
    }

    fn get_proc_address(&self, addr: &CStr) -> *const ffi::c_void {
        unsafe { self.inner.glx.GetProcAddress(addr.as_ptr() as *const _) as *const _ }
    }

    fn version_string(&self) -> String {
        format!("GLX {}.{}", self.inner.version.major, self.inner.version.minor)
    }

    fn supported_features(&self) -> DisplayFeatures {
        self.inner.features
    }
}

impl<D: HasDisplayHandle> GetDisplayExtensions for Display<D> {
    fn extensions(&self) -> &HashSet<&'static str> {
        &self.inner.client_extensions
    }
}

impl<D: HasDisplayHandle> AsRawDisplay for Display<D> {
    fn raw_display(&self) -> RawDisplay {
        RawDisplay::Glx(self.inner.raw.cast())
    }
}

impl<D: HasDisplayHandle> Sealed for Display<D> {}

pub(crate) struct DisplayInner<D> {
    pub(crate) glx: &'static Glx,
    pub(crate) glx_extra: Option<&'static GlxExtra>,
    pub(crate) raw: GlxDisplay,
    pub(crate) screen: i32,
    pub(crate) version: Version,
    pub(crate) features: DisplayFeatures,
    /// Client GLX extensions.
    pub(crate) client_extensions: HashSet<&'static str>,

    /// Keep around a display reference so everything stays valid.
    display: D,
}

impl<D> fmt::Debug for DisplayInner<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Display")
            .field("raw", &self.raw)
            .field("version", &self.version)
            .field("screen", &self.screen)
            .field("features", &self.features)
            .field("extensions", &self.client_extensions)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GlxDisplay(*mut GLXDisplay);

unsafe impl Send for GlxDisplay {}
unsafe impl Sync for GlxDisplay {}

impl Deref for GlxDisplay {
    type Target = *mut GLXDisplay;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Load the GLX extensions.
fn get_extensions(glx: &Glx, display: GlxDisplay) -> HashSet<&'static str> {
    unsafe {
        let extensions = glx.GetClientString(display.0, glx::EXTENSIONS as i32);
        if extensions.is_null() {
            return HashSet::new();
        }

        if let Ok(extensions) = CStr::from_ptr(extensions).to_str() {
            extensions.split(' ').collect::<HashSet<_>>()
        } else {
            HashSet::new()
        }
    }
}
