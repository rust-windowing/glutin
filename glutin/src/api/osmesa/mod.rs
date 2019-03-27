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
    Api, ContextCurrentState, ContextError, CreationError, GlAttributes,
    GlProfile, GlRequest, NotCurrentContext, PixelFormatRequirements,
    PossiblyCurrentContext, Robustness,
};

use takeable_option::Takeable;
use winit::dpi;

use std::ffi::CString;
use std::marker::PhantomData;
use std::os::raw;

#[derive(Debug)]
pub struct OsMesaContextInner {
    context: osmesa_sys::OSMesaContext,
    buffer: Vec<u32>,
    width: u32,
    height: u32,
}

#[derive(Debug)]
pub struct OsMesaContext<T: ContextCurrentState> {
    inner: Takeable<OsMesaContextInner>,
    phantom: PhantomData<T>,
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

impl<T: ContextCurrentState> OsMesaContext<T> {
    pub fn new(
        _pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&OsMesaContext<T>>,
        dims: dpi::PhysicalSize,
    ) -> Result<OsMesaContext<NotCurrentContext>, CreationError> {
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

        let dims: (u32, u32) = dims.into();

        Ok(OsMesaContext {
            inner: Takeable::new(OsMesaContextInner {
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
            }),
            phantom: PhantomData,
        })
    }

    fn state_sub<T2: ContextCurrentState>(
        mut self,
        ret: Option<u8>,
    ) -> Result<OsMesaContext<T2>, (Self, ContextError)> {
        // an error can only happen in case of invalid parameter, which would
        // indicate a bug in glutin
        if ret == Some(0) {
            panic!("OSMesaMakeCurrent failed");
        }

        Ok(OsMesaContext {
            inner: Takeable::new(Takeable::take(&mut self.inner)),
            phantom: PhantomData,
        })
    }

    #[inline]
    pub unsafe fn make_current(
        self,
    ) -> Result<OsMesaContext<PossiblyCurrentContext>, (Self, ContextError)>
    {
        let ret = osmesa_sys::OSMesaMakeCurrent(
            self.inner.context,
            self.inner.buffer.as_ptr() as *mut _,
            0x1401,
            self.inner.width as raw::c_int,
            self.inner.height as raw::c_int,
        );

        self.state_sub(Some(ret))
    }

    #[inline]
    pub unsafe fn make_not_current(
        self,
    ) -> Result<OsMesaContext<NotCurrentContext>, (Self, ContextError)> {
        if osmesa_sys::OSMesaGetCurrentContext() == self.inner.context {
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
        self.state_sub(None)
    }

    #[inline]
    pub unsafe fn treat_as_not_current(
        self,
    ) -> OsMesaContext<NotCurrentContext> {
        self.state_sub(None).unwrap()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe { osmesa_sys::OSMesaGetCurrentContext() == self.inner.context }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        Api::OpenGl
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> *mut raw::c_void {
        self.inner.context as *mut _
    }
}

impl OsMesaContext<PossiblyCurrentContext> {
    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        unsafe {
            let c_str = CString::new(addr.as_bytes().to_vec()).unwrap();
            std::mem::transmute(osmesa_sys::OSMesaGetProcAddress(
                std::mem::transmute(c_str.as_ptr()),
            ))
        }
    }
}

impl<T: ContextCurrentState> Drop for OsMesaContext<T> {
    #[inline]
    fn drop(&mut self) {
        if let Some(inner) = Takeable::try_take(&mut self.inner) {
            unsafe { osmesa_sys::OSMesaDestroyContext(inner.context) }
        }
    }
}

unsafe impl<T: ContextCurrentState> Send for OsMesaContext<T> {}
unsafe impl<T: ContextCurrentState> Sync for OsMesaContext<T> {}
