//! WGL display initialization and extension loading.

use std::collections::HashSet;
use std::ffi::{CStr, OsStr};
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::HINSTANCE;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::System::LibraryLoader as dll_loader;

use crate::config::ConfigTemplate;
use crate::display::{AsRawDisplay, RawDisplay};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{PbufferSurface, PixmapSurface, SurfaceAttributes, WindowSurface};

use super::config::Config;
use super::context::NotCurrentContext;
use super::surface::Surface;
use super::WglExtra;

/// A WGL display.
#[derive(Debug, Clone)]
pub struct Display {
    pub(crate) inner: Arc<DisplayInner>,
}

impl Display {
    /// Create WGL display.
    ///
    /// The `native_window` is used to perform extension loading. If it's not
    /// passed the OpenGL will be limited to what it can do, though, basic
    /// operations could still be performed.
    ///
    /// # Safety
    ///
    /// The `native_window` must point to the valid platform window and have
    /// valid `hinstance`.
    pub unsafe fn from_raw(
        display: RawDisplayHandle,
        native_window: Option<RawWindowHandle>,
    ) -> Result<Self> {
        if !matches!(display, RawDisplayHandle::Windows(..)) {
            return Err(ErrorKind::NotSupported("provided native display is not supported").into());
        }

        let name =
            OsStr::new("opengl32.dll").encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>();
        let lib_opengl32 = dll_loader::LoadLibraryW(name.as_ptr());
        if lib_opengl32 == 0 {
            return Err(ErrorKind::NotFound.into());
        }

        // In case native window was provided init extra functions.
        let (wgl_extra, client_extensions) =
            if let Some(RawWindowHandle::Win32(window)) = native_window {
                let (wgl_extra, client_extensions) = super::load_extra_functions(window.hwnd as _)?;
                (Some(wgl_extra), client_extensions)
            } else {
                (None, HashSet::new())
            };

        let inner = Arc::new(DisplayInner { lib_opengl32, wgl_extra, client_extensions });
        Ok(Display { inner })
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
        Self::find_configs(self, template)
    }

    unsafe fn create_window_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Self::WindowSurface> {
        Self::create_window_surface(self, config, surface_attributes)
    }

    unsafe fn create_pbuffer_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Self::PbufferSurface> {
        Self::create_pbuffer_surface(self, config, surface_attributes)
    }

    unsafe fn create_context(
        &self,
        config: &Self::Config,
        context_attributes: &crate::context::ContextAttributes,
    ) -> Result<Self::NotCurrentContext> {
        Self::create_context(self, config, context_attributes)
    }

    unsafe fn create_pixmap_surface(
        &self,
        config: &Self::Config,
        surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Self::PixmapSurface> {
        Self::create_pixmap_surface(self, config, surface_attributes)
    }
}

impl AsRawDisplay for Display {
    fn raw_display(&self) -> RawDisplay {
        RawDisplay::Wgl
    }
}

impl Sealed for Display {}

pub(crate) struct DisplayInner {
    /// Client WGL extensions.
    pub(crate) lib_opengl32: HINSTANCE,

    /// Extra functions used by the impl.
    pub(crate) wgl_extra: Option<&'static WglExtra>,

    pub(crate) client_extensions: HashSet<&'static str>,
}

impl fmt::Debug for DisplayInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Display").field("extensions", &self.client_extensions).finish()
    }
}

pub(crate) fn load_extensions(hdc: HDC, wgl_extra: &WglExtra) -> HashSet<&'static str> {
    let extensions = unsafe {
        if wgl_extra.GetExtensionsStringARB.is_loaded() {
            CStr::from_ptr(wgl_extra.GetExtensionsStringARB(hdc as *const _))
        } else if wgl_extra.GetExtensionsStringEXT.is_loaded() {
            CStr::from_ptr(wgl_extra.GetExtensionsStringEXT())
        } else {
            return HashSet::new();
        }
    };

    if let Ok(extensions) = extensions.to_str() {
        extensions.split(' ').collect::<HashSet<_>>()
    } else {
        HashSet::new()
    }
}
