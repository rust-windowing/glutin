//! The OpenGL platform display selection and creation.
#![allow(unreachable_patterns)]

use std::collections::HashSet;
use std::ffi::{self, CStr};
use std::fmt;

use bitflags::bitflags;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

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
    type WindowSurface<W: HasWindowHandle>: GlSurface<WindowSurface<W>>;
    /// A pixmap surface created by the display.
    type PixmapSurface: GlSurface<PixmapSurface>;
    /// A pbuffer surface created by the display.
    type PbufferSurface: GlSurface<PbufferSurface>;
    /// A config that is used by the display.
    type Config: GlConfig;
    /// A context that is being used by the display.
    type NotCurrentContext: NotCurrentGlContext;

    /// Find configurations matching the given `template`.
    fn find_configs<W: HasWindowHandle>(
        &self,
        template: ConfigTemplate<W>,
    ) -> Result<Box<dyn Iterator<Item = Self::Config> + '_>>;

    /// Create the graphics platform context.
    ///
    /// # Platform-specific
    ///
    /// - **Wayland:** this call may latch the underlying back buffer of the
    ///   currently active context (will do with mesa drivers), meaning that all
    ///   resize operations will apply to it after the next
    ///   [`GlSurface::swap_buffers`]. To workaround this behavior the current
    ///   context should be made [`not current`].
    ///
    /// [`not current`]: crate::context::PossiblyCurrentGlContext::make_not_current
    fn create_context<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        context_attributes: &ContextAttributes<W>,
    ) -> Result<Self::NotCurrentContext>;

    /// Create the surface that can be used to render into native window.
    fn create_window_surface<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<WindowSurface<W>>,
    ) -> Result<Self::WindowSurface<W>>;

    /// Create the surface that can be used to render into pbuffer.
    ///
    /// # Safety
    ///
    /// The function is safe in general, but marked as not for compatibility
    /// reasons.
    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<PbufferSurface>,
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
        surface_attributes: SurfaceAttributes<PixmapSurface>,
    ) -> Result<Self::PixmapSurface>;

    /// Return the address of an OpenGL function.
    ///
    /// # Api-specific
    ///
    /// - **WGL:** to load all the functions you must have a current context on
    ///   the calling thread, otherwise only a limited set of functions will be
    ///   loaded.
    fn get_proc_address(&self, addr: &CStr) -> *const ffi::c_void;

    /// Helper to obtain the information about the underlying display.
    ///
    /// This function is intended to be used for logging purposes to help with
    /// troubleshooting issues.
    fn version_string(&self) -> String;

    /// Get the features supported by the display.
    ///
    /// These features could be used to check that something is supported
    /// beforehand instead of doing fallback.
    fn supported_features(&self) -> DisplayFeatures;
}

/// Get the [`Display`].
pub trait GetGlDisplay: Sealed {
    /// The display used by the object.
    type Target: GlDisplay;

    /// Obtain the GL display used to create a particular GL object.
    fn display(&self) -> Self::Target;
}

/// Obtain the underlying api extensions.
pub trait GetDisplayExtensions: Sealed {
    /// Supported extensions by the display.
    ///
    /// # Api-specific
    ///
    /// - **WGL:** to have extensions loaded, `raw_window_handle` must be used
    ///   when creating the display.
    fn extensions(&self) -> &HashSet<&'static str>;
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
/// test_send::<glutin::display::Display<glutin::NoDisplay>>();
/// test_sync::<glutin::display::Display<glutin::NoDisplay>>();
/// ```
#[derive(Debug)]
pub enum Display<D> {
    /// The EGL display.
    #[cfg(egl_backend)]
    Egl(EglDisplay<D>),

    /// The GLX display.
    #[cfg(glx_backend)]
    Glx(GlxDisplay<D>),

    /// The WGL display.
    #[cfg(wgl_backend)]
    Wgl(WglDisplay<D>),

    /// The CGL display.
    #[cfg(cgl_backend)]
    Cgl(CglDisplay<D>),
}

impl<D> Clone for Display<D> {
    fn clone(&self) -> Self {
        match self {
            #[cfg(egl_backend)]
            Self::Egl(display) => Self::Egl(display.clone()),
            #[cfg(glx_backend)]
            Self::Glx(display) => Self::Glx(display.clone()),
            #[cfg(wgl_backend)]
            Self::Wgl(display) => Self::Wgl(display.clone()),
            #[cfg(cgl_backend)]
            Self::Cgl(display) => Self::Cgl(display.clone()),
        }
    }
}

impl<D: HasDisplayHandle> Display<D> {
    /// Create a graphics platform display from the given raw display handle.
    ///
    /// The display mixing isn't supported, so if you created EGL display you
    /// can't use it with the GLX display objects. Interaction between those
    /// will result in a runtime panic.
    pub fn new(display: D, preference: DisplayApiPreference<'_>) -> Result<Self> {
        match preference {
            #[cfg(egl_backend)]
            DisplayApiPreference::Egl => Ok(Self::Egl(EglDisplay::new(display)?)),
            #[cfg(glx_backend)]
            DisplayApiPreference::Glx(registrar) => {
                Ok(Self::Glx(GlxDisplay::new(display, registrar)?))
            },
            #[cfg(all(egl_backend, glx_backend))]
            DisplayApiPreference::GlxThenEgl(registrar) => {
                match GlxDisplay::new_with_display(display, registrar) {
                    Ok(display) => Ok(Self::Glx(display)),
                    Err(err) => Ok(Self::Egl(EglDisplay::new_with_display(err.display)?)),
                }
            },
            #[cfg(all(egl_backend, glx_backend))]
            DisplayApiPreference::EglThenGlx(registrar) => {
                match EglDisplay::new_with_display(display) {
                    Ok(display) => Ok(Self::Egl(display)),
                    Err(err) => {
                        Ok(Self::Glx(GlxDisplay::new_with_display(err.display, registrar)?))
                    },
                }
            },
            #[cfg(wgl_backend)]
            DisplayApiPreference::Wgl(window_handle) => {
                Ok(Self::Wgl(WglDisplay::new(display, window_handle)?))
            },
            #[cfg(all(egl_backend, wgl_backend))]
            DisplayApiPreference::EglThenWgl(window_handle) => {
                match EglDisplay::new_with_display(display) {
                    Ok(display) => Ok(Self::Egl(display)),
                    Err(err) => {
                        Ok(Self::Wgl(WglDisplay::new_with_display(err.display, window_handle)?))
                    },
                }
            },
            #[cfg(all(egl_backend, wgl_backend))]
            DisplayApiPreference::WglThenEgl(window_handle) => {
                match WglDisplay::new_with_display(display, window_handle) {
                    Ok(display) => Ok(Self::Wgl(display)),
                    Err(err) => Ok(Self::Egl(EglDisplay::new_with_display(err.display)?)),
                }
            },
            #[cfg(cgl_backend)]
            DisplayApiPreference::Cgl => Ok(Self::Cgl(CglDisplay::new(display)?)),
            DisplayApiPreference::__CaptureLifetime(_) => unreachable!(),
        }
    }

    /// Get the display underpinning this type.
    pub fn display(&self) -> &D {
        match self {
            #[cfg(egl_backend)]
            Self::Egl(display) => display.display(),
            #[cfg(glx_backend)]
            Self::Glx(display) => display.display(),
            #[cfg(wgl_backend)]
            Self::Wgl(display) => display.display(),
            #[cfg(cgl_backend)]
            Self::Cgl(display) => display.display(),
        }
    }
}

impl<D: HasDisplayHandle> AsRef<D> for Display<D> {
    #[inline]
    fn as_ref(&self) -> &D {
        self.display()
    }
}

impl<D: HasDisplayHandle> HasDisplayHandle for Display<D> {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        self.display().display_handle()
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
        match self {
            #[cfg(egl_backend)]
            Self::Egl(display) => Ok(Box::new(display.find_configs(template)?.map(Config::Egl))),
            #[cfg(glx_backend)]
            Self::Glx(display) => Ok(Box::new(display.find_configs(template)?.map(Config::Glx))),
            #[cfg(wgl_backend)]
            Self::Wgl(display) => Ok(Box::new(display.find_configs(template)?.map(Config::Wgl))),
            #[cfg(cgl_backend)]
            Self::Cgl(display) => Ok(Box::new(display.find_configs(template)?.map(Config::Cgl))),
        }
    }

    fn create_context<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        context_attributes: &ContextAttributes<W>,
    ) -> Result<Self::NotCurrentContext> {
        match (self, config) {
            #[cfg(egl_backend)]
            (Self::Egl(display), Config::Egl(config)) => {
                Ok(NotCurrentContext::Egl(display.create_context(config, context_attributes)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(display), Config::Glx(config)) => {
                Ok(NotCurrentContext::Glx(display.create_context(config, context_attributes)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(display), Config::Wgl(config)) => {
                Ok(NotCurrentContext::Wgl(display.create_context(config, context_attributes)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(display), Config::Cgl(config)) => {
                Ok(NotCurrentContext::Cgl(display.create_context(config, context_attributes)?))
            },
            _ => unreachable!(),
        }
    }

    fn create_window_surface<W: HasWindowHandle>(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<WindowSurface<W>>,
    ) -> Result<Self::WindowSurface<W>> {
        match (self, config) {
            #[cfg(egl_backend)]
            (Self::Egl(display), Config::Egl(config)) => {
                Ok(Surface::Egl(display.create_window_surface(config, surface_attributes)?))
            },
            #[cfg(glx_backend)]
            (Self::Glx(display), Config::Glx(config)) => {
                Ok(Surface::Glx(display.create_window_surface(config, surface_attributes)?))
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(display), Config::Wgl(config)) => {
                Ok(Surface::Wgl(display.create_window_surface(config, surface_attributes)?))
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(display), Config::Cgl(config)) => {
                Ok(Surface::Cgl(display.create_window_surface(config, surface_attributes)?))
            },
            _ => unreachable!(),
        }
    }

    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: SurfaceAttributes<PbufferSurface>,
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
        surface_attributes: SurfaceAttributes<PixmapSurface>,
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

    fn get_proc_address(&self, addr: &CStr) -> *const ffi::c_void {
        gl_api_dispatch!(self; Self(display) => display.get_proc_address(addr))
    }

    fn version_string(&self) -> String {
        gl_api_dispatch!(self; Self(display) => display.version_string())
    }

    fn supported_features(&self) -> DisplayFeatures {
        gl_api_dispatch!(self; Self(display) => display.supported_features())
    }
}

impl<D: HasDisplayHandle> AsRawDisplay for Display<D> {
    fn raw_display(&self) -> RawDisplay {
        gl_api_dispatch!(self; Self(display) => display.raw_display())
    }
}

impl<D: HasDisplayHandle> Sealed for Display<D> {}

/// Preference of the display that should be used.
pub enum DisplayApiPreference<'a> {
    /// Use only EGL.
    ///
    /// The EGL is a cross platform recent OpenGL platform. That being said
    /// it's usually lacking on Windows and not present at all on macOS
    /// natively.
    ///
    /// Be also aware that some features may not be present with it, like window
    /// transparency on X11 with mesa.
    ///
    /// But despite this issues it should be preferred on at least Linux over
    /// GLX, given that GLX is phasing away.
    ///
    /// # Platform-specific
    ///
    /// **Windows:** ANGLE can be used if `libEGL.dll` and `libGLESv2.dll` are
    ///              in the library search path.
    #[cfg(egl_backend)]
    Egl,

    /// Use only GLX.
    ///
    /// The native GLX platform, it's not very optimal since it's usually tied
    /// to Xlib. It's know to work fine, but be aware that you must register
    /// glutin with your X11  error handling callback, since it's a
    /// per-process global state.
    ///
    /// The hook to register glutin error handler in the X11 error handling
    /// function.
    #[cfg(glx_backend)]
    Glx(XlibErrorHookRegistrar),

    /// Use only WGL.
    ///
    /// The most spread platform on Windows and what should be used on it by
    /// default. EGL usually not present there so you'd have to account for that
    /// and create the window beforehand.
    ///
    /// When raw window handle isn't provided the display will lack extensions
    /// support and most features will be lacking.
    #[cfg(wgl_backend)]
    Wgl(Option<raw_window_handle::WindowHandle<'a>>),

    /// Use only CGL.
    ///
    /// The only option on macOS for now.
    #[cfg(cgl_backend)]
    Cgl,

    /// Prefer EGL and fallback to GLX.
    ///
    /// See [`Egl`] and [`Glx`] to decide what you want.
    ///
    /// [`Egl`]: Self::Egl
    /// [`Glx`]: Self::Glx
    #[cfg(all(egl_backend, glx_backend))]
    EglThenGlx(XlibErrorHookRegistrar),

    /// Prefer GLX and fallback to EGL.
    ///
    /// See [`Egl`] and [`Glx`] to decide what you want.
    ///
    /// [`Egl`]: Self::Egl
    /// [`Glx`]: Self::Glx
    #[cfg(all(egl_backend, glx_backend))]
    GlxThenEgl(XlibErrorHookRegistrar),

    /// Prefer EGL and fallback to WGL.
    ///
    /// See [`Egl`] and [`Wgl`] to decide what you want.
    ///
    /// [`Egl`]: Self::Egl
    /// [`Wgl`]: Self::Wgl
    #[cfg(all(egl_backend, wgl_backend))]
    EglThenWgl(Option<raw_window_handle::WindowHandle<'a>>),

    /// Prefer WGL and fallback to EGL.
    ///
    /// See [`Egl`] and [`Wgl`] to decide what you want.
    ///
    /// [`Egl`]: Self::Egl
    /// [`Wgl`]: Self::Wgl
    #[cfg(all(egl_backend, wgl_backend))]
    WglThenEgl(Option<raw_window_handle::WindowHandle<'a>>),

    /// Hidden option to capture the lifetime.
    #[doc(hidden)]
    __CaptureLifetime(std::marker::PhantomData<&'a ()>),
}

impl fmt::Debug for DisplayApiPreference<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let api = match self {
            #[cfg(egl_backend)]
            DisplayApiPreference::Egl => "Egl",
            #[cfg(glx_backend)]
            DisplayApiPreference::Glx(_) => "Glx",
            #[cfg(all(egl_backend, glx_backend))]
            DisplayApiPreference::GlxThenEgl(_) => "GlxThenEgl",
            #[cfg(all(egl_backend, glx_backend))]
            DisplayApiPreference::EglThenGlx(_) => "EglThenGlx",
            #[cfg(wgl_backend)]
            DisplayApiPreference::Wgl(_) => "Wgl",
            #[cfg(all(egl_backend, wgl_backend))]
            DisplayApiPreference::EglThenWgl(_) => "EglThenWgl",
            #[cfg(all(egl_backend, wgl_backend))]
            DisplayApiPreference::WglThenEgl(_) => "WglThenEgl",
            #[cfg(cgl_backend)]
            DisplayApiPreference::Cgl => "Cgl",
            DisplayApiPreference::__CaptureLifetime(_) => unreachable!(),
        };

        f.write_fmt(format_args!("DisplayApiPreference::{api}"))
    }
}

bitflags! {
    /// The features and extensions supported by the [`Display`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DisplayFeatures: u32 {
        /// The display supports creating [`robust`] context.
        ///
        /// [`robust`]: crate::context::Robustness
        const CONTEXT_ROBUSTNESS          = 0b0000_0001;

        /// The display supports creating [`no error`] context.
        ///
        /// [`no error`]: crate::context::Robustness::NoError
        const CONTEXT_NO_ERROR            = 0b0000_0010;

        /// The display supports [`floating`] pixel formats.
        ///
        /// [`floating`]: crate::config::ConfigTemplateBuilder::with_float_pixels
        const FLOAT_PIXEL_FORMAT          = 0b0000_0100;

        /// The display supports changing the [`swap interval`] on surfaces.
        ///
        /// [`swap interval`]: crate::surface::GlSurface::set_swap_interval
        const SWAP_CONTROL                = 0b0000_1000;

        /// The display supports creating context with explicit [`release behavior`].
        ///
        /// [`release behavior`]: crate::context::ReleaseBehavior
        const CONTEXT_RELEASE_BEHAVIOR   = 0b0001_0000;

        /// The display supports creating OpenGL ES [`context`].
        ///
        /// [`context`]: crate::context::ContextApi::Gles
        const CREATE_ES_CONTEXT           = 0b0010_0000;

        /// The display supports pixel formats with [`multisampling`].
        ///
        /// [`multisampling`]: crate::config::ConfigTemplateBuilder::with_multisampling
        const MULTISAMPLING_PIXEL_FORMATS = 0b0100_0000;

        /// The display supports creating surfaces backed by [`SRGB`] framebuffers.
        ///
        /// [`SRGB`]: crate::surface::SurfaceAttributesBuilder::with_srgb
        const SRGB_FRAMEBUFFERS           = 0b1000_0000;
    }
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

#[cfg_attr(cgl_backend, allow(dead_code))]
pub(crate) struct DisplayError<D> {
    /// The error that occurred.
    pub(crate) error: crate::error::Error,

    /// The display that caused the error.
    pub(crate) display: D,
}

impl<D> DisplayError<Option<D>> {
    #[allow(unused)]
    pub(crate) fn unwrap(self) -> DisplayError<D> {
        DisplayError { error: self.error, display: self.display.unwrap() }
    }
}

impl<D> From<DisplayError<D>> for crate::error::Error {
    fn from(value: DisplayError<D>) -> Self {
        value.error
    }
}

impl<D> From<(crate::error::Error, D)> for DisplayError<D> {
    fn from(value: (crate::error::Error, D)) -> Self {
        Self { error: value.0, display: value.1 }
    }
}

impl<D> From<(crate::error::ErrorKind, D)> for DisplayError<D> {
    fn from(value: (crate::error::ErrorKind, D)) -> Self {
        Self { error: value.0.into(), display: value.1 }
    }
}

#[cfg_attr(cgl_backend, allow(dead_code))]
pub(crate) type DisplayResult<T, D> = std::result::Result<T, DisplayError<D>>;
