#![cfg(target_os = "emscripten")]

use std::ffi::CString;

use {Api, ContextError, CreationError, GlAttributes, GlRequest};
use {PixelFormat, PixelFormatRequirements};

use winit;

mod ffi;

pub struct Context {
    context: ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
}

impl Context {
    #[inline]
    pub fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError>
    {
        let window = window_builder.build(events_loop)?;

        // getting the default values of attributes
        let mut attributes = unsafe {
            use std::mem;
            let mut attributes: ffi::EmscriptenWebGLContextAttributes = mem::uninitialized();
            ffi::emscripten_webgl_init_context_attributes(&mut attributes);
            attributes
        };

        // setting the attributes
        if let GlRequest::Specific(Api::WebGl, (major, minor)) = gl_attr.version {
            attributes.majorVersion = major as _;
            attributes.minorVersion = minor as _;
        }

        // creating the context
        let context = unsafe {
            use std::{mem, ptr};
            // TODO: correct first parameter based on the window
            let context = ffi::emscripten_webgl_create_context(ptr::null(), &attributes);
            if context <= 0 {
                return Err(CreationError::OsError(format!("Error while calling emscripten_webgl_create_context: {}",
                    error_to_str(mem::transmute(context)))));
            }
            context
        };

        // TODO: emscripten_set_webglcontextrestored_callback

        let ctxt = Context {
            context: context
        };

        Ok((window, ctxt))
    }

    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        // TODO: ?
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        // TOOD: check if == EMSCRIPTEN_RESULT
        ffi::emscripten_webgl_make_context_current(self.context);
        Ok(())
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe {
            ffi::emscripten_webgl_get_current_context() == self.context
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let addr = CString::new(addr).unwrap();

        unsafe {
            // FIXME: if `as_ptr()` is used, then wrong data is passed to emscripten
            ffi::emscripten_GetProcAddress(addr.into_raw() as *const _) as *const _
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        Ok(())
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        Api::WebGl
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        // FIXME: this is a dummy pixel format
        PixelFormat {
            hardware_accelerated: true,
            color_bits: 24,
            alpha_bits: 8,
            depth_bits: 24,
            stencil_bits: 8,
            stereoscopy: false,
            double_buffer: true,
            multisampling: None,
            srgb: true,
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE {
        self.context
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            ffi::emscripten_webgl_destroy_context(self.context);
        }
    }
}

#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

pub struct HeadlessContext {
    context: ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE,
}

impl HeadlessContext {
    pub fn new(dimensions: (u32, u32), pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&HeadlessContext>,
               _: &PlatformSpecificHeadlessBuilderAttributes)
               -> Result<HeadlessContext, CreationError>
    {
        unimplemented!()
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        // TOOD: check if == EMSCRIPTEN_RESULT
        ffi::emscripten_webgl_make_context_current(self.context);
        Ok(())
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe {
            ffi::emscripten_webgl_get_current_context() == self.context
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let addr = CString::new(addr).unwrap();

        unsafe {
            // FIXME: if `as_ptr()` is used, then wrong data is passed to emscripten
            ffi::emscripten_GetProcAddress(addr.into_raw() as *const _) as *const _
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        Ok(())
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        Api::WebGl
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        unimplemented!()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE {
        self.context
    }
}

fn error_to_str(code: ffi::EMSCRIPTEN_RESULT) -> &'static str {
    match code {
        ffi::EMSCRIPTEN_RESULT_SUCCESS | ffi::EMSCRIPTEN_RESULT_DEFERRED
            => "Internal error in the library (success detected as failure)",

        ffi::EMSCRIPTEN_RESULT_NOT_SUPPORTED => "Not supported",
        ffi::EMSCRIPTEN_RESULT_FAILED_NOT_DEFERRED => "Failed not deferred",
        ffi::EMSCRIPTEN_RESULT_INVALID_TARGET => "Invalid target",
        ffi::EMSCRIPTEN_RESULT_UNKNOWN_TARGET => "Unknown target",
        ffi::EMSCRIPTEN_RESULT_INVALID_PARAM => "Invalid parameter",
        ffi::EMSCRIPTEN_RESULT_FAILED => "Failed",
        ffi::EMSCRIPTEN_RESULT_NO_DATA => "No data",

        _ => "Undocumented error"
    }
}
