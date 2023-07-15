//! A CGL display.

use std::ffi::{self, CStr};
use std::sync::Arc;

use core_foundation::base::TCFType;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};
use core_foundation::string::CFString;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle};

use crate::config::ConfigTemplate;
use crate::display::{AsRawDisplay, DisplayFeatures, RawDisplay};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{PbufferSurface, PixmapSurface, SurfaceAttributes, WindowSurface};

use super::config::Config;
use super::context::NotCurrentContext;
use super::surface::Surface;

/// The CGL display.
#[derive(Debug)]
pub struct Display<D> {
    /// The inner display object to keep alive.
    display: Arc<D>,
}

impl<D> Clone for Display<D> {
    fn clone(&self) -> Self {
        Self { display: self.display.clone() }
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

impl<D: HasDisplayHandle> Display<D> {
    /// Create CGL display.
    pub fn new(display: D) -> Result<Self> {
        match display.display_handle()?.as_raw() {
            RawDisplayHandle::AppKit(..) => Ok(Display { display: Arc::new(display) }),
            _ => Err(ErrorKind::NotSupported("provided native display is not supported").into()),
        }
    }

    /// Get the underlying display implementation
    pub fn display(&self) -> &D {
        &self.display
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
        let symbol_name = CFString::new(addr.to_str().unwrap());
        let framework_name = CFString::new("com.apple.opengl");
        unsafe {
            let framework = CFBundleGetBundleWithIdentifier(framework_name.as_concrete_TypeRef());
            CFBundleGetFunctionPointerForName(framework, symbol_name.as_concrete_TypeRef()).cast()
        }
    }

    fn version_string(&self) -> String {
        String::from("Apple CGL")
    }

    fn supported_features(&self) -> DisplayFeatures {
        DisplayFeatures::MULTISAMPLING_PIXEL_FORMATS
            | DisplayFeatures::FLOAT_PIXEL_FORMAT
            | DisplayFeatures::SRGB_FRAMEBUFFERS
            | DisplayFeatures::SWAP_CONTROL
    }
}

impl<D: HasDisplayHandle> AsRawDisplay for Display<D> {
    fn raw_display(&self) -> RawDisplay {
        RawDisplay::Cgl
    }
}

impl<D: HasDisplayHandle> Sealed for Display<D> {}
