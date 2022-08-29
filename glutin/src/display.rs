//! The OpenGL platform display selection and creation.
#![allow(unreachable_patterns)]

use std::fmt;

use raw_window_handle::RawDisplayHandle;

use crate::config::{Config, ConfigTemplate, GlConfig};
use crate::context::{ContextAttributes, NotCurrentContext, NotCurrentGlContext};
use crate::error::Result;
use crate::private::{gl_api_dispatch, Sealed};
use crate::surface::{
    GlSurface, PbufferSurface, PixmapSurface, Surface, SurfaceAttributes, WindowSurface,
};

#[cfg(cgl_backend)]
use crate::api::cgl::display::Display as CglDisplay;
#[cfg(egl_backend)]
use crate::api::egl::display::Display as EglDisplay;
#[cfg(glx_backend)]
use crate::api::glx::display::Display as GlxDisplay;
#[cfg(glx_backend)]
use crate::api::glx::XlibErrorHookRegistrar;
#[cfg(wgl_backend)]
use crate::api::wgl::display::Display as WglDisplay;

/// A trait to group common display operations.
pub trait GlDisplay: Sealed {
    /// A window surface created by the display.
    type WindowSurface: GlSurface<WindowSurface>;
    /// A pixmap surface created by the display.
    type PixmapSurface: GlSurface<PixmapSurface>;
    /// A pbuffer surface created by the display.
    type PbufferSurface: GlSurface<PbufferSurface>;
    /// A config that is used by the display.
    type Config: GlConfig;
    /// A context that is being used by the display.
    type NotCurrentContext: NotCurrentGlContext;

    /// Find configurations matching the given `template`.
    ///
    /// # Safety
    ///
    /// Some platforms use [`RawWindowHandle`] to pick configs, so it
    /// must point to a valid object if it was passed on
    /// [`crate::config::ConfigTemplate`].
    ///
    /// [`RawWindowHandle`]: raw_window_handle::RawWindowHandle
    unsafe fn find_configs(
        &self,
        template: ConfigTemplate,
    ) -> Result<Box<dyn Iterator<Item = Self::Config> + '_>>;

    /// Create the graphics platform context.
    ///
    /// # Safety
    ///
    /// Some platforms use [`RawWindowHandle`] for context creation, so it must
    /// point to a valid object.
    ///
    /// [`RawWindowHandle`]: raw_window_handle::RawWindowHandle
    unsafe fn create_context(
        &self,
        config: &Self::Config,
        context_attributes: &ContextAttributes,
    ) -> Result<Self::NotCurrentContext>;

    /// Create the surface that can be used to render into native window.
    ///
    /// # Safety
    ///
    /// The [`RawWindowHandle`] must point to a valid object.
    ///
    /// [`RawWindowHandle`]: raw_window_handle::RawWindowHandle
    unsafe fn create_window_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Self::WindowSurface>;

    /// Create the surface that can be used to render into pbuffer.
    ///
    /// # Safety
    ///
    /// The function is safe in general, but marked as not for compatibility
    /// reasons.
    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Self::PbufferSurface>;

    /// Create the surface that can be used to render into pixmap.
    ///
    /// # Safety
    ///
    /// The [`NativePixmap`] must represent a valid native pixmap.
    ///
    /// [`NativePixmap`]: crate::surface::NativePixmap
    unsafe fn create_pixmap_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Self::PixmapSurface>;
}

/// Get the [`Display`].
pub trait GetGlDisplay: Sealed {
    /// The display used by the object.
    type Target: GlDisplay;

    /// Obtain the GL display used to create a particular GL object.
    fn display(&self) -> Self::Target;
}

/// Get the raw handle to the [`Display`].
pub trait AsRawDisplay {
    /// A raw handle to the underlying Api display.
    fn raw_display(&self) -> RawDisplay;
}

/// The graphics display to handle underlying graphics platform in a
/// cross-platform way.
///
/// The display can be accessed from any thread.
///
/// ```no_run
/// fn test_send<T: Send>() {}
/// fn test_sync<T: Sync>() {}
/// test_send::<glutin::display::Display>();
/// test_sync::<glutin::display::Display>();
/// ```
#[derive(Debug, Clone)]
pub enum Display {
    /// The EGL display.
    #[cfg(egl_backend)]
    Egl(EglDisplay),

    /// The GLX display.
    #[cfg(glx_backend)]
    Glx(GlxDisplay),

    /// The WGL display.
    #[cfg(wgl_backend)]
    Wgl(WglDisplay),

    /// The CGL display.
    #[cfg(cgl_backend)]
    Cgl(CglDisplay),
}

impl Display {
    /// Create a graphics platform display from the given raw display handle.
    ///
    /// The display mixing isn't supported, so if you created EGL display you
    /// can't use it with the GLX display objects. Interaction between those
    /// will result in a runtime panic.
    ///
    /// # Safety
    ///
    /// The `display` must point to the valid platform display and be valid for
    /// the entire lifetime of all Objects created with that display.
    ///
    /// The `picker` must contain pointers to the valid values if GLX or WGL
    /// specific options were used.
    pub unsafe fn from_raw(display: RawDisplayHandle, picker: DisplayPicker) -> Result<Self> {
        #[cfg(glx_backend)]
        let registrar = picker
            .glx_error_registrar
            .expect("glx was requested, but error hook registrar wasn't provided.");

        match picker.api_preference {
            #[cfg(egl_backend)]
            DisplayApiPreference::Egl => unsafe { Ok(Self::Egl(EglDisplay::from_raw(display)?)) },
            #[cfg(glx_backend)]
            DisplayApiPreference::Glx => unsafe {
                Ok(Self::Glx(GlxDisplay::from_raw(display, registrar)?))
            },
            #[cfg(wgl_backend)]
            DisplayApiPreference::Wgl => unsafe {
                Ok(Self::Wgl(WglDisplay::from_raw(display, picker.window_handle)?))
            },
            #[cfg(cgl_backend)]
            DisplayApiPreference::Cgl => Ok(Self::Cgl(CglDisplay::from_raw(display)?)),

            #[cfg(all(egl_backend, glx_backend))]
            DisplayApiPreference::EglThenGlx => unsafe {
                if let Ok(display) = EglDisplay::from_raw(display) {
                    Ok(Self::Egl(display))
                } else {
                    Ok(Self::Glx(GlxDisplay::from_raw(display, registrar)?))
                }
            },
            #[cfg(all(egl_backend, glx_backend))]
            DisplayApiPreference::GlxThenEgl => unsafe {
                if let Ok(display) = GlxDisplay::from_raw(display, registrar) {
                    Ok(Self::Glx(display))
                } else {
                    Ok(Self::Egl(EglDisplay::from_raw(display)?))
                }
            },

            #[cfg(all(egl_backend, wgl_backend))]
            DisplayApiPreference::EglThenWgl => unsafe {
                if let Ok(display) = EglDisplay::from_raw(display) {
                    Ok(Self::Egl(display))
                } else {
                    Ok(Self::Wgl(WglDisplay::from_raw(display, picker.window_handle)?))
                }
            },
            #[cfg(all(egl_backend, wgl_backend))]
            DisplayApiPreference::WglThenEgl => unsafe {
                if let Ok(display) = WglDisplay::from_raw(display, picker.window_handle) {
                    Ok(Self::Wgl(display))
                } else {
                    Ok(Self::Egl(EglDisplay::from_raw(display)?))
                }
            },
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
        match self {
            #[cfg(egl_backend)]
            Self::Egl(display) => unsafe {
                Ok(Box::new(display.find_configs(template)?.into_iter().map(Config::Egl)))
            },
            #[cfg(glx_backend)]
            Self::Glx(display) => unsafe {
                Ok(Box::new(display.find_configs(template)?.into_iter().map(Config::Glx)))
            },
            #[cfg(wgl_backend)]
            Self::Wgl(display) => unsafe {
                Ok(Box::new(display.find_configs(template)?.into_iter().map(Config::Wgl)))
            },
            #[cfg(cgl_backend)]
            Self::Cgl(display) => unsafe {
                Ok(Box::new(display.find_configs(template)?.into_iter().map(Config::Cgl)))
            },
        }
    }

    unsafe fn create_context(
        &self,
        config: &Self::Config,
        context_attributes: &ContextAttributes,
    ) -> Result<Self::NotCurrentContext> {
        match (self, config) {
            #[cfg(egl_backend)]
            (Self::Egl(display), Config::Egl(config)) => unsafe {
                Ok(NotCurrentContext::Egl(display.create_context(config, context_attributes)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(display), Config::Glx(config)) => unsafe {
                Ok(NotCurrentContext::Glx(display.create_context(config, context_attributes)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(display), Config::Wgl(config)) => unsafe {
                Ok(NotCurrentContext::Wgl(display.create_context(config, context_attributes)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(display), Config::Cgl(config)) => unsafe {
                Ok(NotCurrentContext::Cgl(display.create_context(config, context_attributes)?))
            },
            _ => unreachable!(),
        }
    }

    unsafe fn create_window_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Self::WindowSurface> {
        match (self, config) {
            #[cfg(egl_backend)]
            (Self::Egl(display), Config::Egl(config)) => unsafe {
                Ok(Surface::Egl(display.create_window_surface(config, surface_attributes)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(display), Config::Glx(config)) => unsafe {
                Ok(Surface::Glx(display.create_window_surface(config, surface_attributes)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(display), Config::Wgl(config)) => unsafe {
                Ok(Surface::Wgl(display.create_window_surface(config, surface_attributes)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(display), Config::Cgl(config)) => unsafe {
                Ok(Surface::Cgl(display.create_window_surface(config, surface_attributes)?))
            },
            _ => unreachable!(),
        }
    }

    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Self::PbufferSurface> {
        match (self, config) {
            #[cfg(egl_backend)]
            (Self::Egl(display), Config::Egl(config)) => unsafe {
                Ok(Surface::Egl(display.create_pbuffer_surface(config, surface_attributes)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(display), Config::Glx(config)) => unsafe {
                Ok(Surface::Glx(display.create_pbuffer_surface(config, surface_attributes)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(display), Config::Wgl(config)) => unsafe {
                Ok(Surface::Wgl(display.create_pbuffer_surface(config, surface_attributes)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(display), Config::Cgl(config)) => unsafe {
                Ok(Surface::Cgl(display.create_pbuffer_surface(config, surface_attributes)?))
            },
            _ => unreachable!(),
        }
    }

    unsafe fn create_pixmap_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Self::PixmapSurface> {
        match (self, config) {
            #[cfg(egl_backend)]
            (Self::Egl(display), Config::Egl(config)) => unsafe {
                Ok(Surface::Egl(display.create_pixmap_surface(config, surface_attributes)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(display), Config::Glx(config)) => unsafe {
                Ok(Surface::Glx(display.create_pixmap_surface(config, surface_attributes)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(display), Config::Wgl(config)) => unsafe {
                Ok(Surface::Wgl(display.create_pixmap_surface(config, surface_attributes)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(display), Config::Cgl(config)) => unsafe {
                Ok(Surface::Cgl(display.create_pixmap_surface(config, surface_attributes)?))
            },
            _ => unreachable!(),
        }
    }
}

impl AsRawDisplay for Display {
    fn raw_display(&self) -> RawDisplay {
        gl_api_dispatch!(self; Self(display) => display.raw_display())
    }
}

impl Sealed for Display {}

/// Settings to control automatic display picking.
pub struct DisplayPicker {
    pub(crate) api_preference: DisplayApiPreference,
    #[cfg(glx_backend)]
    pub(crate) glx_error_registrar: Option<XlibErrorHookRegistrar>,
    #[cfg(wgl_backend)]
    pub(crate) window_handle: Option<raw_window_handle::RawWindowHandle>,
}

impl DisplayPicker {
    /// Create a default display picker.
    pub fn new() -> Self {
        Default::default()
    }

    /// The preference of the underlying system Api.
    ///
    /// For platforms supporting `EGL` the default is `EGL` given that other api
    /// providers require extra options to get them work.
    pub fn with_api_preference(mut self, api_preference: DisplayApiPreference) -> Self {
        self.api_preference = api_preference;
        self
    }

    /// Create WGL display which will be the most suitable to operate with the
    /// given window.
    ///
    /// When the window isn't provided the support for extensions and pixel
    /// formats may be lacking.
    #[cfg(wgl_backend)]
    pub fn with_most_compatible_for_window(
        mut self,
        window_handle: raw_window_handle::RawWindowHandle,
    ) -> Self {
        self.window_handle = Some(window_handle);
        self
    }

    /// The hook to register glutin error handler in X11 error handling
    /// function.
    ///
    /// The hook registrar must be provided in case GLX will be used.
    #[cfg(glx_backend)]
    pub fn with_glx_error_registrar(mut self, error_registrar: XlibErrorHookRegistrar) -> Self {
        self.glx_error_registrar = Some(error_registrar);
        self
    }
}

impl Default for DisplayPicker {
    #[cfg(all(egl_backend, glx_backend))]
    fn default() -> Self {
        Self { api_preference: DisplayApiPreference::Egl, glx_error_registrar: None }
    }

    #[cfg(all(egl_backend, not(glx_backend), not(wgl_backend)))]
    fn default() -> Self {
        Self { api_preference: DisplayApiPreference::Egl }
    }

    #[cfg(all(glx_backend, not(egl_backend)))]
    fn default() -> Self {
        Self { api_preference: DisplayApiPreference::Glx, glx_error_registrar: None }
    }

    #[cfg(all(wgl_backend, not(egl_backend)))]
    fn default() -> Self {
        Self { api_preference: DisplayApiPreference::Wgl, window_handle: None }
    }

    #[cfg(all(wgl_backend, egl_backend))]
    fn default() -> Self {
        Self { api_preference: DisplayApiPreference::WglThenEgl, window_handle: None }
    }

    #[cfg(cgl_backend)]
    fn default() -> Self {
        Self { api_preference: DisplayApiPreference::Cgl }
    }
}

impl fmt::Debug for DisplayPicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("DisplayPicker");
        let f = f.field("api_preference", &self.api_preference);

        #[cfg(glx_backend)]
        let f = f.field("glx_registrar", &self.glx_error_registrar.is_some());

        #[cfg(wgl_backend)]
        let f = f.field("window_handle", &self.window_handle);

        f.finish()
    }
}

/// Preference of the display that should be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayApiPreference {
    /// Prefer EGL.
    #[cfg(egl_backend)]
    Egl,
    /// Prefer GLX.
    #[cfg(glx_backend)]
    Glx,
    /// Prefer WGL.
    #[cfg(wgl_backend)]
    Wgl,
    /// Prefer CGL.
    #[cfg(cgl_backend)]
    Cgl,

    /// Prefer EGL and fallback to GLX.
    #[cfg(all(egl_backend, glx_backend))]
    EglThenGlx,
    /// Prefer GLX and fallback to EGL.
    #[cfg(all(egl_backend, glx_backend))]
    GlxThenEgl,

    /// Prefer EGL and fallback to GLX.
    #[cfg(all(egl_backend, wgl_backend))]
    EglThenWgl,
    /// Prefer GLX and fallback to EGL.
    #[cfg(all(egl_backend, wgl_backend))]
    WglThenEgl,
}

/// Raw GL platform display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawDisplay {
    /// Raw EGL display.
    #[cfg(egl_backend)]
    Egl(*const std::ffi::c_void),

    /// Raw GLX display.
    #[cfg(glx_backend)]
    Glx(*const std::ffi::c_void),

    /// Raw display is WGL.
    #[cfg(wgl_backend)]
    Wgl,

    /// Raw display is CGL.
    #[cfg(cgl_backend)]
    Cgl,
}
