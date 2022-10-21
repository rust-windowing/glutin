//! A CGL display.

use std::ffi::{self, CStr};
use std::marker::PhantomData;

use core_foundation::base::TCFType;
use core_foundation::bundle::{CFBundleGetBundleWithIdentifier, CFBundleGetFunctionPointerForName};
use core_foundation::string::CFString;
use raw_window_handle::RawDisplayHandle;

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
#[derive(Debug, Clone)]
pub struct Display {
    // Prevent building of it without constructor.
    _marker: PhantomData<()>,
}

impl Display {
    /// Create CGL display.
    ///
    /// # Safety
    ///
    /// The function is unsafe for consistency.
    pub unsafe fn new(display: RawDisplayHandle) -> Result<Self> {
        match display {
            RawDisplayHandle::AppKit(..) => Ok(Display { _marker: PhantomData }),
            _ => Err(ErrorKind::NotSupported("provided native display is not supported").into()),
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

impl AsRawDisplay for Display {
    fn raw_display(&self) -> RawDisplay {
        RawDisplay::Cgl
    }
}

impl Sealed for Display {}
