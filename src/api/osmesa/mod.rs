#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]

pub mod ffi {
    pub use osmesa_sys::OSMesaContext;
}

use crate::{
    Api, ContextError, CreationError, GlAttributes, GlProfile, GlRequest,
    PixelFormat, PixelFormatRequirements, Robustness,
};

use libc;

use std::ffi::CString;
use std::os::raw;

pub struct OsMesaContext {
    context: osmesa_sys::OSMesaContext,
    buffer: Vec<u32>,
    width: u32,
    height: u32,
}

#[derive(Debug)]
struct NoEsOrWebGlSupported;

impl std::fmt::Display for NoEsOrWebGlSupported {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "OsMesa only works with desktop OpenGL; OpenGL ES or WebGL are not supported"
        )
    }
}

impl std::error::Error for NoEsOrWebGlSupported {
    fn description(&self) -> &str {
        "OsMesa only works with desktop OpenGL"
    }
}

#[derive(Debug)]
struct LoadingError(String);

impl LoadingError {
    fn new<D: std::fmt::Debug>(d: D) -> Self {
        LoadingError(format!("{:?}", d))
    }
}

impl std::fmt::Display for LoadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Failed to load OsMesa dynamic library: {}", self.0)
    }
}

impl std::error::Error for LoadingError {
    fn description(&self) -> &str {
        "The library or a symbol of it could not be loaded"
    }
}

impl OsMesaContext {
    pub fn new(
        dims: (u32, u32),
        _pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&OsMesaContext>,
    ) -> Result<OsMesaContext, CreationError> {
        osmesa_sys::OsMesa::try_loading()
            .map_err(LoadingError::new)
            .map_err(|e| CreationError::NoBackendAvailable(Box::new(e)))?;

        if opengl.sharing.is_some() {
            panic!("Context sharing not possible with OsMesa")
        }

        match opengl.robustness {
            Robustness::RobustNoResetNotification
            | Robustness::RobustLoseContextOnReset => {
                return Err(CreationError::RobustnessNotSupported.into());
            }
            _ => (),
        }

        // TODO: use `pf_reqs` for the format

        let mut attribs = Vec::new();

        if let Some(profile) = opengl.profile {
            attribs.push(osmesa_sys::OSMESA_PROFILE);

            match profile {
                GlProfile::Compatibility => {
                    attribs.push(osmesa_sys::OSMESA_COMPAT_PROFILE);
                }
                GlProfile::Core => {
                    attribs.push(osmesa_sys::OSMESA_CORE_PROFILE);
                }
            }
        }

        match opengl.version {
            GlRequest::Latest => {}
            GlRequest::Specific(Api::OpenGl, (major, minor)) => {
                attribs.push(osmesa_sys::OSMESA_CONTEXT_MAJOR_VERSION);
                attribs.push(major as libc::c_int);
                attribs.push(osmesa_sys::OSMESA_CONTEXT_MINOR_VERSION);
                attribs.push(minor as libc::c_int);
            }
            GlRequest::Specific(Api::OpenGlEs, _)
            | GlRequest::Specific(Api::WebGl, _) => {
                return Err(CreationError::NoBackendAvailable(Box::new(
                    NoEsOrWebGlSupported,
                )));
            }
            GlRequest::GlThenGles {
                opengl_version: (major, minor),
                ..
            } => {
                attribs.push(osmesa_sys::OSMESA_CONTEXT_MAJOR_VERSION);
                attribs.push(major as libc::c_int);
                attribs.push(osmesa_sys::OSMESA_CONTEXT_MINOR_VERSION);
                attribs.push(minor as libc::c_int);
            }
        }

        // attribs array must be NULL terminated.
        attribs.push(0);

        Ok(OsMesaContext {
            width: dims.0,
            height: dims.1,
            buffer: std::iter::repeat(unsafe { std::mem::uninitialized() })
                .take((dims.0 * dims.1) as usize)
                .collect(),
            context: unsafe {
                let ctx = osmesa_sys::OSMesaCreateContextAttribs(
                    attribs.as_ptr(),
                    std::ptr::null_mut(),
                );
                if ctx.is_null() {
                    return Err(CreationError::OsError(
                        "OSMesaCreateContextAttribs failed".to_string(),
                    ));
                }
                ctx
            },
        })
    }

    #[inline]
    pub fn get_framebuffer(&self) -> &[u32] {
        &self.buffer
    }

    #[inline]
    pub fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        let ret = osmesa_sys::OSMesaMakeCurrent(
            self.context,
            self.buffer.as_ptr() as *mut _,
            0x1401,
            self.width as libc::c_int,
            self.height as libc::c_int,
        );

        // an error can only happen in case of invalid parameter, which would
        // indicate a bug in glutin
        if ret == 0 {
            panic!("OSMesaMakeCurrent failed");
        }

        Ok(())
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe { osmesa_sys::OSMesaGetCurrentContext() == self.context }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        unsafe {
            let c_str = CString::new(addr.as_bytes().to_vec()).unwrap();
            std::mem::transmute(osmesa_sys::OSMesaGetProcAddress(
                std::mem::transmute(c_str.as_ptr()),
            ))
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        Api::OpenGl
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        unimplemented!();
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> *mut raw::c_void {
        self.context as *mut _
    }
}

impl Drop for OsMesaContext {
    #[inline]
    fn drop(&mut self) {
        unsafe { osmesa_sys::OSMesaDestroyContext(self.context) }
    }
}

unsafe impl Send for OsMesaContext {}
unsafe impl Sync for OsMesaContext {}
