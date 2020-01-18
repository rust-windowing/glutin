#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod osmesa;

pub use crate::api::egl::ffi::{EGLConfig, EGLContext, EGLDisplay, EGLSurface};
pub use crate::api::glx::ffi::{Display as GLXDisplay, GLXContext, GLXDrawable, GLXFBConfig};
use crate::config::{Config, ConfigsFinder};
use crate::context::Context;
use crate::surface::{Surface, SurfaceTypeTrait};

use std::os::raw;

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NativeConfig {
    Egl(EGLConfig),
    Glx(GLXFBConfig),
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NativeDisplay {
    Egl(EGLDisplay),
    Glx(*mut GLXDisplay),
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NativeSurface {
    Egl(EGLSurface),
    Glx(GLXDrawable),
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NativeContext {
    Egl(EGLContext),
    Glx(GLXContext),
}

pub trait ConfigExt {
    fn config(&self) -> NativeConfig;
    fn display(&self) -> NativeDisplay;
}

impl ConfigExt for Config {
    #[inline]
    fn config(&self) -> NativeConfig {
        // FIXME
        unimplemented!()
    }
    #[inline]
    fn display(&self) -> NativeDisplay {
        // FIXME
        unimplemented!()
    }
}

pub trait SurfaceExt {
    fn surface(&self) -> NativeSurface;
}

impl<T: SurfaceTypeTrait> SurfaceExt for Surface<T> {
    #[inline]
    fn surface(&self) -> NativeSurface {
        // FIXME
        unimplemented!()
    }
}

pub trait ContextExt {
    fn context(&self) -> NativeContext;
}

impl ContextExt for Context {
    #[inline]
    fn context(&self) -> NativeContext {
        // FIXME
        unimplemented!()
    }
}

/// Which backing api should Glutin use. Wayland requires EGL.
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
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ConfigPlatformAttributes {
    pub x11_visual_xid: Option<raw::c_ulong>,
    pub x11_transparency: Option<bool>,
    pub backing_api: BackingApi,
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
