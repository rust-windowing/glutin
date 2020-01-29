#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod osmesa;

use crate::config::{Config, ConfigsFinder};
use crate::context::Context;
use crate::surface::{Surface, SurfaceTypeTrait};

use std::os::raw;

/// The raw config type from the underlying API.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RawConfig {
    /// An EGLContext
    Egl(*mut raw::c_void),
    /// An GLXFBConfig
    Glx(*mut raw::c_void),
}

/// The raw display type from the underlying API.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RawDisplay {
    /// An EGLDisplay
    Egl(*mut raw::c_void),
    /// An Display
    Glx(*mut raw::c_void),
}

/// The raw surface type from the underlying API.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RawSurface {
    /// An EGLSurface
    Egl(*mut raw::c_void),
    /// An GLXDrawable
    Glx(*mut raw::c_void),
}

/// The raw context type from the underlying API.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RawContext {
    /// An EGLContext
    Egl(*mut raw::c_void),
    /// An GLXContext
    Glx(*mut raw::c_void),
}

/// An extention implemented on [`Config`] for getting the [`Config`]'s
/// [`RawConfig`] and [`RawDisplay`].
///
/// [`Config`]: crate::config::Config
/// [`RawConfig`]: crate::platform::unix::RawConfig
/// [`RawDisplay`]: crate::platform::unix::RawDisplay
pub trait ConfigExt {
    /// Returns this [`Config`]'s [`RawConfig`].
    ///
    /// # Saftey
    ///
    /// Should not outlive the underlying display. The underlying display is only
    /// released when all of the following have been released:
    ///
    ///  * This [`Config`],
    ///  * All the sister [`Config`]s returned alongside this [`Config`] by the [`ConfigsFinder`],
    ///  * All the [`Surface`]s and [`Context`]s made with this [`Config`]; and
    ///  * All the [`Surface`]s and [`Context`]s made with this [`Config`]'s sister [`Config`]s.
    ///
    /// [`Config`]: crate::config::Config
    /// [`ConfigsFinder`]: crate::config::ConfigsFinder
    /// [`Surface`]: crate::surface::Surface
    /// [`Context`]: crate::context::Context
    /// [`RawConfig`]: crate::platform::unix::RawConfig
    unsafe fn raw_config(&self) -> RawConfig;

    /// Returns this [`Config`]'s [`RawDisplay`].
    ///
    /// # Saftey
    ///
    /// See [`ConfigExt::config`].
    ///
    /// [`Config`]: crate::config::Config
    /// [`RawDisplay`]: crate::platform::unix::RawDisplay
    /// [`ConfigExt::config`]: crate::platform::unix::ConfigExt::config
    unsafe fn raw_display(&self) -> RawDisplay;
}

impl ConfigExt for Config {
    #[inline]
    unsafe fn raw_config(&self) -> RawConfig {
        self.config.raw_config()
    }

    #[inline]
    unsafe fn raw_display(&self) -> RawDisplay {
        self.config.raw_display()
    }
}

/// An extention implemented on [`Surface`] for getting the [`Surface`]'s
/// and [`RawSurface`].
///
/// [`Surface`]: crate::config::Surface
/// [`RawSurface`]: crate::platform::unix::RawSurface
pub trait SurfaceExt {
    /// Returns this [`Surface`]'s [`RawSurface`].
    ///
    /// # Saftey
    ///
    /// Should not outlive this [`Surface`].
    ///
    /// [`Surface`]: crate::config::Surface
    /// [`RawSurface`]: crate::platform::unix::RawSurface
    unsafe fn raw_surface(&self) -> RawSurface;
}

impl<T: SurfaceTypeTrait> SurfaceExt for Surface<T> {
    #[inline]
    unsafe fn raw_surface(&self) -> RawSurface {
        self.0.raw_surface()
    }
}

/// An extention implemented on [`Context`] for getting the [`Context`]'s
/// and [`RawContext`].
///
/// [`Context`]: crate::context::Context
/// [`RawContext`]: crate::platform::unix::RawContext
pub trait ContextExt {
    /// Returns this [`Context`]'s [`RawContext`].
    ///
    /// # Saftey
    ///
    /// Should not outlive this [`Context`].
    ///
    /// [`Context`]: crate::context::Context
    /// [`RawContext`]: crate::platform::unix::RawContext
    unsafe fn context(&self) -> RawContext;
}

impl ContextExt for Context {
    #[inline]
    unsafe fn context(&self) -> RawContext {
        self.0.raw_context()
    }
}

/// Which backing api should Glutin use. Non-X11 requires EGL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackingApi {
    GlxThenEgl,
    EglThenGlx,
    Egl,
    Glx,
}

impl Default for BackingApi {
    #[inline]
    fn default() -> Self {
        BackingApi::GlxThenEgl
    }
}

/// Platform specific config attributes for unix.
///
/// For details on what each member controls, please scroll through
/// [`ConfigPlatformAttributesExt`]'s [methods].
///
/// [`ConfigPlatformAttributesExt`]: crate::platform::unix::ConfigPlatformAttributesExt
/// [methods]: ./trait.ConfigPlatformAttributesExt.html#methods
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ConfigPlatformAttributes {
    pub x11_visual_xid: Option<raw::c_ulong>,
    pub x11_transparency: Option<bool>,
    pub backing_api: BackingApi,
}

impl Default for ConfigPlatformAttributes {
    #[inline]
    fn default() -> Self {
        ConfigPlatformAttributes {
            x11_transparency: Some(false),
            x11_visual_xid: None,
            backing_api: Default::default(),
        }
    }
}

/// A trait implemention functions for controlling unix's platform specific
/// config attributes.
pub trait ConfigPlatformAttributesExt {
    /// X11 only: set to insure a certain visual xid is used when
    /// choosing the fbconfig.
    fn with_x11_visual_xid(self, xid: Option<raw::c_ulong>) -> Self;

    /// X11 only: whether the X11 Visual will have transparency support.
    fn with_x11_transparency(self, trans: Option<bool>) -> Self;

    /// Wayland/X11 only. Wayland requires EGL.
    fn with_backing_api(self, backing_api: BackingApi) -> Self;
}

impl ConfigPlatformAttributesExt for ConfigsFinder {
    #[inline]
    fn with_x11_visual_xid(mut self, xid: Option<raw::c_ulong>) -> Self {
        self.plat_attr.x11_visual_xid = xid;
        self
    }

    #[inline]
    fn with_x11_transparency(mut self, trans: Option<bool>) -> Self {
        self.plat_attr.x11_transparency = trans;
        self
    }

    #[inline]
    fn with_backing_api(mut self, backing_api: BackingApi) -> Self {
        self.plat_attr.backing_api = backing_api;
        self
    }
}
