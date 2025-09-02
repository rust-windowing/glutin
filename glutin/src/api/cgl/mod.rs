//! The CGL Api.

#![allow(non_upper_case_globals)]
#![allow(clippy::let_unit_value)] // Temporary

use std::ffi::CStr;

#[allow(deprecated)]
use objc2_open_gl::{CGLError, CGLErrorString};

use crate::error::{Error, ErrorKind, Result};

pub mod config;
pub mod context;
pub mod display;
pub mod surface;

#[allow(deprecated)]
pub(crate) fn check_error(error: CGLError) -> Result<()> {
    let kind = match error {
        CGLError::NoError => return Ok(()),
        CGLError::BadAttribute => ErrorKind::BadAttribute,
        CGLError::BadProperty => ErrorKind::BadParameter,
        CGLError::BadPixelFormat => ErrorKind::BadConfig,
        CGLError::BadContext => ErrorKind::BadContext,
        CGLError::BadDrawable => ErrorKind::BadSurface,
        CGLError::BadDisplay => ErrorKind::BadDisplay,
        CGLError::BadState => ErrorKind::BadContextState,
        CGLError::BadValue => ErrorKind::BadAttribute,
        CGLError::BadEnumeration => ErrorKind::BadAttribute,
        CGLError::BadOffScreen => ErrorKind::BadSurface,
        CGLError::BadMatch => ErrorKind::BadMatch,
        CGLError::BadWindow => ErrorKind::BadNativeWindow,
        CGLError::BadAddress => ErrorKind::BadAccess,
        CGLError::BadAlloc => ErrorKind::OutOfMemory,
        CGLError::BadCodeModule
        | CGLError::BadConnection
        | CGLError::BadRendererInfo
        | CGLError::BadFullScreen => ErrorKind::Misc,
        _ => ErrorKind::Misc,
    };

    let description = unsafe {
        CStr::from_ptr(CGLErrorString(error).as_ptr()).to_str().unwrap_or_default().to_string()
    };
    Err(Error::new(Some(error.0 as _), Some(description), kind))
}
