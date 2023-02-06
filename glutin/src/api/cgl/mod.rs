//! The CGL Api.

#![allow(non_upper_case_globals)]
#![allow(clippy::let_unit_value)] // Temporary

use std::ffi::CStr;
use std::os::raw::c_int;

use cgl::{kCGLNoError, CGLError, CGLErrorString};

use crate::error::{Error, ErrorKind, Result};

mod appkit;
pub mod config;
pub mod context;
pub mod display;
pub mod surface;

const kCGLBadAttribute: c_int = 10000;
const kCGLBadProperty: c_int = 10001;
const kCGLBadPixelFormat: c_int = 10002;
const kCGLBadRendererInfo: c_int = 10003;
const kCGLBadContext: c_int = 10004;
const kCGLBadDrawable: c_int = 10005;
const kCGLBadDisplay: c_int = 10006;
const kCGLBadState: c_int = 10007;
const kCGLBadValue: c_int = 10008;
const kCGLBadMatch: c_int = 10009;
const kCGLBadEnumeration: c_int = 10010;
const kCGLBadOffScreen: c_int = 10011;
const kCGLBadFullScreen: c_int = 10012;
const kCGLBadWindow: c_int = 10013;
const kCGLBadAddress: c_int = 10014;
const kCGLBadCodeModule: c_int = 10015;
const kCGLBadAlloc: c_int = 10016;
const kCGLBadConnection: c_int = 10017;

pub(crate) fn check_error(error: CGLError) -> Result<()> {
    let kind = match error {
        kCGLNoError => return Ok(()),
        kCGLBadAttribute => ErrorKind::BadAttribute,
        kCGLBadProperty => ErrorKind::BadParameter,
        kCGLBadPixelFormat => ErrorKind::BadConfig,
        kCGLBadContext => ErrorKind::BadContext,
        kCGLBadDrawable => ErrorKind::BadSurface,
        kCGLBadDisplay => ErrorKind::BadDisplay,
        kCGLBadState => ErrorKind::BadContextState,
        kCGLBadValue => ErrorKind::BadAttribute,
        kCGLBadEnumeration => ErrorKind::BadAttribute,
        kCGLBadOffScreen => ErrorKind::BadSurface,
        kCGLBadMatch => ErrorKind::BadMatch,
        kCGLBadWindow => ErrorKind::BadNativeWindow,
        kCGLBadAddress => ErrorKind::BadAccess,
        kCGLBadAlloc => ErrorKind::OutOfMemory,
        kCGLBadCodeModule | kCGLBadConnection | kCGLBadRendererInfo | kCGLBadFullScreen => {
            ErrorKind::Misc
        },
        _ => ErrorKind::Misc,
    };

    let description =
        unsafe { CStr::from_ptr(CGLErrorString(error)).to_str().unwrap_or_default().to_string() };
    Err(Error::new(Some(error as _), Some(description), kind))
}
