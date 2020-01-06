#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod ffi {
    pub use osmesa_sys::OSMesaContext;
}

use crate::{
    Api, ContextError, CreationError, GlAttributes, GlProfile, GlRequest,
    PixelFormatRequirements, Robustness,
};

use winit::dpi;

use std::ffi::CString;
use std::os::raw;

#[derive(Debug)]
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
        _pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&OsMesaContext>,
        size: dpi::PhysicalSize<u32>,
    ) -> Result<Self, CreationError> {
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
                attribs.push(major as raw::c_int);
                attribs.push(osmesa_sys::OSMESA_CONTEXT_MINOR_VERSION);
                attribs.push(minor as raw::c_int);
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
                attribs.push(major as raw::c_int);
                attribs.push(osmesa_sys::OSMESA_CONTEXT_MINOR_VERSION);
                attribs.push(minor as raw::c_int);
            }
        }

        // attribs array must be NULL terminated.
        attribs.push(0);

        let size: (u32, u32) = size.into();

        Ok(OsMesaContext {
            width: size.0,
            height: size.1,
            buffer: std::iter::repeat(unsafe { std::mem::zeroed() })
                .take((size.0 * size.1) as usize)
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
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        let ret = osmesa_sys::OSMesaMakeCurrent(
            self.context,
            self.buffer.as_ptr() as *mut _,
            0x1401,
            self.width as raw::c_int,
            self.height as raw::c_int,
        );

        // an error can only happen in case of invalid parameter, which would
        // indicate a bug in glutin
        if ret == 0 {
            panic!("OSMesaMakeCurrent failed");
        }

        Ok(())
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        if osmesa_sys::OSMesaGetCurrentContext() == self.context {
            // Supported with the non-gallium drivers, but not the gallium ones
            // I (gentz) have filed a patch upstream to mesa to correct this,
            // however, older users (or anyone not running mesa-git, tbh)
            // probably won't support this.
            //
            // There is no way to tell, ofc, without just calling the function
            // and seeing if it work.
            //
            // https://gitlab.freedesktop.org/mesa/mesa/merge_requests/533
            let ret = osmesa_sys::OSMesaMakeCurrent(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                0,
                0,
            );

            if ret == 0 {
                unimplemented!(
                    "OSMesaMakeCurrent failed to make the context not current. This most likely means that you're using an older gallium-based mesa driver."
                )
            }
        }

        Ok(())
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe { osmesa_sys::OSMesaGetCurrentContext() == self.context }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        Api::OpenGl
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> *mut raw::c_void {
        self.context as *mut _
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        unsafe {
            let c_str = CString::new(addr.as_bytes().to_vec()).unwrap();
            core::mem::transmute(osmesa_sys::OSMesaGetProcAddress(
                c_str.as_ptr() as *mut _,
            ))
        }
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
