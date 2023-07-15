//! Everything related to finding and manipulating the `EGLConfig`.
#![allow(clippy::unnecessary_cast)] // needed for 32bit & 64bit support

use std::ops::Deref;
use std::sync::Arc;
use std::{fmt, mem};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};

use glutin_egl_sys::egl;
use glutin_egl_sys::egl::types::{EGLConfig, EGLint};

use crate::config::{
    Api, AsRawConfig, ColorBufferType, ConfigSurfaceTypes, ConfigTemplate, RawConfig,
};
use crate::display::{DisplayFeatures, GetGlDisplay};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;

#[cfg(x11_platform)]
use crate::platform::x11::{X11GlConfigExt, X11VisualInfo};

use super::display::Display;

impl<D: HasDisplayHandle> Display<D> {
    pub(crate) fn find_configs<W: HasWindowHandle>(
        &self,
        template: ConfigTemplate<W>,
    ) -> Result<Box<dyn Iterator<Item = Config<D>> + '_>> {
        use raw_window_handle::{XcbWindowHandle, XlibWindowHandle};

        let mut config_attributes = Vec::<EGLint>::new();

        // Add color buffer type.
        match template.color_buffer_type {
            ColorBufferType::Rgb { r_size, g_size, b_size } => {
                // Type.
                config_attributes.push(egl::COLOR_BUFFER_TYPE as EGLint);
                config_attributes.push(egl::RGB_BUFFER as EGLint);

                // R.
                config_attributes.push(egl::RED_SIZE as EGLint);
                config_attributes.push(r_size as EGLint);

                // G.
                config_attributes.push(egl::GREEN_SIZE as EGLint);
                config_attributes.push(g_size as EGLint);

                // B.
                config_attributes.push(egl::BLUE_SIZE as EGLint);
                config_attributes.push(b_size as EGLint);
            },
            ColorBufferType::Luminance(luminance) => {
                // Type.
                config_attributes.push(egl::COLOR_BUFFER_TYPE as EGLint);
                config_attributes.push(egl::LUMINANCE_BUFFER as EGLint);

                // L.
                config_attributes.push(egl::LUMINANCE_SIZE as EGLint);
                config_attributes.push(luminance as EGLint);
            },
        };

        if template.float_pixels
            && self.inner.features.contains(DisplayFeatures::FLOAT_PIXEL_FORMAT)
        {
            config_attributes.push(egl::COLOR_COMPONENT_TYPE_EXT as EGLint);
            config_attributes.push(egl::COLOR_COMPONENT_TYPE_FLOAT_EXT as EGLint);
        } else if template.float_pixels {
            return Err(ErrorKind::NotSupported("float pixels not supported").into());
        }

        // Add alpha.
        config_attributes.push(egl::ALPHA_SIZE as EGLint);
        config_attributes.push(template.alpha_size as EGLint);

        // Add depth.
        config_attributes.push(egl::DEPTH_SIZE as EGLint);
        config_attributes.push(template.depth_size as EGLint);

        // Add stencil.
        config_attributes.push(egl::STENCIL_SIZE as EGLint);
        config_attributes.push(template.stencil_size as EGLint);

        // Add surface type.
        config_attributes.push(egl::SURFACE_TYPE as EGLint);
        let mut surface_type = 0;
        if template.config_surface_types.contains(ConfigSurfaceTypes::WINDOW) {
            surface_type |= egl::WINDOW_BIT;
        }
        if template.config_surface_types.contains(ConfigSurfaceTypes::PBUFFER) {
            surface_type |= egl::PBUFFER_BIT;
        }
        if template.config_surface_types.contains(ConfigSurfaceTypes::PIXMAP) {
            surface_type |= egl::PIXMAP_BIT;
        }
        config_attributes.push(surface_type as EGLint);

        // Add caveat.
        if let Some(hardware_accelerated) = template.hardware_accelerated {
            config_attributes.push(egl::CONFIG_CAVEAT as EGLint);
            if hardware_accelerated {
                config_attributes.push(egl::NONE as EGLint);
            } else {
                config_attributes.push(egl::SLOW_CONFIG as EGLint);
            }
        }

        // Add minimum swap interval.
        if let Some(min_swap_interval) = template.min_swap_interval {
            config_attributes.push(egl::MIN_SWAP_INTERVAL as EGLint);
            config_attributes.push(min_swap_interval as EGLint)
        }

        // Add maximum swap interval.
        if let Some(max_swap_interval) = template.max_swap_interval {
            config_attributes.push(egl::MAX_SWAP_INTERVAL as EGLint);
            config_attributes.push(max_swap_interval as EGLint)
        }

        // Add multisampling.
        if let Some(num_samples) = template.num_samples {
            config_attributes.push(egl::SAMPLE_BUFFERS as EGLint);
            config_attributes.push(1);
            config_attributes.push(egl::SAMPLES as EGLint);
            config_attributes.push(num_samples as EGLint);
        }

        config_attributes.push(egl::RENDERABLE_TYPE as EGLint);
        let api = if let Some(requested_api) = template.api {
            let mut api = 0;
            if requested_api.contains(Api::GLES1) {
                api |= egl::OPENGL_ES_BIT;
            }
            if requested_api.contains(Api::GLES2) {
                api |= egl::OPENGL_ES2_BIT;
            }
            if requested_api.contains(Api::GLES3) {
                api |= egl::OPENGL_ES3_BIT;
            }
            if requested_api.contains(Api::OPENGL) {
                api |= egl::OPENGL_BIT;
            }
            api
        } else {
            // NOTE: use ES2 by default to avoid matching pure ES1 configs,
            // for more see https://github.com/rust-windowing/glutin/issues/1586.
            egl::OPENGL_ES2_BIT
        };
        config_attributes.push(api as EGLint);

        // Add maximum height of pbuffer.
        if let Some(pbuffer_width) = template.max_pbuffer_width {
            config_attributes.push(egl::MAX_PBUFFER_WIDTH as EGLint);
            config_attributes.push(pbuffer_width as EGLint);
        }

        // Add maximum width of pbuffer.
        if let Some(pbuffer_height) = template.max_pbuffer_height {
            config_attributes.push(egl::MAX_PBUFFER_HEIGHT as EGLint);
            config_attributes.push(pbuffer_height as EGLint);
        }

        // Push `egl::NONE` to terminate the list.
        config_attributes.push(egl::NONE as EGLint);

        let mut configs_number = self.configs_number() as EGLint;
        let mut found_configs: Vec<EGLConfig> =
            unsafe { vec![mem::zeroed(); configs_number as usize] };

        unsafe {
            let result = self.inner.egl.ChooseConfig(
                *self.inner.raw,
                config_attributes.as_ptr(),
                found_configs.as_mut_ptr(),
                configs_number as EGLint,
                &mut configs_number,
            );

            if result == egl::FALSE {
                return Err(ErrorKind::BadConfig.into());
            }

            found_configs.set_len(configs_number as usize);
        }

        let raw_handle =
            template._native_window.map(|w| w.window_handle().map(|w| w.as_raw())).transpose()?;
        let configs = found_configs
            .into_iter()
            .map(move |raw| {
                let raw = EglConfig(raw);
                let inner = Arc::new(ConfigInner { display: self.clone(), raw });
                Config { inner }
            })
            .filter(move |config| {
                // Filter configs not compatible with the native window.
                //
                // XXX This can't be done by passing visual in the EGL attributes
                // when calling `eglChooseConfig` since the visual is ignored.
                match raw_handle {
                    Some(RawWindowHandle::Xcb(XcbWindowHandle {
                        visual_id: Some(visual_id),
                        ..
                    })) => visual_id.get() == config.native_visual(),
                    Some(RawWindowHandle::Xlib(XlibWindowHandle { visual_id, .. }))
                        if visual_id > 0 =>
                    {
                        visual_id as u32 == config.native_visual()
                    },
                    _ => true,
                }
            })
            .filter(move |config| {
                !template.transparency || config.supports_transparency().unwrap_or(true)
            });

        Ok(Box::new(configs))
    }

    fn configs_number(&self) -> usize {
        unsafe {
            let mut num_configs = 0;
            self.inner.egl.GetConfigs(*self.inner.raw, std::ptr::null_mut(), 0, &mut num_configs);
            num_configs as usize
        }
    }
}

/// A simple wrapper around `EGLConfig` that could be used with `EGLContext`
/// and `EGLSurface`.
#[derive(Debug)]
pub struct Config<D> {
    pub(crate) inner: Arc<ConfigInner<D>>,
}

impl<D> Clone for Config<D> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<D> PartialEq for Config<D> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<D> Eq for Config<D> {}

impl<D: HasDisplayHandle> Config<D> {
    /// The native visual identifier.
    ///
    /// The interpretation of this value is platform dependant. Consult
    /// `platform` extension you're ended up using.
    pub fn native_visual(&self) -> u32 {
        unsafe { self.raw_attribute(egl::NATIVE_VISUAL_ID as EGLint) as u32 }
    }

    /// # Safety
    ///
    /// The caller must ensure that the attribute could be present.
    unsafe fn raw_attribute(&self, attr: EGLint) -> EGLint {
        unsafe {
            let mut val = 0;
            self.inner.display.inner.egl.GetConfigAttrib(
                *self.inner.display.inner.raw,
                *self.inner.raw,
                attr,
                &mut val,
            );
            val as EGLint
        }
    }
}

impl<D: HasDisplayHandle> GlConfig for Config<D> {
    fn color_buffer_type(&self) -> Option<ColorBufferType> {
        unsafe {
            match self.raw_attribute(egl::COLOR_BUFFER_TYPE as EGLint) as _ {
                egl::LUMINANCE_BUFFER => {
                    let luma = self.raw_attribute(egl::LUMINANCE_SIZE as EGLint);
                    Some(ColorBufferType::Luminance(luma as u8))
                },
                egl::RGB_BUFFER => {
                    let r_size = self.raw_attribute(egl::RED_SIZE as EGLint) as u8;
                    let g_size = self.raw_attribute(egl::GREEN_SIZE as EGLint) as u8;
                    let b_size = self.raw_attribute(egl::BLUE_SIZE as EGLint) as u8;
                    Some(ColorBufferType::Rgb { r_size, g_size, b_size })
                },
                _ => None,
            }
        }
    }

    fn float_pixels(&self) -> bool {
        unsafe {
            if self.inner.display.inner.features.contains(DisplayFeatures::FLOAT_PIXEL_FORMAT) {
                matches!(
                    self.raw_attribute(egl::COLOR_COMPONENT_TYPE_EXT as EGLint) as _,
                    egl::COLOR_COMPONENT_TYPE_FLOAT_EXT
                )
            } else {
                false
            }
        }
    }

    fn alpha_size(&self) -> u8 {
        unsafe { self.raw_attribute(egl::ALPHA_SIZE as EGLint) as u8 }
    }

    fn srgb_capable(&self) -> bool {
        self.inner.display.inner.features.contains(DisplayFeatures::SRGB_FRAMEBUFFERS)
    }

    fn depth_size(&self) -> u8 {
        unsafe { self.raw_attribute(egl::DEPTH_SIZE as EGLint) as u8 }
    }

    fn stencil_size(&self) -> u8 {
        unsafe { self.raw_attribute(egl::STENCIL_SIZE as EGLint) as u8 }
    }

    fn num_samples(&self) -> u8 {
        unsafe { self.raw_attribute(egl::SAMPLES as EGLint) as u8 }
    }

    fn config_surface_types(&self) -> ConfigSurfaceTypes {
        let mut ty = ConfigSurfaceTypes::empty();

        let raw_ty = unsafe { self.raw_attribute(egl::SURFACE_TYPE as EGLint) as u32 };
        if raw_ty & egl::WINDOW_BIT as u32 != 0 {
            ty.insert(ConfigSurfaceTypes::WINDOW);
        }
        if raw_ty & egl::PBUFFER_BIT as u32 != 0 {
            ty.insert(ConfigSurfaceTypes::PBUFFER);
        }
        if raw_ty & egl::PIXMAP_BIT as u32 != 0 {
            ty.insert(ConfigSurfaceTypes::PIXMAP);
        }

        ty
    }

    fn hardware_accelerated(&self) -> bool {
        unsafe { self.raw_attribute(egl::CONFIG_CAVEAT as EGLint) != egl::SLOW_CONFIG as EGLint }
    }

    #[cfg(not(any(wayland_platform, x11_platform)))]
    fn supports_transparency(&self) -> Option<bool> {
        None
    }

    #[cfg(any(wayland_platform, x11_platform))]
    fn supports_transparency(&self) -> Option<bool> {
        use raw_window_handle::RawDisplayHandle;
        match self.inner.display.inner.native_display.as_ref()?.display_handle().ok()?.as_raw() {
            #[cfg(x11_platform)]
            RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_) => {
                self.x11_visual().map(|visual| visual.supports_transparency())
            },
            #[cfg(wayland_platform)]
            RawDisplayHandle::Wayland(_) => Some(self.alpha_size() != 0),
            _ => None,
        }
    }

    fn api(&self) -> Api {
        let mut api = Api::empty();
        let raw_api = unsafe { self.raw_attribute(egl::RENDERABLE_TYPE as EGLint) as u32 };
        if raw_api & egl::OPENGL_BIT as u32 != 0 {
            api.insert(Api::OPENGL);
        }
        if raw_api & egl::OPENGL_ES_BIT as u32 != 0 {
            api.insert(Api::GLES1);
        }
        if raw_api & egl::OPENGL_ES2_BIT as u32 != 0 {
            api.insert(Api::GLES2);
        }
        if raw_api & egl::OPENGL_ES3_BIT as u32 != 0 {
            api.insert(Api::GLES3);
        }

        api
    }
}

impl<D: HasDisplayHandle> GetGlDisplay for Config<D> {
    type Target = Display<D>;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl<D: HasDisplayHandle> AsRawConfig for Config<D> {
    fn raw_config(&self) -> RawConfig {
        RawConfig::Egl(*self.inner.raw)
    }
}

#[cfg(x11_platform)]
impl<D: HasDisplayHandle> X11GlConfigExt for Config<D> {
    fn x11_visual(&self) -> Option<X11VisualInfo> {
        match self.inner.display.inner.native_display.as_ref()?.display_handle().ok()?.as_raw() {
            raw_window_handle::RawDisplayHandle::Xlib(display_handle) => unsafe {
                let xid = self.native_visual();
                X11VisualInfo::from_xid(
                    display_handle.display.map_or(std::ptr::null_mut(), |d| d.as_ptr() as *mut _),
                    xid as _,
                )
            },
            _ => None,
        }
    }
}

impl<D: HasDisplayHandle> Sealed for Config<D> {}

pub(crate) struct ConfigInner<D> {
    display: Display<D>,
    pub(crate) raw: EglConfig,
}

impl<D> PartialEq for ConfigInner<D> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<D> Eq for ConfigInner<D> {}

impl<D> fmt::Debug for ConfigInner<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("raw", &self.raw)
            .field("display", &self.display.inner.raw)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EglConfig(EGLConfig);

unsafe impl Send for EglConfig {}
unsafe impl Sync for EglConfig {}

impl Deref for EglConfig {
    type Target = EGLConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
