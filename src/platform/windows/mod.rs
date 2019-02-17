#![cfg(target_os = "windows")]

use std::os::raw;
use std::ptr;

use winapi::shared::windef::{HGLRC, HWND};
use winit;

use Api;
use ContextError;
use CreationError;
use GlAttributes;
use GlRequest;
use PixelFormat;
use PixelFormatRequirements;

use api::egl;
use api::egl::Context as EglContext;
use api::egl::EGL;
use api::wgl::Context as WglContext;
use os::windows::WindowExt;

/// Context handles available on Windows.
#[derive(Clone, Debug)]
pub enum RawHandle {
    Egl(egl::ffi::EGLContext),
    Wgl(HGLRC),
}

pub enum Context {
    /// A regular window
    Egl(EglContext),
    Wgl(WglContext),
    /// A regular window, but invisible.
    HiddenWindowEgl(winit::Window, EglContext),
    HiddenWindowWgl(winit::Window, WglContext),
    /// An EGL pbuffer.
    EglPbuffer(EglContext),
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    /// See the docs in the crate root file.
    #[inline]
    pub fn new(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<(winit::Window, Self), CreationError> {
        let window = wb.build(el)?;
        let ctx = Self::new_separated(&window, el, pf_reqs, gl_attr)?;

        Ok((window, ctx))
    }

    #[inline]
    pub fn new_separated(
        window: &winit::Window,
        _el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<Self, CreationError> {
        let w = window.get_hwnd() as HWND;
        match gl_attr.version {
            GlRequest::Specific(Api::OpenGlEs, (_major, _minor)) => {
                match (gl_attr.sharing, &*EGL) {
                    // We must use WGL.
                    (Some(&Context::HiddenWindowWgl(_, _)), _)
                    | (Some(&Context::Wgl(_)), _)
                    | (None, None) => {
                        let gl_attr_wgl =
                            gl_attr.clone().map_sharing(|ctx| match *ctx {
                                Context::HiddenWindowWgl(_, ref c)
                                | Context::Wgl(ref c) => c.get_hglrc(),
                                _ => unreachable!(),
                            });
                        unsafe {
                            WglContext::new(&pf_reqs, &gl_attr_wgl, w)
                                .map(Context::Wgl)
                        }
                    }
                    // We must use EGL.
                    (Some(_), Some(_)) => {
                        let gl_attr_egl =
                            gl_attr.clone().map_sharing(|ctx| match *ctx {
                                Context::Egl(ref c)
                                | Context::EglPbuffer(ref c)
                                | Context::HiddenWindowEgl(_, ref c) => c,
                                _ => unreachable!(),
                            });

                        EglContext::new(
                            &pf_reqs,
                            &gl_attr_egl,
                            egl::NativeDisplay::Other(Some(ptr::null())),
                        )
                        .and_then(|p| p.finish(w))
                        .map(|c| Context::Egl(c))
                    }
                    // Try EGL, fallback to WGL.
                    (None, Some(_)) => {
                        let gl_attr_egl =
                            gl_attr.clone().map_sharing(|_| unreachable!());
                        let gl_attr_wgl =
                            gl_attr.clone().map_sharing(|_| unreachable!());

                        if let Ok(c) = EglContext::new(
                            &pf_reqs,
                            &gl_attr_egl,
                            egl::NativeDisplay::Other(Some(ptr::null())),
                        )
                        .and_then(|p| p.finish(w))
                        {
                            Ok(Context::Egl(c))
                        } else {
                            unsafe {
                                WglContext::new(&pf_reqs, &gl_attr_wgl, w)
                                    .map(Context::Wgl)
                            }
                        }
                    }
                    _ => panic!(),
                }
            }
            _ => {
                let gl_attr_wgl =
                    gl_attr.clone().map_sharing(|ctx| match *ctx {
                        Context::HiddenWindowWgl(_, ref c)
                        | Context::Wgl(ref c) => c.get_hglrc(),
                        _ => panic!(),
                    });
                unsafe {
                    WglContext::new(&pf_reqs, &gl_attr_wgl, w).map(Context::Wgl)
                }
            }
        }
    }

    #[inline]
    pub fn new_context(
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        // if EGL is available, we try using EGL first
        // if EGL returns an error, we try the hidden window method
        match (gl_attr.sharing, &*EGL) {
            (None, Some(_))
            | (Some(&Context::Egl(_)), Some(_))
            | (Some(&Context::HiddenWindowEgl(_, _)), Some(_))
            | (Some(&Context::EglPbuffer(_)), Some(_)) => {
                let gl_attr_egl =
                    gl_attr.clone().map_sharing(|ctx| match *ctx {
                        Context::Egl(ref c)
                        | Context::EglPbuffer(ref c)
                        | Context::HiddenWindowEgl(_, ref c) => c,
                        _ => unreachable!(),
                    });

                let native_display = egl::NativeDisplay::Other(None);
                let context =
                    EglContext::new(pf_reqs, &gl_attr_egl, native_display)
                        .and_then(|prototype| prototype.finish_pbuffer((1, 1)))
                        .map(|ctx| Context::EglPbuffer(ctx));

                if let Ok(context) = context {
                    return Ok(context);
                }
            }
            _ => (),
        }

        let wb = winit::WindowBuilder::new().with_visibility(false);
        Self::new(wb, &el, pf_reqs, gl_attr).map(|(window, context)| {
            match context {
                Context::Egl(context) => {
                    Context::HiddenWindowEgl(window, context)
                }
                Context::Wgl(context) => {
                    Context::HiddenWindowWgl(window, context)
                }
                _ => unreachable!(),
            }
        })
    }

    #[inline]
    pub fn resize(&self, _width: u32, _height: u32) {
        // Method is for API consistency.
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => {
                c.make_current()
            }
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.make_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => {
                c.is_current()
            }
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.is_current(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => {
                c.get_proc_address(addr)
            }
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::Wgl(ref c) => c.swap_buffers(),
            Context::Egl(ref c) => c.swap_buffers(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => {
                c.get_api()
            }
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.get_api(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::Wgl(ref c) => c.get_pixel_format(),
            Context::Egl(ref c) => c.get_pixel_format(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => {
                RawHandle::Wgl(c.get_hglrc())
            }
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => RawHandle::Egl(c.raw_handle()),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match *self {
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => Some(c.get_egl_display()),
            _ => None,
        }
    }
}
