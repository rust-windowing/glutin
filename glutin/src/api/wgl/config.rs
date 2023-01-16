//! Handling of PIXELFORMATDESCRIPTOR and pixel format index.

use std::io::Error as IoError;
use std::mem::{self, MaybeUninit};
use std::os::raw::c_int;
use std::sync::Arc;
use std::{fmt, iter};

use glutin_wgl_sys::wgl_extra;
use raw_window_handle::RawWindowHandle;
use windows_sys::Win32::Graphics::Gdi::{self as gdi, HDC};
use windows_sys::Win32::Graphics::OpenGL::{self as gl, PIXELFORMATDESCRIPTOR};

use crate::config::{
    Api, AsRawConfig, ColorBufferType, ConfigSurfaceTypes, ConfigTemplate, GlConfig, RawConfig,
};
use crate::display::{DisplayFeatures, GetGlDisplay};
use crate::error::{ErrorKind, Result};
use crate::private::Sealed;

use super::display::Display;

/// The maximum amount of configs to query.
const MAX_QUERY_CONFIGS: usize = 256;

// Srgb extensions.
const SRGB_ARB: &str = "WGL_ARB_framebuffer_sRGB";
const SRGB_EXT: &str = "WGL_EXT_framebuffer_sRGB";

impl Display {
    pub(crate) unsafe fn find_configs(
        &self,
        template: ConfigTemplate,
    ) -> Result<Box<dyn Iterator<Item = Config> + '_>> {
        let hwnd = match template.native_window {
            Some(RawWindowHandle::Win32(window_handle)) => window_handle.hwnd as _,
            _ => 0,
        };
        let hdc = unsafe { gdi::GetDC(hwnd) };

        match self.inner.wgl_extra {
            // Check that particular function was loaded.
            Some(wgl_extra) if wgl_extra.ChoosePixelFormatARB.is_loaded() => {
                self.find_configs_arb(template, hdc)
            },
            _ => self.find_normal_configs(template, hdc),
        }
    }

    fn find_normal_configs(
        &self,
        template: ConfigTemplate,
        hdc: HDC,
    ) -> Result<Box<dyn Iterator<Item = Config> + '_>> {
        let (r_size, g_size, b_size) = match template.color_buffer_type {
            ColorBufferType::Rgb { r_size, g_size, b_size } => (r_size, g_size, b_size),
            _ => {
                return Err(
                    ErrorKind::NotSupported("luminance buffers are not supported with WGL").into()
                )
            },
        };

        let mut dw_flags = gl::PFD_SUPPORT_OPENGL;
        if !template.single_buffering {
            dw_flags |= gl::PFD_DOUBLEBUFFER;
        }

        if template.config_surface_types.contains(ConfigSurfaceTypes::WINDOW) {
            dw_flags |= gl::PFD_DRAW_TO_WINDOW;
        }

        if template.config_surface_types.contains(ConfigSurfaceTypes::PIXMAP) {
            dw_flags |= gl::PFD_DRAW_TO_BITMAP;
        }

        dw_flags |= match template.stereoscopy {
            Some(true) => gl::PFD_STEREO,
            Some(false) => 0,
            None => gl::PFD_STEREO_DONTCARE,
        };

        // Hardware acceleration.
        dw_flags |= match template.hardware_accelerated {
            Some(true) => gl::PFD_GENERIC_ACCELERATED,
            Some(false) => gl::PFD_GENERIC_FORMAT,
            None => 0,
        };

        let pixel_format_descriptor = PIXELFORMATDESCRIPTOR {
            nSize: mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
            // Should be one according to the docs.
            nVersion: 1,
            dwFlags: dw_flags,
            iPixelType: gl::PFD_TYPE_RGBA,
            cColorBits: r_size + g_size + b_size,
            cRedBits: r_size,
            cRedShift: 0,
            cGreenBits: g_size,
            cGreenShift: 0,
            cBlueBits: b_size,
            cBlueShift: 0,
            cAlphaBits: template.alpha_size,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: template.depth_size,
            cStencilBits: template.stencil_size,
            cAuxBuffers: 0,
            iLayerType: gl::PFD_MAIN_PLANE,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };

        unsafe {
            let pixel_format_index = gl::ChoosePixelFormat(hdc, &pixel_format_descriptor);
            if pixel_format_index == 0 {
                return Err(ErrorKind::BadConfig.into());
            }

            let mut descriptor = MaybeUninit::<PIXELFORMATDESCRIPTOR>::uninit();
            if gl::DescribePixelFormat(
                hdc,
                pixel_format_index as _,
                mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
                descriptor.as_mut_ptr(),
            ) == 0
            {
                return Err(IoError::last_os_error().into());
            };

            let descriptor = descriptor.assume_init();

            if descriptor.iPixelType != gl::PFD_TYPE_RGBA {
                return Err(ErrorKind::BadConfig.into());
            }

            let inner = Arc::new(ConfigInner {
                display: self.clone(),
                hdc,
                pixel_format_index,
                descriptor: Some(descriptor),
            });
            let config = Config { inner };

            Ok(Box::new(iter::once(config)))
        }
    }

    fn find_configs_arb(
        &self,
        template: ConfigTemplate,
        hdc: HDC,
    ) -> Result<Box<dyn Iterator<Item = Config> + '_>> {
        let wgl_extra = self.inner.wgl_extra.unwrap();
        let mut attrs = Vec::<c_int>::with_capacity(32);

        match template.color_buffer_type {
            ColorBufferType::Rgb { r_size, g_size, b_size } => {
                attrs.push(wgl_extra::RED_BITS_ARB as c_int);
                attrs.push(r_size as c_int);
                attrs.push(wgl_extra::GREEN_BITS_ARB as c_int);
                attrs.push(g_size as c_int);
                attrs.push(wgl_extra::BLUE_BITS_ARB as c_int);
                attrs.push(b_size as c_int);
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("luminance buffers are not supported with WGL").into()
                )
            },
        }

        attrs.push(wgl_extra::ALPHA_BITS_ARB as c_int);
        attrs.push(template.alpha_size as c_int);

        attrs.push(wgl_extra::DEPTH_BITS_ARB as c_int);
        attrs.push(template.depth_size as c_int);

        attrs.push(wgl_extra::STENCIL_BITS_ARB as c_int);
        attrs.push(template.stencil_size as c_int);

        attrs.push(wgl_extra::SUPPORT_OPENGL_ARB as c_int);
        attrs.push(1);

        attrs.push(wgl_extra::DOUBLE_BUFFER_ARB as c_int);
        attrs.push(!template.single_buffering as c_int);

        let pixel_type = if self.inner.features.contains(DisplayFeatures::FLOAT_PIXEL_FORMAT)
            && template.float_pixels
        {
            wgl_extra::TYPE_RGBA_FLOAT_ARB
        } else if template.float_pixels {
            return Err(ErrorKind::NotSupported("float pixels are not supported").into());
        } else {
            wgl_extra::TYPE_RGBA_ARB
        };

        if let Some(num_samples) = template.num_samples {
            if self.inner.features.contains(DisplayFeatures::MULTISAMPLING_PIXEL_FORMATS) {
                attrs.push(wgl_extra::SAMPLE_BUFFERS_ARB as c_int);
                attrs.push(1);
                attrs.push(wgl_extra::SAMPLES_ARB as c_int);
                attrs.push(num_samples as c_int);
            }
        }

        attrs.push(wgl_extra::PIXEL_TYPE_ARB as c_int);
        attrs.push(pixel_type as c_int);

        if let Some(stereo) = template.stereoscopy {
            attrs.push(wgl_extra::STEREO_ARB as c_int);
            attrs.push(stereo as c_int)
        }

        if let Some(hardware_accelerated) = template.hardware_accelerated {
            attrs.push(wgl_extra::ACCELERATION_ARB as c_int);
            if hardware_accelerated {
                attrs.push(wgl_extra::FULL_ACCELERATION_ARB as c_int);
            } else {
                attrs.push(wgl_extra::NO_ACCELERATION_ARB as c_int);
            }
        }

        if template.config_surface_types.contains(ConfigSurfaceTypes::WINDOW) {
            attrs.push(wgl_extra::DRAW_TO_WINDOW_ARB as c_int);
            attrs.push(1);
        }

        if template.config_surface_types.contains(ConfigSurfaceTypes::PIXMAP) {
            attrs.push(wgl_extra::DRAW_TO_WINDOW_ARB as c_int);
            attrs.push(1);
        }

        if template.transparency {
            attrs.push(wgl_extra::TRANSPARENT_ARB as c_int);
            attrs.push(1);
        }

        // Terminate attrs with zero.
        attrs.push(0);

        unsafe {
            let mut num_configs = 0;
            let mut configs = Vec::<c_int>::with_capacity(MAX_QUERY_CONFIGS);

            if wgl_extra.ChoosePixelFormatARB(
                hdc as *const _,
                attrs.as_ptr().cast(),
                std::ptr::null(),
                configs.capacity() as _,
                configs.as_mut_ptr().cast(),
                &mut num_configs,
            ) == 0
            {
                return Err(IoError::last_os_error().into());
            }
            configs.set_len(num_configs as _);

            Ok(Box::new(configs.into_iter().map(move |pixel_format_index| {
                let inner = Arc::new(ConfigInner {
                    display: self.clone(),
                    hdc,
                    pixel_format_index,
                    descriptor: None,
                });
                Config { inner }
            })))
        }
    }
}

/// A wrapper around `PIXELFORMAT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub(crate) inner: Arc<ConfigInner>,
}

impl Config {
    /// Set the pixel format on the native window.
    ///
    /// # Safety
    ///
    /// The `raw_window_handle` should point to a valid value.
    pub unsafe fn apply_on_native_window(&self, raw_window_handle: &RawWindowHandle) -> Result<()> {
        let hdc = match raw_window_handle {
            RawWindowHandle::Win32(window) => unsafe { gdi::GetDC(window.hwnd as _) },
            _ => return Err(ErrorKind::BadNativeWindow.into()),
        };

        let descriptor =
            self.inner.descriptor.as_ref().map(|desc| desc as _).unwrap_or(std::ptr::null());

        unsafe {
            if gl::SetPixelFormat(hdc, self.inner.pixel_format_index, descriptor) == 0 {
                Err(IoError::last_os_error().into())
            } else {
                Ok(())
            }
        }
    }

    pub(crate) fn is_single_buffered(&self) -> bool {
        match self.inner.descriptor.as_ref() {
            Some(descriptor) => (descriptor.dwFlags & gl::PFD_DOUBLEBUFFER) == 0,
            None => unsafe { self.raw_attribute(wgl_extra::DOUBLE_BUFFER_ARB as c_int) == 0 },
        }
    }

    /// # Safety
    ///
    /// The caller must ensure that the attribute could be present.
    unsafe fn raw_attribute(&self, attr: c_int) -> c_int {
        unsafe {
            let wgl_extra = self.inner.display.inner.wgl_extra.unwrap();
            let mut res = 0;
            wgl_extra.GetPixelFormatAttribivARB(
                self.inner.hdc as *const _,
                self.inner.pixel_format_index,
                gl::PFD_MAIN_PLANE as _,
                1,
                &attr,
                &mut res,
            );
            res
        }
    }
}

impl GlConfig for Config {
    fn color_buffer_type(&self) -> Option<ColorBufferType> {
        let (r_size, g_size, b_size) = match self.inner.descriptor.as_ref() {
            Some(descriptor) => (descriptor.cRedBits, descriptor.cGreenBits, descriptor.cBlueBits),
            _ => unsafe {
                let r_size = self.raw_attribute(wgl_extra::RED_BITS_ARB as c_int) as u8;
                let g_size = self.raw_attribute(wgl_extra::GREEN_BITS_ARB as c_int) as u8;
                let b_size = self.raw_attribute(wgl_extra::BLUE_BITS_ARB as c_int) as u8;
                (r_size, g_size, b_size)
            },
        };

        Some(ColorBufferType::Rgb { r_size, g_size, b_size })
    }

    fn float_pixels(&self) -> bool {
        unsafe {
            self.inner.display.inner.features.contains(DisplayFeatures::FLOAT_PIXEL_FORMAT)
                && self.raw_attribute(wgl_extra::PIXEL_TYPE_ARB as c_int)
                    == wgl_extra::TYPE_RGBA_FLOAT_ARB as c_int
        }
    }

    fn alpha_size(&self) -> u8 {
        match self.inner.descriptor.as_ref() {
            Some(descriptor) => descriptor.cAlphaBits,
            _ => unsafe { self.raw_attribute(wgl_extra::ALPHA_BITS_ARB as c_int) as _ },
        }
    }

    fn srgb_capable(&self) -> bool {
        if self.inner.display.inner.client_extensions.contains(SRGB_EXT)
            || self.inner.display.inner.client_extensions.contains("WGL_EXT_colorspace")
        {
            unsafe { self.raw_attribute(wgl_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as c_int) != 0 }
        } else if self.inner.display.inner.client_extensions.contains(SRGB_ARB) {
            unsafe { self.raw_attribute(wgl_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as c_int) != 0 }
        } else {
            false
        }
    }

    fn depth_size(&self) -> u8 {
        match self.inner.descriptor.as_ref() {
            Some(descriptor) => descriptor.cDepthBits,
            _ => unsafe { self.raw_attribute(wgl_extra::DEPTH_BITS_ARB as c_int) as _ },
        }
    }

    fn stencil_size(&self) -> u8 {
        match self.inner.descriptor.as_ref() {
            Some(descriptor) => descriptor.cStencilBits,
            _ => unsafe { self.raw_attribute(wgl_extra::STENCIL_BITS_ARB as c_int) as _ },
        }
    }

    fn num_samples(&self) -> u8 {
        if self.inner.display.inner.features.contains(DisplayFeatures::MULTISAMPLING_PIXEL_FORMATS)
        {
            unsafe { self.raw_attribute(wgl_extra::SAMPLES_ARB as c_int) as _ }
        } else {
            0
        }
    }

    fn config_surface_types(&self) -> ConfigSurfaceTypes {
        let mut flags = ConfigSurfaceTypes::empty();
        match self.inner.descriptor.as_ref() {
            Some(descriptor) => {
                let dw_flags = descriptor.dwFlags;
                if dw_flags & gl::PFD_DRAW_TO_WINDOW != 0 {
                    flags |= ConfigSurfaceTypes::WINDOW;
                }

                if dw_flags & gl::PFD_DRAW_TO_BITMAP != 0 {
                    flags |= ConfigSurfaceTypes::PIXMAP;
                }
            },
            _ => unsafe {
                if self.raw_attribute(wgl_extra::DRAW_TO_WINDOW_ARB as c_int) != 0 {
                    flags |= ConfigSurfaceTypes::WINDOW
                }
                if self.raw_attribute(wgl_extra::DRAW_TO_BITMAP_ARB as c_int) != 0 {
                    flags |= ConfigSurfaceTypes::WINDOW
                }
            },
        }

        flags
    }

    fn hardware_accelerated(&self) -> bool {
        if let Some(descriptor) = self.inner.descriptor.as_ref() {
            descriptor.dwFlags & gl::PFD_GENERIC_ACCELERATED != 0
        } else {
            unsafe {
                self.raw_attribute(wgl_extra::ACCELERATION_ARB as c_int)
                    != wgl_extra::NO_ACCELERATION_ARB as c_int
            }
        }
    }

    fn supports_transparency(&self) -> Option<bool> {
        if self.inner.descriptor.as_ref().is_some() {
            None
        } else {
            unsafe { Some(self.raw_attribute(wgl_extra::TRANSPARENT_ARB as c_int) != 0) }
        }
    }

    fn api(&self) -> Api {
        let mut api = Api::OPENGL;
        if self.inner.display.inner.features.contains(DisplayFeatures::CREATE_ES_CONTEXT) {
            api |= Api::GLES1 | Api::GLES2;
        }

        api
    }
}

impl GetGlDisplay for Config {
    type Target = Display;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl AsRawConfig for Config {
    fn raw_config(&self) -> RawConfig {
        RawConfig::Wgl(self.inner.pixel_format_index)
    }
}

impl Sealed for Config {}

pub(crate) struct ConfigInner {
    pub(crate) display: Display,
    pub(crate) hdc: HDC,
    pub(crate) pixel_format_index: i32,
    pub(crate) descriptor: Option<PIXELFORMATDESCRIPTOR>,
}

impl PartialEq for ConfigInner {
    fn eq(&self, other: &Self) -> bool {
        self.pixel_format_index == other.pixel_format_index
    }
}

impl Eq for ConfigInner {}

impl fmt::Debug for ConfigInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("hdc", &self.hdc)
            .field("pixel_format_index", &self.pixel_format_index)
            .finish()
    }
}

/// This function chooses a pixel format that is likely to be provided by the
/// main video driver of the system.
pub(crate) fn choose_dummy_pixel_format(hdc: HDC) -> Result<(i32, PIXELFORMATDESCRIPTOR)> {
    let descriptor = PIXELFORMATDESCRIPTOR {
        nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
        nVersion: 1,
        dwFlags: gl::PFD_DRAW_TO_WINDOW | gl::PFD_SUPPORT_OPENGL | gl::PFD_DOUBLEBUFFER,
        iPixelType: gl::PFD_TYPE_RGBA,
        cColorBits: 24,
        cRedBits: 0,
        cRedShift: 0,
        cGreenBits: 0,
        cGreenShift: 0,
        cBlueBits: 0,
        cBlueShift: 0,
        cAlphaBits: 8,
        cAlphaShift: 0,
        cAccumBits: 0,
        cAccumRedBits: 0,
        cAccumGreenBits: 0,
        cAccumBlueBits: 0,
        cAccumAlphaBits: 0,
        cDepthBits: 24,
        cStencilBits: 8,
        cAuxBuffers: 0,
        iLayerType: gl::PFD_MAIN_PLANE,
        bReserved: 0,
        dwLayerMask: 0,
        dwVisibleMask: 0,
        dwDamageMask: 0,
    };

    let pixel_format_index = unsafe { gl::ChoosePixelFormat(hdc, &descriptor) };
    if pixel_format_index == 0 {
        return Err(IoError::last_os_error().into());
    }

    unsafe {
        let mut descriptor = MaybeUninit::<PIXELFORMATDESCRIPTOR>::uninit();
        if gl::DescribePixelFormat(
            hdc,
            pixel_format_index as _,
            mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
            descriptor.as_mut_ptr(),
        ) == 0
        {
            return Err(IoError::last_os_error().into());
        };

        let descriptor = descriptor.assume_init();

        if descriptor.iPixelType != gl::PFD_TYPE_RGBA {
            return Err(IoError::last_os_error().into());
        }

        Ok((pixel_format_index, descriptor))
    }
}
