//! Everything related to `NSOpenGLPixelFormat`.

use std::ops::Deref;
use std::sync::Arc;
use std::{fmt, iter};

use cocoa::appkit::{NSOpenGLPixelFormat, NSOpenGLPixelFormatAttribute};
use cocoa::base::{id, nil, BOOL};

use crate::config::{
    Api, AsRawConfig, ColorBufferType, ConfigSurfaceTypes, ConfigTemplate, GlConfig, RawConfig,
};
use crate::display::GetGlDisplay;
use crate::error::{ErrorKind, Result};
use crate::private::Sealed;

use super::display::Display;

impl Display {
    pub(crate) unsafe fn find_configs(
        &self,
        template: ConfigTemplate,
    ) -> Result<Box<dyn Iterator<Item = Config> + '_>> {
        let mut attrs = Vec::<u32>::with_capacity(32);

        // We use minimum to follow behavior of other platforms here.
        attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAMinimumPolicy as u32);

        // Color.
        match template.color_buffer_type {
            ColorBufferType::Rgb { r_size, g_size, b_size } => {
                attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAColorSize as u32);
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
        attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAAlphaSize as u32);
        attrs.push(template.alpha_size as u32);

        // Depth.
        attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFADepthSize as u32);
        attrs.push(template.depth_size as u32);

        // Stencil.
        attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAStencilSize as u32);
        attrs.push(template.stencil_size as u32);

        // Float colors.
        if template.float_pixels {
            attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAColorFloat as u32);
        }

        // Sample buffers.
        if let Some(num_samples) = template.num_samples {
            attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAMultisample as u32);
            attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFASampleBuffers as u32);
            attrs.push(1);
            attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFASamples as u32);
            attrs.push(num_samples as u32);
        }

        // Double buffering.
        if !template.single_buffering {
            attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFADoubleBuffer as u32);
        }

        if template.hardware_accelerated == Some(true) {
            attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAAccelerated as u32);
        }

        // Stereo.
        if template.stereoscopy == Some(true) {
            attrs.push(NSOpenGLPixelFormatAttribute::NSOpenGLPFAStereo as u32);
        }

        // Terminate attrs with zero.
        attrs.push(0);

        let raw = unsafe {
            let raw = NSOpenGLPixelFormat::alloc(nil).initWithAttributes_(&attrs);
            if raw.is_null() {
                return Err(ErrorKind::BadConfig.into());
            }
            raw
        };

        let inner = Arc::new(ConfigInner {
            display: self.clone(),
            raw: NSOpenGLPixelFormatId(raw),
            transparency: template.transparency,
        });
        let config = Config { inner };

        Ok(Box::new(iter::once(config)))
    }
}

/// A wrapper around NSOpenGLPixelFormat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub(crate) inner: Arc<ConfigInner>,
}

impl Config {
    fn raw_attribute(&self, attrib: NSOpenGLPixelFormatAttribute) -> i32 {
        unsafe {
            let mut value = 0;
            NSOpenGLPixelFormat::getValues_forAttribute_forVirtualScreen_(
                *self.inner.raw,
                &mut value,
                attrib,
                // They do differ per monitor and require context. Which is kind of insane, but
                // whatever. Zero is a primary monitor.
                0,
            );
            value as i32
        }
    }

    pub(crate) fn is_single_buffered(&self) -> bool {
        self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFATripleBuffer) == 0
            && self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFADoubleBuffer) == 0
    }
}

impl GlConfig for Config {
    fn color_buffer_type(&self) -> Option<ColorBufferType> {
        // On macos all color formats divide by 3 without reminder, except for the RGB
        // 565. So we can convert it in a hopefully reliable way. Also we should remove
        // alpha.
        let color = self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFAColorSize)
            - self.alpha_size() as i32;
        let r_size = (color / 3) as u8;
        let b_size = (color / 3) as u8;
        let g_size = (color - r_size as i32 - b_size as i32) as u8;
        Some(ColorBufferType::Rgb { r_size, g_size, b_size })
    }

    fn float_pixels(&self) -> bool {
        self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFAColorFloat) != 0
    }

    fn alpha_size(&self) -> u8 {
        self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFAAlphaSize) as u8
    }

    fn srgb_capable(&self) -> bool {
        true
    }

    fn depth_size(&self) -> u8 {
        self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFADepthSize) as u8
    }

    fn stencil_size(&self) -> u8 {
        self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFAStencilSize) as u8
    }

    fn num_samples(&self) -> u8 {
        self.raw_attribute(NSOpenGLPixelFormatAttribute::NSOpenGLPFASamples) as u8
    }

    fn config_surface_types(&self) -> ConfigSurfaceTypes {
        ConfigSurfaceTypes::WINDOW
    }

    fn api(&self) -> Api {
        Api::OPENGL
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
        RawConfig::Cgl(self.inner.raw.cast())
    }
}

impl Sealed for Config {}

pub(crate) struct ConfigInner {
    display: Display,
    pub(crate) transparency: bool,
    pub(crate) raw: NSOpenGLPixelFormatId,
}

impl Drop for ConfigInner {
    fn drop(&mut self) {
        if *self.raw != nil {
            let _: () = unsafe { msg_send![*self.raw, release] };
        }
    }
}

impl PartialEq for ConfigInner {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            let is_equal: BOOL = msg_send![*self.raw, isEqual: *other.raw];
            is_equal != 0
        }
    }
}

impl Eq for ConfigInner {}

impl fmt::Debug for ConfigInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config").field("id", &self.raw).finish()
    }
}

#[derive(Debug)]
pub(crate) struct NSOpenGLPixelFormatId(id);

unsafe impl Send for NSOpenGLPixelFormatId {}
unsafe impl Sync for NSOpenGLPixelFormatId {}

impl Deref for NSOpenGLPixelFormatId {
    type Target = id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
