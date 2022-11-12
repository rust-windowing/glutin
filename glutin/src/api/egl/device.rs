//! Everything related to `EGLDevice`.

use std::collections::HashSet;
use std::ffi::{c_void, CStr};
use std::ptr;

use glutin_egl_sys::egl;
use glutin_egl_sys::egl::types::EGLDeviceEXT;

use crate::error::{ErrorKind, Result};

use super::display::{extensions_from_ptr, get_extensions, NO_DISPLAY_EXTENSIONS};
use super::{Egl, EGL};

/// Wrapper for `EGLDevice`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    inner: EGLDeviceEXT,
    extensions: HashSet<&'static str>,
    name: Option<String>,
    vendor: Option<String>,
}

impl Device {
    /// Query the available devices.
    ///
    /// This function returns [`Err`] if the `EGL_EXT_device_query` and
    /// `EGL_EXT_device_enumeration` or `EGL_EXT_device_base` extensions are
    /// not available.
    pub fn query_devices() -> Result<impl Iterator<Item = Device>> {
        let egl = match EGL.as_ref() {
            Some(egl) => egl,
            None => return Err(ErrorKind::NotFound.into()),
        };

        let no_display_extensions =
            NO_DISPLAY_EXTENSIONS.get_or_init(|| get_extensions(egl, egl::NO_DISPLAY));

        // Querying devices requires EGL_EXT_device_enumeration and
        // EGL_EXT_device_query.
        //
        // Or we can check for the EGL_EXT_device_base extension since it contains both
        // extensions.
        if (!no_display_extensions.contains("EGL_EXT_device_enumeration")
            && !no_display_extensions.contains("EGL_EXT_device_query"))
            || !no_display_extensions.contains("EGL_EXT_device_base")
        {
            return Err(ErrorKind::NotSupported("EGL does not support EGL_EXT_device_base").into());
        }

        let mut device_count = 0;

        if unsafe {
            // The specification states:
            // > An EGL_BAD_PARAMETER error is generated if <max_devices> is
            // > less than or equal to zero unless <devices> is NULL, or if
            // > <num_devices> is NULL.
            //
            // The error will never be generated since num_devices is a pointer
            // to the count being queried. Therefore there is no need to check
            // the error.
            egl.QueryDevicesEXT(0, ptr::null_mut(), &mut device_count) == egl::FALSE
        } {
            super::check_error()?;
            // On failure, EGL_FALSE is returned.
            return Err(ErrorKind::NotSupported("Querying device count failed").into());
        }

        let mut devices = Vec::with_capacity(device_count as usize);

        unsafe {
            let mut count = device_count;
            if egl.QueryDevicesEXT(device_count, devices.as_mut_ptr(), &mut count) == egl::FALSE {
                super::check_error()?;
                // On failure, EGL_FALSE is returned.
                return Err(ErrorKind::NotSupported("Querying devices failed").into());
            }

            // SAFETY: EGL has initialized the vector for the number of devices.
            devices.set_len(device_count as usize);
        }

        Ok(devices.into_iter().flat_map(|ptr| Device::from_ptr(egl, ptr)))
    }

    /// Get the device extensions supported by this device.
    ///
    /// These extensions are distinct from the display extensions and should not
    /// be used interchangeably.
    pub fn extensions(&self) -> &HashSet<&str> {
        &self.extensions
    }

    /// Get the name of the device.
    ///
    /// This function will return [`None`] if the `EGL_EXT_device_query_name`
    /// device extension is not available.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the vendor of the device.
    ///
    /// This function will return [`None`] if the `EGL_EXT_device_query_name`
    /// device extension is not available.
    pub fn vendor(&self) -> Option<&str> {
        self.vendor.as_deref()
    }

    /// Get a raw handle to the `EGLDevice`.
    pub fn raw_device(&self) -> *const c_void {
        self.inner
    }
}

// SAFETY: An EGLDevice is immutable and valid for the lifetime of the EGL
// library.
unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device {
    unsafe fn query_string(egl_device: *const c_void, name: egl::types::EGLenum) -> Option<String> {
        let egl = super::EGL.as_ref().unwrap();

        // SAFETY: The caller has ensured the name is valid.
        let ptr = unsafe { egl.QueryDeviceStringEXT(egl_device, name as _) };

        if ptr.is_null() {
            return None;
        }

        unsafe { CStr::from_ptr(ptr) }.to_str().ok().map(String::from)
    }

    pub(crate) fn from_ptr(egl: &Egl, ptr: *const c_void) -> Result<Self> {
        // SAFETY: The EGL specification guarantees the returned string is
        // static and null terminated:
        //
        // > eglQueryDeviceStringEXT returns a pointer to a static,
        // > zero-terminated string describing some aspect of the specified
        // > EGLDeviceEXT. <name> must be EGL_EXTENSIONS.
        let extensions =
            unsafe { extensions_from_ptr(egl.QueryDeviceStringEXT(ptr, egl::EXTENSIONS as _)) };

        let (name, vendor) = if extensions.contains("EGL_EXT_device_query_name") {
            // SAFETY: RENDERER_EXT and VENDOR are valid strings for device string queries
            // if EGL_EXT_device_query_name.
            unsafe {
                (Self::query_string(ptr, egl::RENDERER_EXT), Self::query_string(ptr, egl::VENDOR))
            }
        } else {
            (None, None)
        };

        Ok(Self { inner: ptr, extensions, name, vendor })
    }
}
