#![cfg(target_os = "windows")]

use std::ffi::CString;
use std::ops::{Deref, DerefMut};

use winapi::shared::windef::HGLRC;
use winapi::um::libloaderapi::*;
use winit;

use Api;
use ContextError;
use CreationError;
use PixelFormat;
use PixelFormatRequirements;
use GlAttributes;

use api::egl::ffi::egl::Egl;
use api::egl;
use api::egl::Context as EglContext;

mod context;

/// Context handles available on Windows.
#[derive(Clone, Debug)]
pub enum RawHandle {
    Egl(egl::ffi::EGLContext),
    Wgl(HGLRC),
}

/// Stupid wrapper because `*const libc::c_void` doesn't implement `Sync`.
struct EglWrapper(Egl);
unsafe impl Sync for EglWrapper {}

#[cfg(feature = "windows-static-egl")]
fn static_egl() -> Option<EglWrapper> {
    use std::os::raw::{c_char, c_void};

    extern {
        fn eglGetProcAddress(name: *const c_char) -> *const c_void;
    }

    Some(EglWrapper(Egl::load_with(|name| {
        let name = CString::new(name).unwrap();
        unsafe { eglGetProcAddress(name.as_ptr()) }
    })))
}

#[cfg(not(feature = "windows-static-egl"))]
fn static_egl() -> Option<EglWrapper> {
    None
}

lazy_static! {
    // An EGL implementation available on the system.
    static ref EGL: Option<EglWrapper> = static_egl().or_else(|| {
        // the ATI drivers provide an EGL implementation in their DLLs
        let ati_dll_name = if cfg!(target_pointer_width = "64") {
            b"atio6axx.dll\0"
        } else {
            b"atioglxx.dll\0"
        };

        for dll_name in &[b"libEGL.dll\0" as &[u8], ati_dll_name] {
            let dll = unsafe { LoadLibraryA(dll_name.as_ptr() as *const _) };
            if dll.is_null() {
                continue;
            }

            let egl = Egl::load_with(|name| {
                let name = CString::new(name).unwrap();
                unsafe { GetProcAddress(dll, name.as_ptr()) as *const _ }
            });

            return Some(EglWrapper(egl))
        }

        None
    });
}

/// The Win32 implementation of the main `Context` object.
pub struct Context(context::Context);

impl Context {
    /// See the docs in the crate root file.
    #[inline]
    pub fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Self>,
    ) -> Result<(winit::Window, Self), CreationError> {
        context::Context::new(
            window_builder,
            events_loop,
            pf_reqs,
            &opengl.clone().map_sharing(|w| &w.0),
            EGL.as_ref().map(|w| &w.0),
        ).map(|(w, c)| (w, Context(c)))
    }
}

impl Deref for Context {
    type Target = context::Context;

    #[inline]
    fn deref(&self) -> &context::Context {
        &self.0
    }
}

impl DerefMut for Context {
    #[inline]
    fn deref_mut(&mut self) -> &mut context::Context {
        &mut self.0
    }
}

#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

pub enum HeadlessContext {
    /// A regular window, but invisible.
    HiddenWindow(winit::EventsLoop, winit::Window, context::Context),
    /// An EGL pbuffer.
    EglPbuffer(EglContext),
}

impl HeadlessContext {
    pub fn new(
        dimensions: (u32, u32),
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
        _: &PlatformSpecificHeadlessBuilderAttributes,
    ) -> Result<Self, CreationError>
    {
        // if EGL is available, we try using EGL first
        // if EGL returns an error, we try the hidden window method
        if let &Some(ref egl) = &*EGL {
            let gl_attr = &gl_attr.clone().map_sharing(|_| unimplemented!()); // TODO
            let native_display = egl::NativeDisplay::Other(None);
            let context = EglContext::new(egl.0.clone(), pf_reqs, &gl_attr, native_display)
                .and_then(|prototype| prototype.finish_pbuffer(dimensions))
                .map(|ctxt| HeadlessContext::EglPbuffer(ctxt));
            if let Ok(context) = context {
                return Ok(context);
            }
        }
        let events_loop = winit::EventsLoop::new();
        let window_builder = winit::WindowBuilder::new().with_visibility(false);
        let gl_attr = &gl_attr.clone().map_sharing(|_| unimplemented!());
        let egl = EGL.as_ref().map(|w| &w.0);
        context::Context::new(window_builder, &events_loop, pf_reqs, gl_attr, egl)
            .map(|(window, context)| HeadlessContext::HiddenWindow(events_loop, window, context))
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self {
            &HeadlessContext::HiddenWindow(_, _, ref ctxt) => ctxt.make_current(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.make_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self {
            &HeadlessContext::HiddenWindow(_, _, ref ctxt) => ctxt.is_current(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.is_current(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match self {
            &HeadlessContext::HiddenWindow(_, _, ref ctxt) => ctxt.get_proc_address(addr),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match self {
            &HeadlessContext::HiddenWindow(_, _, ref ctxt) => ctxt.swap_buffers(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.swap_buffers(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self {
            &HeadlessContext::HiddenWindow(_, _, ref ctxt) => ctxt.get_api(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_api(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match self {
            &HeadlessContext::HiddenWindow(_, _, ref ctxt) => ctxt.get_pixel_format(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_pixel_format(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            HeadlessContext::HiddenWindow(_, _, ref ctxt) => ctxt.raw_handle(),
            HeadlessContext::EglPbuffer(ref ctxt) => RawHandle::Egl(ctxt.raw_handle()),
        }
    }
}
