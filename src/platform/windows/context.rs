#![cfg(target_os = "windows")]

use std::ptr;

use winit;

use ContextError;
use CreationError;
use GlAttributes;
use GlRequest;
use Api;
use PixelFormat;
use PixelFormatRequirements;

use winapi;

use api::wgl::Context as WglContext;
use api::egl::Context as EglContext;
use api::egl::ffi::egl::Egl;
use api::egl;

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

pub enum Context {
    Egl(EglContext),
    Wgl(WglContext),
}


impl Context {

    /// See the docs in the crate root file.
    pub fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
        egl: Option<&Egl>,
    ) -> Result<(winit::Window, Self), CreationError>
    {
        let window = try!(window_builder.build(events_loop));
        let gl_attr = gl_attr.clone().map_sharing(|ctxt| {
            match *ctxt {
                Context::Wgl(ref c) => c.get_hglrc(),
                // FIXME
                Context::Egl(_) => unimplemented!(),
            }
        });
        let context_result = unsafe {
            let w = window.platform_window() as winapi::HWND;
            match gl_attr.version {
                GlRequest::Specific(Api::OpenGlEs, (_major, _minor)) => {
                    if let Some(egl) = egl {
                        if let Ok(c) =
                               EglContext::new(egl.clone(),
                                               &pf_reqs,
                                               &gl_attr.clone().map_sharing(|_| unimplemented!()),
                                               egl::NativeDisplay::Other(Some(ptr::null())))
                            .and_then(|p| p.finish(w)) {
                            Ok(Context::Egl(c))
                        } else {
                            WglContext::new(&pf_reqs, &gl_attr, w).map(Context::Wgl)
                        }

                    } else {
                        // falling back to WGL, which is always available
                        WglContext::new(&pf_reqs, &gl_attr, w).map(Context::Wgl)
                    }
                }
                _ => WglContext::new(&pf_reqs, &gl_attr, w).map(Context::Wgl),
            }
        };
        context_result.map(|context| (window, context))
    }

    #[inline]
    pub fn resize(&self, _width: u32, _height: u32) {
        // Method is for API consistency.
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::Wgl(ref c) => c.make_current(),
            Context::Egl(ref c) => c.make_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::Wgl(ref c) => c.is_current(),
            Context::Egl(ref c) => c.is_current(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::Wgl(ref c) => c.get_proc_address(addr),
            Context::Egl(ref c) => c.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::Wgl(ref c) => c.swap_buffers(),
            Context::Egl(ref c) => c.swap_buffers(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match *self {
            Context::Wgl(ref c) => c.get_api(),
            Context::Egl(ref c) => c.get_api(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::Wgl(ref c) => c.get_pixel_format(),
            Context::Egl(ref c) => c.get_pixel_format(),
        }
    }
}
