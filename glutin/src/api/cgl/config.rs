//! Everything related to `NSOpenGLPixelFormat`.

use std::sync::Arc;
use std::{fmt, iter};

#[allow(deprecated)]
use icrate::AppKit::{
    NSOpenGLPFAAccelerated, NSOpenGLPFAAllowOfflineRenderers, NSOpenGLPFAAlphaSize,
    NSOpenGLPFAColorFloat, NSOpenGLPFAColorSize, NSOpenGLPFADepthSize, NSOpenGLPFADoubleBuffer,
    NSOpenGLPFAMinimumPolicy, NSOpenGLPFAMultisample, NSOpenGLPFAOpenGLProfile,
    NSOpenGLPFASampleBuffers, NSOpenGLPFASamples, NSOpenGLPFAStencilSize, NSOpenGLPFAStereo,
    NSOpenGLPFATripleBuffer, NSOpenGLPixelFormatAttribute, NSOpenGLProfileVersion3_2Core,
    NSOpenGLProfileVersion4_1Core, NSOpenGLProfileVersionLegacy,
};
use objc2::rc::Id;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::config::{
    Api, AsRawConfig, ColorBufferType, ConfigSurfaceTypes, ConfigTemplate, GlConfig, RawConfig,
};
use crate::display::GetGlDisplay;
use crate::error::{ErrorKind, Result};
use crate::private::Sealed;

use super::appkit::NSOpenGLPixelFormat;
use super::display::Display;

impl<D: HasDisplayHandle> Display<D> {
    #[allow(deprecated)]
    pub(crate) fn find_configs<W: HasWindowHandle>(
        &self,
        template: ConfigTemplate<W>,
    ) -> Result<Box<dyn Iterator<Item = Config<D>> + '_>> {
        let mut attrs = Vec::<NSOpenGLPixelFormatAttribute>::with_capacity(32);

        // We use minimum to follow behavior of other platforms here.
        attrs.push(NSOpenGLPFAMinimumPolicy);

        // Allow offline renderers.
        attrs.push(NSOpenGLPFAAllowOfflineRenderers);

        // Color.
        match template.color_buffer_type {
            ColorBufferType::Rgb { r_size, g_size, b_size } => {
                attrs.push(NSOpenGLPFAColorSize);
                // We can't specify particular color, so we provide the sum, and also requires
                // an alpha.
                attrs.push((r_size + g_size + b_size + template.alpha_size) as u32);
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("luminance buffers are not supported with CGL").into()
                )
            },
        }

        // Alpha.
        attrs.push(NSOpenGLPFAAlphaSize);
        attrs.push(template.alpha_size as u32);

        // Depth.
        attrs.push(NSOpenGLPFADepthSize);
        attrs.push(template.depth_size as u32);

        // Stencil.
        attrs.push(NSOpenGLPFAStencilSize);
        attrs.push(template.stencil_size as u32);

        // Float colors.
        if template.float_pixels {
            attrs.push(NSOpenGLPFAColorFloat);
        }

        // Sample buffers.
        if let Some(num_samples) = template.num_samples {
            attrs.push(NSOpenGLPFAMultisample);
            attrs.push(NSOpenGLPFASampleBuffers);
            attrs.push(1);
            attrs.push(NSOpenGLPFASamples);
            attrs.push(num_samples as u32);
        }

        // Double buffering.
        if !template.single_buffering {
            attrs.push(NSOpenGLPFADoubleBuffer);
        }

        if template.hardware_accelerated == Some(true) {
            attrs.push(NSOpenGLPFAAccelerated);
        }

        // Stereo.
        if template.stereoscopy == Some(true) {
            attrs.push(NSOpenGLPFAStereo);
        }

        attrs.push(NSOpenGLPFAOpenGLProfile);

        // Stash profile pos for latter insert.
        let profile_attr_pos = attrs.len();
        // Add place holder for the GL profile.
        attrs.push(NSOpenGLProfileVersion4_1Core);

        // Terminate attrs with zero.
        attrs.push(0);

        // Automatically pick the latest profile.
        let raw = [
            NSOpenGLProfileVersion4_1Core,
            NSOpenGLProfileVersion3_2Core,
            NSOpenGLProfileVersionLegacy,
        ]
        .into_iter()
        .find_map(|profile| {
            attrs[profile_attr_pos] = profile;
            // initWithAttributes returns None if the attributes were invalid
            unsafe { NSOpenGLPixelFormat::newWithAttributes(&attrs) }
        })
        .ok_or(ErrorKind::BadConfig)?;

        let inner = Arc::new(ConfigInner {
            display: self.clone(),
            raw,
            transparency: template.transparency,
        });
        let config = Config { inner };

        Ok(Box::new(iter::once(config)))
    }
}

/// A wrapper around NSOpenGLPixelFormat.
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
    fn raw_attribute(&self, attrib: NSOpenGLPixelFormatAttribute) -> i32 {
        unsafe {
            let mut value = 0;
            self.inner.raw.getValues_forAttribute_forVirtualScreen(
                &mut value, attrib,
                // They do differ per monitor and require context. Which is kind of insane, but
                // whatever. Zero is a primary monitor.
                0,
            );
            value
        }
    }

    #[allow(deprecated)]
    pub(crate) fn is_single_buffered(&self) -> bool {
        self.raw_attribute(NSOpenGLPFATripleBuffer) == 0
            && self.raw_attribute(NSOpenGLPFADoubleBuffer) == 0
    }
}

#[allow(deprecated)]
impl<D: HasDisplayHandle> GlConfig for Config<D> {
    fn color_buffer_type(&self) -> Option<ColorBufferType> {
        // On macos all color formats divide by 3 without reminder, except for the RGB
        // 565. So we can convert it in a hopefully reliable way. Also we should remove
        // alpha.
        let color = self.raw_attribute(NSOpenGLPFAColorSize) - self.alpha_size() as i32;
        let r_size = (color / 3) as u8;
        let b_size = (color / 3) as u8;
        let g_size = (color - r_size as i32 - b_size as i32) as u8;
        Some(ColorBufferType::Rgb { r_size, g_size, b_size })
    }

    fn float_pixels(&self) -> bool {
        self.raw_attribute(NSOpenGLPFAColorFloat) != 0
    }

    fn alpha_size(&self) -> u8 {
        self.raw_attribute(NSOpenGLPFAAlphaSize) as u8
    }

    fn srgb_capable(&self) -> bool {
        true
    }

    fn hardware_accelerated(&self) -> bool {
        self.raw_attribute(NSOpenGLPFAAccelerated) != 0
    }

    fn depth_size(&self) -> u8 {
        self.raw_attribute(NSOpenGLPFADepthSize) as u8
    }

    fn stencil_size(&self) -> u8 {
        self.raw_attribute(NSOpenGLPFAStencilSize) as u8
    }

    fn num_samples(&self) -> u8 {
        self.raw_attribute(NSOpenGLPFASamples) as u8
    }

    fn config_surface_types(&self) -> ConfigSurfaceTypes {
        ConfigSurfaceTypes::WINDOW
    }

    fn supports_transparency(&self) -> Option<bool> {
        Some(self.inner.transparency)
    }

    fn api(&self) -> Api {
        Api::OPENGL
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
        RawConfig::Cgl(Id::as_ptr(&self.inner.raw).cast())
    }
}

impl<D: HasDisplayHandle> Sealed for Config<D> {}

pub(crate) struct ConfigInner<D> {
    display: Display<D>,
    pub(crate) transparency: bool,
    pub(crate) raw: Id<NSOpenGLPixelFormat>,
}

impl<D> PartialEq for ConfigInner<D> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<D> Eq for ConfigInner<D> {}

impl<D> fmt::Debug for ConfigInner<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config").field("id", &self.raw).finish()
    }
}
