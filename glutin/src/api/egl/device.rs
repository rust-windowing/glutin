//! Everything related to `EGLDevice`.

use std::collections::HashSet;
use std::ffi::CStr;
use std::path::Path;
use std::ptr;

use glutin_egl_sys::egl;
use glutin_egl_sys::egl::types::EGLDeviceEXT;

use crate::error::{ErrorKind, Result};

use super::display::{extensions_from_ptr, get_extensions, CLIENT_EXTENSIONS};
use super::{Egl, EGL};

/// Wrapper for `EGLDevice`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    inner: EGLDeviceEXT,
    extensions: HashSet<&'static str>,
    name: Option<&'static str>,
    vendor: Option<&'static str>,
}

// SAFETY: An EGLDevice is immutable and valid for the lifetime of the EGL
// library.
unsafe impl Send for Device {}
unsafe impl Sync for Device {}

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

        let client_extensions =
            CLIENT_EXTENSIONS.get_or_init(|| get_extensions(egl, egl::NO_DISPLAY));

        // Querying devices requires EGL_EXT_device_enumeration or EGL_EXT_device_base.
        if !client_extensions.contains("EGL_EXT_device_base") {
            if !client_extensions.contains("EGL_EXT_device_enumeration") {
                return Err(ErrorKind::NotSupported(
                    "Enumerating devices is not supported by the EGL instance",
                )
                .into());
            }
            // EGL_EXT_device_enumeration depends on EGL_EXT_device_query,
            // so also check that just in case.
            if !client_extensions.contains("EGL_EXT_device_query") {
                return Err(ErrorKind::NotSupported(
                    "EGL_EXT_device_enumeration without EGL_EXT_device_query, buggy driver?",
                )
                .into());
            }
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
    pub fn extensions(&self) -> &HashSet<&'static str> {
        &self.extensions
    }

    /// Get the name of the device.
    ///
    /// This function will return [`None`] if the `EGL_EXT_device_query_name`
    /// device extension is not available.
    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Get the vendor of the device.
    ///
    /// This function will return [`None`] if the `EGL_EXT_device_query_name`
    /// device extension is not available.
    pub fn vendor(&self) -> Option<&'static str> {
        self.vendor
    }

    /// Get a raw handle to the `EGLDevice`.
    pub fn raw_device(&self) -> EGLDeviceEXT {
        self.inner
    }

    /// Get the DRM primary or render device node path for this
    /// [`EGLDeviceEXT`].
    ///
    /// Requires the [`EGL_EXT_device_drm`] extension.
    ///
    /// If the [`EGL_EXT_device_drm_render_node`] extension is supported, this
    /// is guaranteed to return the **primary** device node path, or [`None`].
    /// Consult [`Self::drm_render_device_node_path()`] to retrieve the
    /// **render** device node path.
    ///
    /// [`EGL_EXT_device_drm`]: https://registry.khronos.org/EGL/extensions/EXT/EGL_EXT_device_drm.txt
    /// [`EGL_EXT_device_drm_render_node`]: https://registry.khronos.org/EGL/extensions/EXT/EGL_EXT_device_drm_render_node.txt
    pub fn drm_device_node_path(&self) -> Option<&'static Path> {
        if !self.extensions.contains("EGL_EXT_device_drm") {
            return None;
        }

        // SAFETY: We pass a valid EGLDevice pointer, and validated that the enum name
        // is valid because the extension is present.
        unsafe { Self::query_string(self.raw_device(), egl::DRM_DEVICE_FILE_EXT) }.map(Path::new)
    }

    /// Get the DRM render device node path for this [`EGLDeviceEXT`].
    ///
    /// Requires the [`EGL_EXT_device_drm_render_node`] extension.
    ///
    /// If the [`EGL_EXT_device_drm`] extension is supported in addition to
    /// [`EGL_EXT_device_drm_render_node`],
    /// consult [`Self::drm_device_node_path()`] to retrieve the **primary**
    /// device node path.
    ///
    /// [`EGL_EXT_device_drm`]: https://registry.khronos.org/EGL/extensions/EXT/EGL_EXT_device_drm.txt
    /// [`EGL_EXT_device_drm_render_node`]: https://registry.khronos.org/EGL/extensions/EXT/EGL_EXT_device_drm_render_node.txt
    pub fn drm_render_device_node_path(&self) -> Option<&'static Path> {
        if !self.extensions.contains("EGL_EXT_device_drm_render_node") {
            return None;
        }

        const EGL_DRM_RENDER_NODE_PATH_EXT: egl::types::EGLenum = 0x3377;
        // SAFETY: We pass a valid EGLDevice pointer, and validated that the enum name
        // is valid because the extension is present.
        unsafe { Self::query_string(self.raw_device(), EGL_DRM_RENDER_NODE_PATH_EXT) }
            .map(Path::new)
    }

    /// # Safety
    /// The caller must pass  a valid `egl_device` pointer and must ensure that
    /// `name` is valid for this device, i.e. by guaranteeing that the
    /// extension that introduces it is present.
    ///
    /// The returned string is `'static` for the lifetime of the globally loaded
    /// EGL library in [`EGL`].
    unsafe fn query_string(
        egl_device: EGLDeviceEXT,
        name: egl::types::EGLenum,
    ) -> Option<&'static str> {
        let egl = super::EGL.as_ref().unwrap();

        // SAFETY: The caller has ensured the name is valid.
        let ptr = unsafe { egl.QueryDeviceStringEXT(egl_device, name as _) };

        if ptr.is_null() {
            return None;
        }

        unsafe { CStr::from_ptr(ptr) }.to_str().ok()
    }

    pub(crate) fn from_ptr(egl: &Egl, ptr: EGLDeviceEXT) -> Result<Self> {
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
