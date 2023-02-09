//! Everything related to finding and manipulating the `GLXFBConfig`.

use std::ops::Deref;
use std::os::raw::c_int;
use std::sync::Arc;
use std::{fmt, slice};

use glutin_glx_sys::glx::types::GLXFBConfig;
use glutin_glx_sys::{glx, glx_extra};
use raw_window_handle::RawWindowHandle;

use crate::config::{
    Api, AsRawConfig, ColorBufferType, ConfigSurfaceTypes, ConfigTemplate, GlConfig, RawConfig,
};
use crate::display::{DisplayFeatures, GetGlDisplay};
use crate::error::{ErrorKind, Result};
use crate::platform::x11::{X11GlConfigExt, X11VisualInfo, XLIB};
use crate::private::Sealed;

use super::display::Display;

impl Display {
    pub(crate) unsafe fn find_configs(
        &self,
        template: ConfigTemplate,
    ) -> Result<Box<dyn Iterator<Item = Config> + '_>> {
        let mut config_attributes = Vec::<c_int>::new();

        // Add color buffer type.
        match template.color_buffer_type {
            ColorBufferType::Rgb { r_size, g_size, b_size } => {
                // Type.
                config_attributes.push(glx::X_VISUAL_TYPE as c_int);
                config_attributes.push(glx::TRUE_COLOR as c_int);

                // R.
                config_attributes.push(glx::RED_SIZE as c_int);
                config_attributes.push(r_size as c_int);

                // G.
                config_attributes.push(glx::GREEN_SIZE as c_int);
                config_attributes.push(g_size as c_int);

                // B.
                config_attributes.push(glx::BLUE_SIZE as c_int);
                config_attributes.push(b_size as c_int);
            },
            ColorBufferType::Luminance(luminance) => {
                // Type.
                config_attributes.push(glx::X_VISUAL_TYPE as c_int);
                config_attributes.push(glx::GRAY_SCALE as c_int);

                // L.
                config_attributes.push(glx::RED_SIZE as c_int);
                config_attributes.push(luminance as c_int);
            },
        };

        // Render type.
        config_attributes.push(glx::RENDER_TYPE as c_int);

        if template.float_pixels
            && self.inner.features.contains(DisplayFeatures::FLOAT_PIXEL_FORMAT)
        {
            config_attributes.push(glx_extra::RGBA_FLOAT_BIT_ARB as c_int);
        } else if template.float_pixels {
            return Err(ErrorKind::NotSupported("float pixels are not supported").into());
        } else {
            config_attributes.push(glx::RGBA_BIT as c_int);
        }

        // Add caveat.
        if let Some(hardware_accelerated) = template.hardware_accelerated {
            config_attributes.push(glx::CONFIG_CAVEAT as c_int);
            if hardware_accelerated {
                config_attributes.push(glx::NONE as c_int);
            } else {
                config_attributes.push(glx::SLOW_CONFIG as c_int);
            }
        }

        // Double buffer.
        config_attributes.push(glx::DOUBLEBUFFER as c_int);
        config_attributes.push(!template.single_buffering as c_int);

        // Add alpha.
        config_attributes.push(glx::ALPHA_SIZE as c_int);
        config_attributes.push(template.alpha_size as c_int);

        // Add depth.
        config_attributes.push(glx::DEPTH_SIZE as c_int);
        config_attributes.push(template.depth_size as c_int);

        // Add stencil.
        config_attributes.push(glx::STENCIL_SIZE as c_int);
        config_attributes.push(template.stencil_size as c_int);

        // Add visual if was provided.
        if let Some(RawWindowHandle::Xlib(window)) = template.native_window {
            if window.visual_id > 0 {
                config_attributes.push(glx::VISUAL_ID as c_int);
                config_attributes.push(window.visual_id as c_int);
            }
        }

        // Add surface type.
        config_attributes.push(glx::DRAWABLE_TYPE as c_int);
        let mut surface_type = 0;
        if template.config_surface_types.contains(ConfigSurfaceTypes::WINDOW) {
            surface_type |= glx::WINDOW_BIT;
        }
        if template.config_surface_types.contains(ConfigSurfaceTypes::PBUFFER) {
            surface_type |= glx::PBUFFER_BIT;
        }
        if template.config_surface_types.contains(ConfigSurfaceTypes::PIXMAP) {
            surface_type |= glx::PIXMAP_BIT;
        }
        config_attributes.push(surface_type as c_int);

        // Add maximum height of pbuffer.
        if let Some(pbuffer_width) = template.max_pbuffer_width {
            config_attributes.push(glx::MAX_PBUFFER_WIDTH as c_int);
            config_attributes.push(pbuffer_width as c_int);
        }

        // Add maximum width of pbuffer.
        if let Some(pbuffer_height) = template.max_pbuffer_height {
            config_attributes.push(glx::MAX_PBUFFER_HEIGHT as c_int);
            config_attributes.push(pbuffer_height as c_int);
        }

        // Add stereoscopy, if present.
        if let Some(stereoscopy) = template.stereoscopy {
            config_attributes.push(glx::STEREO as c_int);
            config_attributes.push(stereoscopy as c_int);
        }

        // Add multisampling.
        if let Some(num_samples) = template.num_samples {
            if self.inner.features.contains(DisplayFeatures::MULTISAMPLING_PIXEL_FORMATS) {
                config_attributes.push(glx::SAMPLE_BUFFERS as c_int);
                config_attributes.push(1);
                config_attributes.push(glx::SAMPLES as c_int);
                config_attributes.push(num_samples as c_int);
            }
        }

        // Push X11 `None` to terminate the list.
        config_attributes.push(0);

        unsafe {
            let mut num_configs = 0;
            let raw_configs = self.inner.glx.ChooseFBConfig(
                self.inner.raw.cast(),
                self.inner.screen as _,
                config_attributes.as_ptr() as *const _,
                &mut num_configs,
            );

            if raw_configs.is_null() {
                return Err(ErrorKind::BadConfig.into());
            }

            let configs = slice::from_raw_parts_mut(raw_configs, num_configs as usize).to_vec();

            // Free the memory from the Xlib, since we've just copied it.
            (XLIB.as_ref().unwrap().XFree)(raw_configs as *mut _);

            let iter = configs
                .into_iter()
                .map(move |raw| {
                    let raw = GlxConfig(raw);
                    let inner = Arc::new(ConfigInner { display: self.clone(), raw });
                    Config { inner }
                })
                .filter(move |config| {
                    !template.transparency || config.supports_transparency().unwrap_or(false)
                });

            Ok(Box::new(iter))
        }
    }
}

/// A wrapper around `GLXFBConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub(crate) inner: Arc<ConfigInner>,
}

impl Config {
    /// # Safety
    ///
    /// The caller must ensure that the attribute could be present.
    unsafe fn raw_attribute(&self, attr: c_int) -> c_int {
        unsafe {
            let mut val = 0;
            self.inner.display.inner.glx.GetFBConfigAttrib(
                self.inner.display.inner.raw.cast(),
                *self.inner.raw,
                attr,
                &mut val,
            );
            val as c_int
        }
    }

    pub(crate) fn is_single_buffered(&self) -> bool {
        unsafe { self.raw_attribute(glx::DOUBLEBUFFER as c_int) == 0 }
    }
}

impl GlConfig for Config {
    fn color_buffer_type(&self) -> Option<ColorBufferType> {
        unsafe {
            match self.raw_attribute(glx::X_VISUAL_TYPE as c_int) as _ {
                glx::TRUE_COLOR => {
                    let r_size = self.raw_attribute(glx::RED_SIZE as c_int) as u8;
                    let g_size = self.raw_attribute(glx::GREEN_SIZE as c_int) as u8;
                    let b_size = self.raw_attribute(glx::BLUE_SIZE as c_int) as u8;
                    Some(ColorBufferType::Rgb { r_size, g_size, b_size })
                },
                glx::GRAY_SCALE => {
                    let luma = self.raw_attribute(glx::RED_SIZE as c_int);
                    Some(ColorBufferType::Luminance(luma as u8))
                },
                _ => None,
            }
        }
    }

    fn float_pixels(&self) -> bool {
        if self.inner.display.inner.features.contains(DisplayFeatures::FLOAT_PIXEL_FORMAT) {
            let render_type =
                unsafe { self.raw_attribute(glx::RENDER_TYPE as c_int) as glx::types::GLenum };
            render_type == glx_extra::RGBA_FLOAT_BIT_ARB
        } else {
            false
        }
    }

    fn alpha_size(&self) -> u8 {
        unsafe { self.raw_attribute(glx::ALPHA_SIZE as c_int) as u8 }
    }

    fn hardware_accelerated(&self) -> bool {
        unsafe { self.raw_attribute(glx::CONFIG_CAVEAT as c_int) != glx::SLOW_CONFIG as c_int }
    }

    fn srgb_capable(&self) -> bool {
        if self.inner.display.inner.client_extensions.contains("GLX_ARB_framebuffer_sRGB") {
            unsafe { self.raw_attribute(glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as c_int) != 0 }
        } else if self.inner.display.inner.client_extensions.contains("GLX_EXT_framebuffer_sRGB") {
            unsafe { self.raw_attribute(glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as c_int) != 0 }
        } else {
            false
        }
    }

    fn depth_size(&self) -> u8 {
        unsafe { self.raw_attribute(glx::DEPTH_SIZE as c_int) as u8 }
    }

    fn stencil_size(&self) -> u8 {
        unsafe { self.raw_attribute(glx::STENCIL_SIZE as c_int) as u8 }
    }

    fn num_samples(&self) -> u8 {
        unsafe { self.raw_attribute(glx::SAMPLES as c_int) as u8 }
    }

    fn config_surface_types(&self) -> ConfigSurfaceTypes {
        let mut ty = ConfigSurfaceTypes::empty();

        let raw_ty = unsafe { self.raw_attribute(glx::DRAWABLE_TYPE as c_int) as u32 };
        if raw_ty & glx::WINDOW_BIT as u32 != 0 {
            ty.insert(ConfigSurfaceTypes::WINDOW);
        }
        if raw_ty & glx::PBUFFER_BIT as u32 != 0 {
            ty.insert(ConfigSurfaceTypes::PBUFFER);
        }
        if raw_ty & glx::PIXMAP_BIT as u32 != 0 {
            ty.insert(ConfigSurfaceTypes::PIXMAP);
        }

        ty
    }

    fn supports_transparency(&self) -> Option<bool> {
        self.x11_visual().map(|visual| visual.supports_transparency())
    }

    fn api(&self) -> Api {
        let mut api = Api::OPENGL;
        if self.inner.display.inner.features.contains(DisplayFeatures::CREATE_ES_CONTEXT) {
            api |= Api::GLES1 | Api::GLES2;
        }

        api
    }
}

impl X11GlConfigExt for Config {
    fn x11_visual(&self) -> Option<X11VisualInfo> {
        unsafe {
            let raw_visual = self
                .inner
                .display
                .inner
                .glx
                .GetVisualFromFBConfig(self.inner.display.inner.raw.cast(), *self.inner.raw);
            if raw_visual.is_null() {
                None
            } else {
                Some(X11VisualInfo::from_raw(
                    self.inner.display.inner.raw.cast(),
                    raw_visual as *mut _,
                ))
            }
        }
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
        RawConfig::Glx(*self.inner.raw)
    }
}

impl Sealed for Config {}

pub(crate) struct ConfigInner {
    display: Display,
    pub(crate) raw: GlxConfig,
}

impl PartialEq for ConfigInner {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for ConfigInner {}

impl fmt::Debug for ConfigInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("raw", &self.raw)
            .field("display", &self.display.inner.raw)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GlxConfig(GLXFBConfig);

unsafe impl Send for GlxConfig {}
unsafe impl Sync for GlxConfig {}

impl Deref for GlxConfig {
    type Target = GLXFBConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
