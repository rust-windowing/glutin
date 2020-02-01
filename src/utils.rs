use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::config::{ConfigAttribs, ConfigsFinder};
use winit_types::error::{Error, ErrorType};

#[derive(Copy, Clone)]
pub(crate) struct NoPrint<T>(pub(crate) T);

impl<T> fmt::Debug for NoPrint<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoPrint(...)")
    }
}

impl<T> Deref for NoPrint<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoPrint<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct NoCmp<T>(pub(crate) T);

impl<T> PartialEq for NoCmp<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl<T> Eq for NoCmp<T> {}

impl<T> Deref for NoCmp<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoCmp<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn common_attribs_match(attribs: &ConfigAttribs, cf: &ConfigsFinder) -> Result<(), Error> {
    let change_window = cf.must_support_windows && !attribs.supports_windows;
    let change_pbuffer = cf.must_support_pbuffers && !attribs.supports_pbuffers;
    let change_pixmap = cf.must_support_pixmaps && !attribs.supports_pixmaps;
    if change_window || change_pixmap || change_pbuffer {
        return Err(make_error!(ErrorType::SurfaceTypesNotSupported {
            change_pbuffer,
            change_window,
            change_pixmap,
            change_surfaceless: false
        }));
    }

    if let Some(multisampling) = cf.multisampling {
        let ms = attribs.multisampling.unwrap_or(0);
        if ms != multisampling {
            return Err(make_error!(ErrorType::MultisamplingNotSupported));
        }
    }

    use winit_types::error::BitType;
    if let Some(depth) = cf.depth_bits {
        if depth != attribs.depth_bits {
            return Err(make_error!(ErrorType::NumberOfBitsNotSupported(
                BitType::Depth,
                attribs.depth_bits
            )));
        }
    }

    if let Some(alpha) = cf.alpha_bits {
        if alpha != attribs.alpha_bits {
            return Err(make_error!(ErrorType::NumberOfBitsNotSupported(
                BitType::Alpha,
                attribs.alpha_bits
            )));
        }
    }

    if let Some(stencil) = cf.stencil_bits {
        if stencil != attribs.stencil_bits {
            return Err(make_error!(ErrorType::NumberOfBitsNotSupported(
                BitType::Stencil,
                attribs.stencil_bits
            )));
        }
    }

    if let Some(color) = cf.color_bits {
        if color != attribs.color_bits {
            return Err(make_error!(ErrorType::NumberOfBitsNotSupported(
                BitType::Color,
                attribs.color_bits
            )));
        }
    }

    if let Some(hardware_accelerated) = cf.hardware_accelerated {
        if hardware_accelerated != attribs.hardware_accelerated {
            return Err(make_error!(ErrorType::HardwareAccelerationNotSupported));
        }
    }

    if let Some(stereoscopy) = cf.stereoscopy {
        if stereoscopy != attribs.stereoscopy {
            return Err(make_error!(ErrorType::StereoscopyNotSupported));
        }
    }

    Ok(())
}
