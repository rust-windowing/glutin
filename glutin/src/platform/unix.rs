#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod osmesa;

pub use crate::api::egl::ffi::EGLContext;
pub use crate::api::glx::ffi::glx::types::GLXContext;
use crate::config::ConfigBuilder;

use std::os::raw;

#[derive(Debug, Clone, Copy)]
pub enum BackingApi {
    GlxThenEgl,
    EglThenGlx,
    Egl,
    Glx,
}

impl Default for BackingApi {
    fn default() -> Self {
        BackingApi::GlxThenEgl
    }
}

#[derive(Default, Debug, Clone)]
pub struct ConfigPlatformAttributes {
    /// X11 only: set to insure a certain visual xid is used when
    /// choosing the fbconfig.
    pub x11_visual_xid: Option<raw::c_ulong>,

    /// Whether the X11 Visual will have transparency support.
    pub x11_transparency: Option<bool>,

    /// Wayland/X11 only.
    pub backing_api: BackingApi,
}

pub trait ConfigPlatformAttributesExt {
    fn with_x11_visual_xid(mut self, xid: Option<raw::c_ulong>) -> Self;
    fn with_x11_transparency(mut self, trans: Option<bool>) -> Self;
    fn with_backing_api(mut self, backing_api: BackingApi) -> Self;
}

impl ConfigPlatformAttributesExt for ConfigBuilder {
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
