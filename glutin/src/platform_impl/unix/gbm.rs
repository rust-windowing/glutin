use crate::api::egl::{Context as EglContext, NativeDisplay, SurfaceType as EglSurfaceType};
use crate::{ContextError, CreationError, GlAttributes, PixelFormatRequirements};

use glutin_egl_sys as ffi;

use std::ffi::CString;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::Path;

#[derive(Debug)]
pub struct Context {
    dri_device_fd: RawFd,
    gbm_device: *mut gbm_sys::gbm_device,
    context: ManuallyDrop<EglContext>,
}

impl Deref for Context {
    type Target = EglContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl Context {
    #[inline]
    pub fn new_surfaceless(
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        dri_device_path: &Path,
    ) -> Result<Self, CreationError> {
        // Open DRI device.
        let path = CString::new(dri_device_path.as_os_str().as_bytes()).unwrap();
        let dri_device_fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR) };
        if dri_device_fd == -1 {
            return Err(CreationError::OsError(std::io::Error::last_os_error().to_string()));
        }

        // Create GBM device from DRI device.
        let gbm_device = unsafe { gbm_sys::gbm_create_device(dri_device_fd) };
        if gbm_device.is_null() {
            unsafe { libc::close(dri_device_fd) };
            return Err(CreationError::OsError("GBM device creation failed".to_string()));
        }

        // Create EGL context.
        let gl_attr = gl_attr.clone().map_sharing(|c| &**c);
        let native_display = NativeDisplay::Gbm(Some(gbm_device as *const _));
        let context = match EglContext::new(
            pf_reqs,
            &gl_attr,
            native_display,
            EglSurfaceType::Surfaceless,
            |c, _| Ok(c[0]),
        )
        .and_then(|p| p.finish_surfaceless())
        {
            Ok(context) => context,
            Err(err) => {
                unsafe { gbm_sys::gbm_device_destroy(gbm_device) };
                unsafe { libc::close(dri_device_fd) };
                return Err(err);
            }
        };

        Ok(Self { dri_device_fd, gbm_device, context: ManuallyDrop::new(context) })
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        (**self).make_current()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        (**self).make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        (**self).is_current()
    }

    #[inline]
    pub fn get_api(&self) -> crate::Api {
        (**self).get_api()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EGLContext {
        (**self).raw_handle()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        (**self).get_proc_address(addr)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Drop EGL context.
        unsafe { ManuallyDrop::drop(&mut self.context) };

        // Destroy GBM device.
        unsafe { gbm_sys::gbm_device_destroy(self.gbm_device) };

        // Close DRI device.
        unsafe { libc::close(self.dri_device_fd) };
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}
