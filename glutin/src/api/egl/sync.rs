//! EGL Sync Fences.

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::time::Duration;

use glutin_egl_sys::egl::types::{EGLenum, EGLint};

use super::display::DisplayInner;
use super::{egl, ErrorKind, VERSION_1_5};
use crate::error::Result;

/// EGL sync object.
#[derive(Debug, Clone)]
pub struct Sync(pub(super) Arc<Inner>);

impl Sync {
    /// Insert this sync into the currently bound context.
    ///
    /// If the EGL version is not at least 1.5 or the `EGL_KHR_wait_sync`
    /// extension is not available, this returns [`ErrorKind::NotSupported`].
    ///
    /// This will return [`ErrorKind::BadParameter`] if there is no currently
    /// bound context.
    pub fn wait(&self) -> Result<()> {
        if self.0.display.version < VERSION_1_5
            && !self.0.display.display_extensions.contains("EGL_KHR_wait_sync")
        {
            return Err(ErrorKind::NotSupported(
                "Sync::wait is not supported if EGL_KHR_wait_sync isn't available",
            )
            .into());
        }

        if unsafe { self.0.display.egl.WaitSyncKHR(*self.0.display.raw, self.0.inner, 0) }
            == egl::FALSE as EGLint
        {
            return Err(super::check_error().err().unwrap());
        }

        Ok(())
    }

    /// Query if the sync is already
    pub fn is_signalled(&self) -> Result<bool> {
        let status = unsafe { self.get_attrib(egl::SYNC_STATUS) }? as EGLenum;
        Ok(status == egl::SIGNALED)
    }

    /// Block and wait for the sync object to be signalled.
    ///
    /// A timeout of [`None`] will wait forever. If the timeout is [`Some`], the
    /// maximum timeout is [`u64::MAX`] - 1 nanoseconds. Anything larger will be
    /// truncated. If the timeout is reached this function returns [`false`].
    ///
    /// If `flush` is [`true`], the currently bound context is flushed.
    pub fn client_wait(&self, timeout: Option<Duration>, flush: bool) -> Result<bool> {
        let flags = if flush { egl::SYNC_FLUSH_COMMANDS_BIT } else { 0 };
        let timeout = timeout
            .as_ref()
            .map(Duration::as_nanos)
            .map(|nanos| nanos.max(u128::from(u64::MAX)) as u64)
            .unwrap_or(egl::FOREVER);

        let result = unsafe {
            self.0.display.egl.ClientWaitSyncKHR(
                *self.0.display.raw,
                self.0.inner,
                flags as EGLint,
                timeout,
            )
        } as EGLenum;

        match result {
            egl::FALSE => Err(super::check_error().err().unwrap()),
            egl::TIMEOUT_EXPIRED => Ok(false),
            egl::CONDITION_SATISFIED => Ok(true),
            _ => unreachable!(),
        }
    }

    /// Export the fence's underlying sync fd.
    ///
    /// Returns [`ErrorKind::NotSupported`] if the sync is not a native fence.
    ///
    /// # Availability
    ///
    /// This is available on Android and Linux when the
    /// `EGL_ANDROID_native_fence_sync` extension is available.
    #[cfg(unix)]
    pub fn export_sync_fd(&self) -> Result<std::os::unix::prelude::OwnedFd> {
        // Invariants:
        // - EGL_KHR_fence_sync must be supported if a Sync is creatable.
        use std::os::unix::prelude::FromRawFd;

        // Check the type of the fence to see if it can be exported.
        let ty = unsafe { self.get_attrib(egl::SYNC_TYPE) }?;

        // SAFETY: GetSyncAttribKHR was successful.
        if ty as EGLenum != egl::SYNC_NATIVE_FENCE_ANDROID {
            return Err(ErrorKind::NotSupported("The sync is not a native fence").into());
        }

        // SAFETY: The fence type is SYNC_NATIVE_FENCE_ANDROID, making it possible to
        // export the native fence.
        let fd = unsafe {
            self.0.display.egl.DupNativeFenceFDANDROID(*self.0.display.raw, self.0.inner)
        };

        if fd == egl::NO_NATIVE_FENCE_FD_ANDROID {
            return Err(super::check_error().err().unwrap());
        }

        // SAFETY:
        // - The file descriptor from EGL is valid if the return value is not
        //   NO_NATIVE_FENCE_FD_ANDROID.
        // - The EGL implemention duplicates the underlying file descriptor and
        //   transfers ownership to the application.
        Ok(unsafe { std::os::unix::prelude::OwnedFd::from_raw_fd(fd) })
    }

    /// Get a raw handle to the `EGLSync`.
    pub fn raw_device(&self) -> *const c_void {
        self.0.inner
    }

    unsafe fn get_attrib(&self, attrib: EGLenum) -> Result<EGLint> {
        let mut result = MaybeUninit::<EGLint>::uninit();

        if unsafe {
            self.0.display.egl.GetSyncAttribKHR(
                *self.0.display.raw,
                self.0.inner,
                attrib as EGLint,
                result.as_mut_ptr().cast(),
            )
        } == egl::FALSE
        {
            return Err(super::check_error().err().unwrap());
        };

        Ok(unsafe { result.assume_init() })
    }
}

#[derive(Debug)]
pub(super) struct Inner {
    pub(super) inner: egl::types::EGLSyncKHR,
    pub(super) display: Arc<DisplayInner>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // SAFETY: The Sync owns the underlying EGLSyncKHR
        if unsafe { self.display.egl.DestroySyncKHR(*self.display.raw, self.inner) } == egl::FALSE {
            // If this fails we can't do much in Drop. At least drain the error.
            let _ = super::check_error();
        }
    }
}

// SAFETY: The Inner owns the sync and the display is valid.
unsafe impl Send for Inner {}
// SAFETY: EGL allows destroying the sync on any thread.
unsafe impl std::marker::Sync for Inner {}
