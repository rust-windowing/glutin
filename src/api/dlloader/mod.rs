use std::ffi::CString;
use libloading::Library;

use super::egl::ffi::egl::Egl;
use super::glx::ffi::glx::Glx;

// You have to make sure the symbols don't outlive the library,
// easiest way is to just make the whole thing lazy_static.
pub struct GlxOrEgl {
    pub glx: Option<Glx>,
    pub egl: Option<Egl>,
    _libglx: Option<Library>,
    _libegl: Option<Library>,
}

impl GlxOrEgl {
    pub fn new(do_glx: bool) -> GlxOrEgl {
        let glx = if do_glx {
            let libglx = Library::new("libGL.so.1")
                .or_else(|_| Library::new("libGL.so"))
                .ok();
            (
                libglx.as_ref().map(|libglx| {
                    Glx::load_with(|sym| unsafe {
                        libglx
                            .get(
                                CString::new(sym.as_bytes())
                                    .unwrap()
                                    .as_bytes_with_nul(),
                            )
                            .map(|sym| *sym)
                            .unwrap_or(std::ptr::null_mut())
                    })
                }),
                libglx,
            )
        } else {
            (None, None)
        };
        let egl = {
            let libegl = Library::new("libEGL.so.1")
                .or_else(|_| Library::new("libEGL.so"))
                .ok();

            (
                libegl.as_ref().map(|libegl| {
                    Egl::load_with(|sym| unsafe {
                        libegl
                            .get(
                                CString::new(sym.as_bytes())
                                    .unwrap()
                                    .as_bytes_with_nul(),
                            )
                            .map(|sym| *sym)
                            .unwrap_or(std::ptr::null_mut())
                    })
                }),
                libegl,
            )
        };
        GlxOrEgl {
            glx: glx.0,
            egl: egl.0,
            _libglx: glx.1,
            _libegl: egl.1,
        }
    }
}
