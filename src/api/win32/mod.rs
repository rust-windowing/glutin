#![cfg(target_os = "windows")]

use std::ptr;

use winit;

use ContextError;
use CreationError;
use GlAttributes;
use GlContext;
use GlRequest;
use Api;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;

use winapi;

use api::wgl::Context as WglContext;
use api::egl::Context as EglContext;
use api::egl::ffi::egl::Egl;
use api::egl;

pub use winit::{
    MonitorId,
    get_available_monitors,
    get_primary_monitor,
    WindowProxy,
};

/// The Win32 implementation of the main `Window` object.
pub struct Window {
    context: Context,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

enum Context {
    Egl(EglContext),
    Wgl(WglContext),
}

impl Window {
    /// See the docs in the crate root file.
    pub fn new(
        _: &WindowAttributes,
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        egl: Option<&Egl>,
        winit_window: &winit::Window,
    ) -> Result<Window, CreationError> {
        let opengl = opengl.clone().map_sharing(|sharing| {
            match sharing.context {
                Context::Wgl(ref c) => c.get_hglrc(),
                Context::Egl(_) => unimplemented!(),        // FIXME:
            }
        });
        let context = unsafe {
            let w = winit_window.platform_window() as winapi::HWND;
            match opengl.version {
                GlRequest::Specific(Api::OpenGlEs, (_major, _minor)) => {
                    if let Some(egl) = egl {
                        if let Ok(c) = EglContext::new(egl.clone(), &pf_reqs, &opengl.clone().map_sharing(|_| unimplemented!()),
                                                       egl::NativeDisplay::Other(Some(ptr::null())))
                                                                     .and_then(|p| p.finish(w))
                        {
                            Context::Egl(c)
                        } else {
                            try!(WglContext::new(&pf_reqs, &opengl, w)
                                                .map(Context::Wgl))
                        }

                    } else {
                        // falling back to WGL, which is always available
                        try!(WglContext::new(&pf_reqs, &opengl, w)
                                            .map(Context::Wgl))
                    }
                },
                _ => {
                    try!(WglContext::new(&pf_reqs, &opengl, w).map(Context::Wgl))
                }
            }
        };
        Ok(Window {
            context: context,
        })
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self.context {
            Context::Wgl(ref c) => c.make_current(),
            Context::Egl(ref c) => c.make_current(),
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self.context {
            Context::Wgl(ref c) => c.is_current(),
            Context::Egl(ref c) => c.is_current(),
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self.context {
            Context::Wgl(ref c) => c.get_proc_address(addr),
            Context::Egl(ref c) => c.get_proc_address(addr),
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self.context {
            Context::Wgl(ref c) => c.swap_buffers(),
            Context::Egl(ref c) => c.swap_buffers(),
        }
    }

    #[inline]
    fn get_api(&self) -> Api {
        match self.context {
            Context::Wgl(ref c) => c.get_api(),
            Context::Egl(ref c) => c.get_api(),
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self.context {
            Context::Wgl(ref c) => c.get_pixel_format(),
            Context::Egl(ref c) => c.get_pixel_format(),
        }
    }
}