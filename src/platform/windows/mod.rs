#![cfg(target_os = "windows")]

use Api;
use ContextError;
use CreationError;
use PixelFormat;
use PixelFormatRequirements;
use GlAttributes;
use GlContext;
use WindowAttributes;

use winit;

use api::egl::ffi::egl::Egl;
use api::egl;
use api::egl::Context as EglContext;

use std::ffi::CString;
use std::ops::{Deref, DerefMut};
use kernel32;

pub use self::window::{EventsLoop, EventsLoopProxy};

mod window;

/// Stupid wrapper because `*const libc::c_void` doesn't implement `Sync`.
struct EglWrapper(Egl);
unsafe impl Sync for EglWrapper {}

lazy_static! {
    // An EGL implementation available on the system.
    static ref EGL: Option<EglWrapper> = {
        // the ATI drivers provide an EGL implementation in their DLLs
        let ati_dll_name = if cfg!(target_pointer_width = "64") {
            b"atio6axx.dll\0"
        } else {
            b"atioglxx.dll\0"
        };

        for dll_name in &[b"libEGL.dll\0" as &[u8], ati_dll_name] {
            let dll = unsafe { kernel32::LoadLibraryA(dll_name.as_ptr() as *const _) };
            if dll.is_null() {
                continue;
            }

            let egl = Egl::load_with(|name| {
                let name = CString::new(name).unwrap();
                unsafe { kernel32::GetProcAddress(dll, name.as_ptr()) as *const _ }
            });

            return Some(EglWrapper(egl))
        }

        None
    };
}

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;
#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

/// The Win32 implementation of the main `Window` object.
pub struct Window(window::Window);

impl Window {
    /// See the docs in the crate root file.
    #[inline]
    pub fn new(
        events_loop: &EventsLoop,
        window: &WindowAttributes,
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        _: &PlatformSpecificWindowBuilderAttributes,
        winit_builder: winit::WindowBuilder,
    ) -> Result<Window, CreationError> {
        window::Window::new(
            events_loop,
            window,
            pf_reqs,
            &opengl.clone().map_sharing(|w| &w.0),
            EGL.as_ref().map(|w| &w.0),
            winit_builder,
        ).map(|w| Window(w))
    }
}

impl Deref for Window {
    type Target = window::Window;

    #[inline]
    fn deref(&self) -> &window::Window {
        &self.0
    }
}

impl DerefMut for Window {
    #[inline]
    fn deref_mut(&mut self) -> &mut window::Window {
        &mut self.0
    }
}

///
pub enum HeadlessContext {
    /// A regular window, but invisible.
    HiddenWindow(EventsLoop, window::Window),
    /// An EGL pbuffer.
    EglPbuffer(EglContext),
}

impl HeadlessContext {
    pub fn new(dimensions: (u32, u32), pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&HeadlessContext>,
               _: &PlatformSpecificHeadlessBuilderAttributes)
               -> Result<HeadlessContext, CreationError>
    {
        // if EGL is available, we try using EGL first
        // if EGL returns an error, we try the hidden window method
        if let &Some(ref egl) = &*EGL {
            let context = EglContext::new(egl.0.clone(), pf_reqs, &opengl.clone().map_sharing(|_| unimplemented!()),       // TODO:
                                          egl::NativeDisplay::Other(None))
                                .and_then(|prototype| prototype.finish_pbuffer(dimensions))
                                .map(|ctxt| HeadlessContext::EglPbuffer(ctxt));

            if let Ok(context) = context {
                return Ok(context);
            }
        }
        let events_loop = EventsLoop::new();
        let winit_builder = winit::WindowBuilder::new().with_visibility(false);
        let window = try!(window::Window::new(&events_loop,
                                              &WindowAttributes { visible: false, .. Default::default() },
                                              pf_reqs,
                                              &opengl.clone().map_sharing(|_| unimplemented!()),            //TODO:
                                              EGL.as_ref().map(|w| &w.0),
                                              winit_builder));
        Ok(HeadlessContext::HiddenWindow(events_loop, window))
    }
}

impl GlContext for HeadlessContext {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self {
            &HeadlessContext::HiddenWindow(_, ref ctxt) => ctxt.make_current(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.make_current(),
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self {
            &HeadlessContext::HiddenWindow(_, ref ctxt) => ctxt.is_current(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.is_current(),
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self {
            &HeadlessContext::HiddenWindow(_, ref ctxt) => ctxt.get_proc_address(addr),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_proc_address(addr),
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self {
            &HeadlessContext::HiddenWindow(_, ref ctxt) => ctxt.swap_buffers(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.swap_buffers(),
        }
    }

    #[inline]
    fn get_api(&self) -> Api {
        match self {
            &HeadlessContext::HiddenWindow(_, ref ctxt) => ctxt.get_api(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_api(),
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self {
            &HeadlessContext::HiddenWindow(_, ref ctxt) => ctxt.get_pixel_format(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_pixel_format(),
        }
    }
}
