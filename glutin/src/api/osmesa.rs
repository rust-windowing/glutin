#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

use crate::config::{Api, GlRequest, GlVersion};
use crate::context::ContextBuilderWrapper;
use crate::context::{GlProfile, Robustness};
use crate::utils::NoPrint;

use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::ffi::{c_void, CString};
use std::mem::MaybeUninit;
use std::os::raw;

#[derive(Debug)]
pub struct OsMesaContext {
    context: glutin_osmesa_sys::OSMesaContext,
}

#[derive(Debug)]
pub struct OsMesaBuffer {
    buffer: Vec<NoPrint<MaybeUninit<u8>>>,
    width: u32,
    height: u32,
}

impl OsMesaContext {
    pub fn new(
        cb: ContextBuilderWrapper<&OsMesaContext>,
        version: GlRequest,
    ) -> Result<Self, Error> {
        glutin_osmesa_sys::OsMesa::try_loading()
            .map_err(|err| make_oserror!(OsError::OsMesaLoadingError(err)))?;

        if cb.sharing.is_some() {
            panic!("[glutin] Context sharing not possible with OsMesa")
        }

        match cb.robustness {
            Robustness::RobustNoResetNotification | Robustness::RobustLoseContextOnReset => {
                return Err(make_error!(ErrorType::RobustnessNotSupported));
            }
            _ => (),
        }

        let mut attribs = Vec::new();

        if let Some(profile) = cb.profile {
            attribs.push(glutin_osmesa_sys::OSMESA_PROFILE);

            match profile {
                GlProfile::Compatibility => {
                    attribs.push(glutin_osmesa_sys::OSMESA_COMPAT_PROFILE);
                }
                GlProfile::Core => {
                    attribs.push(glutin_osmesa_sys::OSMESA_CORE_PROFILE);
                }
            }
        }

        match version {
            GlRequest::Latest => {}
            GlRequest::Specific(Api::OpenGl, GlVersion(major, minor)) => {
                attribs.push(glutin_osmesa_sys::OSMESA_CONTEXT_MAJOR_VERSION);
                attribs.push(major as raw::c_int);
                attribs.push(glutin_osmesa_sys::OSMESA_CONTEXT_MINOR_VERSION);
                attribs.push(minor as raw::c_int);
            }
            GlRequest::Specific(Api::OpenGlEs, _) | GlRequest::Specific(Api::WebGl, _) => {
                return Err(make_error!(ErrorType::NotSupported(
                    "OSMesa only supports desktop OGL, not WebGL or GLES.".to_string()
                )));
            }
            GlRequest::GlThenGles {
                opengl_version: GlVersion(major, minor),
                ..
            } => {
                attribs.push(glutin_osmesa_sys::OSMESA_CONTEXT_MAJOR_VERSION);
                attribs.push(major as raw::c_int);
                attribs.push(glutin_osmesa_sys::OSMESA_CONTEXT_MINOR_VERSION);
                attribs.push(minor as raw::c_int);
            }
        }

        // attribs array must be NULL terminated.
        attribs.push(0);

        Ok(OsMesaContext {
            context: unsafe {
                let ctx = glutin_osmesa_sys::OSMesaCreateContextAttribs(
                    attribs.as_ptr(),
                    std::ptr::null_mut(),
                );
                if ctx.is_null() {
                    return Err(make_oserror!(OsError::Misc(
                        "OSMesaCreateContextAttribs failed".to_string()
                    )));
                }
                ctx
            },
        })
    }

    #[inline]
    pub unsafe fn make_current_osmesa_buffer(&self, buffer: &OsMesaBuffer) -> Result<(), Error> {
        let ret = glutin_osmesa_sys::OSMesaMakeCurrent(
            self.context,
            buffer.buffer.as_ptr() as *mut _,
            0x1401, // GL_UNSIGNED_BYTE
            buffer.width as raw::c_int,
            buffer.height as raw::c_int,
        );

        // an error can only happen in case of invalid parameter, which would
        // indicate a bug in glutin
        if ret == 0 {
            panic!("[glutin] OSMesaMakeCurrent failed");
        }

        Ok(())
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        if glutin_osmesa_sys::OSMesaGetCurrentContext() == self.context {
            // Supported with the non-gallium drivers, but not the older gallium
            // ones. I (gentz) have filed a patch upstream to mesa to correct
            // this and it eventually got accepted, however, older users
            // probably won't support this.
            //
            // There is no way to tell, ofc, without just calling the function
            // and seeing if it work.
            //
            // https://gitlab.freedesktop.org/mesa/mesa/merge_requests/533
            //
            // Honestly, just go use EGL-Surfaceless!
            let ret = glutin_osmesa_sys::OSMesaMakeCurrent(
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
        unsafe { glutin_osmesa_sys::OSMesaGetCurrentContext() == self.context }
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
    pub fn get_proc_address(&self, addr: &str) -> *const c_void {
        unsafe {
            let c_str = CString::new(addr.as_bytes().to_vec()).unwrap();
            std::mem::transmute(glutin_osmesa_sys::OSMesaGetProcAddress(c_str.as_ptr()))
        }
    }
}

impl Drop for OsMesaContext {
    #[inline]
    fn drop(&mut self) {
        unsafe { glutin_osmesa_sys::OSMesaDestroyContext(self.context) }
    }
}

unsafe impl Send for OsMesaContext {}
unsafe impl Sync for OsMesaContext {}

impl OsMesaBuffer {
    pub fn new(ctx: &OsMesaContext, size: dpi::PhysicalSize) -> Result<Self, Error> {
        let size: (u32, u32) = size.into();
        Ok(OsMesaBuffer {
            width: size.0,
            height: size.1,
            buffer: std::iter::repeat(NoPrint(MaybeUninit::uninit()))
                .take(size.0 as usize * size.1 as usize * 4)
                .collect(),
        })
    }
}
