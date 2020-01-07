#![cfg(target_os = "windows")]

use crate::{
    Api, ContextCurrentState, ContextError, CreationError, GlAttributes,
    GlRequest, NotCurrent, PixelFormat, PixelFormatRequirements, Rect,
};

use crate::api::egl::{
    Context as EglContext, NativeDisplay, SurfaceType as EglSurfaceType, EGL,
};
use crate::api::wgl::Context as WglContext;
use crate::platform::windows::WindowExtWindows;

use glutin_egl_sys as ffi;
use winapi::shared::windef::{HGLRC, HWND};
use winit;
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

use std::marker::PhantomData;
use std::os::raw;

/// Context handles available on Windows.
#[derive(Clone, Debug)]
pub enum RawHandle {
    Egl(ffi::EGLContext),
    Wgl(HGLRC),
}

#[derive(Debug)]
pub enum Context {
    /// A regular window
    Egl(EglContext),
    Wgl(WglContext),
    /// A regular window, but invisible.
    HiddenWindowEgl(Window, EglContext),
    HiddenWindowWgl(Window, WglContext),
    /// An EGL pbuffer.
    EglPbuffer(EglContext),
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    /// See the docs in the crate root file.
    #[inline]
    pub fn new_windowed<T>(
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<(Window, Self), CreationError> {
        let win = wb.build(el)?;
        let hwnd = win.hwnd() as HWND;
        let ctx = Self::new_raw_context(hwnd, pf_reqs, gl_attr)?;

        Ok((win, ctx))
    }

    #[inline]
    pub fn new_raw_context(
        hwnd: HWND,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<Self, CreationError> {
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
                            WglContext::new(&pf_reqs, &gl_attr_wgl, hwnd)
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
                            NativeDisplay::Other(Some(std::ptr::null())),
                            EglSurfaceType::Window,
                            |c, _| Ok(c[0]),
                        )
                        .and_then(|p| p.finish(hwnd))
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
                            NativeDisplay::Other(Some(std::ptr::null())),
                            EglSurfaceType::Window,
                            |c, _| Ok(c[0]),
                        )
                        .and_then(|p| p.finish(hwnd))
                        {
                            Ok(Context::Egl(c))
                        } else {
                            unsafe {
                                WglContext::new(&pf_reqs, &gl_attr_wgl, hwnd)
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
                    WglContext::new(&pf_reqs, &gl_attr_wgl, hwnd)
                        .map(Context::Wgl)
                }
            }
        }
    }

    #[inline]
    pub fn new_headless<T>(
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: dpi::PhysicalSize<u32>,
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

                let native_display = NativeDisplay::Other(None);
                let context = EglContext::new(
                    pf_reqs,
                    &gl_attr_egl,
                    native_display,
                    EglSurfaceType::PBuffer,
                    |c, _| Ok(c[0]),
                )
                .and_then(|prototype| prototype.finish_pbuffer(size))
                .map(|ctx| Context::EglPbuffer(ctx));

                if let Ok(context) = context {
                    return Ok(context);
                }
            }
            _ => (),
        }

        let wb = WindowBuilder::new()
            .with_visible(false)
            .with_inner_size(size);
        Self::new_windowed(wb, &el, pf_reqs, gl_attr).map(|(win, context)| {
            match context {
                Context::Egl(context) => Context::HiddenWindowEgl(win, context),
                Context::Wgl(context) => Context::HiddenWindowWgl(win, context),
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
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::Wgl(ref c) | Context::HiddenWindowWgl(_, ref c) => {
                c.make_not_current()
            }
            Context::Egl(ref c)
            | Context::HiddenWindowEgl(_, ref c)
            | Context::EglPbuffer(ref c) => c.make_not_current(),
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
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
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
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        Err(ContextError::OsError(
            "buffer damage not suported".to_string(),
        ))
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        false
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

pub trait RawContextExt {
    /// Creates a raw context on the provided window.
    ///
    /// Unsafe behaviour might happen if you:
    ///   - Provide us with invalid parameters.
    ///   - The window is destroyed before the context
    unsafe fn build_raw_context(
        self,
        hwnd: *mut raw::c_void,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextCurrentState> RawContextExt
    for crate::ContextBuilder<'a, T>
{
    #[inline]
    unsafe fn build_raw_context(
        self,
        hwnd: *mut raw::c_void,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::new_raw_context(hwnd as *mut _, &pf_reqs, &gl_attr)
            .map(|context| crate::Context {
                context,
                phantom: PhantomData,
            })
            .map(|context| crate::RawContext {
                context,
                window: (),
            })
    }
}
