#![cfg(target_os = "emscripten")]

use crate::{
    Api, ContextCurrentState, ContextError, CreationError, GlAttributes,
    GlRequest, NotCurrentContext, PixelFormat, PixelFormatRequirements,
    PossiblyCurrentContext,
};

use glutin_emscripten_sys as ffi;
use takeable_option::Takeable;
use winit;
use winit::dpi;

use std::ffi::CString;
use std::marker::PhantomData;

#[derive(Debug)]
pub enum ContextInner {
    Window(ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE),
    WindowedContext(winit::Window, ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE),
}

impl ContextInner {
    fn raw_handle(&self) -> ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE {
        match self {
            ContextInner::Window(c) => *c,
            ContextInner::WindowedContext(_, c) => *c,
        }
    }
}

#[derive(Debug)]
pub struct Context<T: ContextCurrentState> {
    inner: Takeable<ContextInner>,
    phantom: PhantomData<T>,
}

impl<T: ContextCurrentState> Context<T> {
    #[inline]
    pub fn new_windowed(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        _pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context<T>>,
    ) -> Result<(winit::Window, Context<NotCurrentContext>), CreationError>
    {
        let win = wb.build(el)?;

        let gl_attr = gl_attr.clone().map_sharing(|_| {
            unimplemented!("Shared contexts are unimplemented in WebGL.")
        });

        // getting the default values of attributes
        let mut attributes = unsafe {
            let mut attributes: ffi::EmscriptenWebGLContextAttributes =
                std::mem::uninitialized();
            ffi::emscripten_webgl_init_context_attributes(&mut attributes);
            attributes
        };

        // setting the attributes
        if let GlRequest::Specific(Api::WebGl, (major, minor)) = gl_attr.version
        {
            attributes.majorVersion = major as _;
            attributes.minorVersion = minor as _;
        }

        // creating the context
        let context = unsafe {
            // TODO: correct first parameter based on the window
            let context = ffi::emscripten_webgl_create_context(
                std::ptr::null(),
                &attributes,
            );
            if context <= 0 {
                return Err(CreationError::OsError(format!(
                    "Error while calling emscripten_webgl_create_context: {}",
                    error_to_str(std::mem::transmute(context))
                )));
            }
            context
        };

        // TODO: emscripten_set_webglcontextrestored_callback

        Ok((
            win,
            Context {
                inner: Takeable::new(ContextInner::Window(context)),
                phantom: PhantomData,
            },
        ))
    }

    #[inline]
    pub fn new_headless(
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context<T>>,
        dims: dpi::PhysicalSize,
    ) -> Result<Context<NotCurrentContext>, CreationError> {
        let wb = winit::WindowBuilder::new()
            .with_visibility(false)
            .with_dimensions(dims.to_logical(1.));

        Self::new_windowed(wb, el, pf_reqs, gl_attr).map(|(w, c)| {
            match *c.inner {
                ContextInner::Window(c) => Context {
                    inner: Takeable::new(ContextInner::WindowedContext(w, c)),
                    phantom: PhantomData,
                },
                _ => panic!(),
            }
        })
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe {
            ffi::emscripten_webgl_get_current_context() == self.raw_handle()
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        Api::WebGl
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EMSCRIPTEN_WEBGL_CONTEXT_HANDLE {
        self.inner.raw_handle()
    }

    fn state_sub<T2, E, F>(mut self, f: F) -> Result<Context<T2>, (Self, E)>
    where
        T2: ContextCurrentState,
        F: FnOnce(ContextInner) -> Result<ContextInner, (ContextInner, E)>,
    {
        match f(Takeable::take(&mut self.inner)) {
            Ok(inner) => Ok(Context {
                inner: Takeable::new(inner),
                phantom: PhantomData,
            }),
            Err((inner, err)) => Err((
                Context {
                    inner: Takeable::new(inner),
                    phantom: PhantomData,
                },
                err,
            )),
        }
    }

    #[inline]
    pub unsafe fn make_current(
        self,
    ) -> Result<Context<PossiblyCurrentContext>, (Self, ContextError)> {
        self.state_sub(
            |inner| match ffi::emscripten_webgl_make_context_current(
                inner.raw_handle(),
            ) {
                ffi::EMSCRIPTEN_RESULT_SUCCESS => Ok(inner),
                err => Err((
                    inner,
                    ContextError::OsError(format!(
                        "`emscripten_webgl_make_context_current` failed: {:?}",
                        err
                    )),
                )),
            },
        )
    }

    #[inline]
    pub unsafe fn make_not_current(
        self,
    ) -> Result<Context<NotCurrentContext>, (Self, ContextError)> {
        self.state_sub(
            |inner| match ffi::emscripten_webgl_make_context_current(0) {
                ffi::EMSCRIPTEN_RESULT_SUCCESS => Ok(inner),
                err => Err((
                    inner,
                    ContextError::OsError(format!(
                        "`emscripten_webgl_make_context_current` failed: {:?}",
                        err
                    )),
                )),
            },
        )
    }

    #[inline]
    pub unsafe fn treat_as_not_current(self) -> Context<NotCurrentContext> {
        self.state_sub::<_, (), _>(|inner| Ok(inner)).unwrap()
    }
}

impl Context<PossiblyCurrentContext> {
    #[inline]
    pub fn resize(&self, _width: u32, _height: u32) {
        match *self.inner {
            ContextInner::Window(_) => (), // TODO: ?
            ContextInner::WindowedContext(_, _) => unreachable!(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let addr = CString::new(addr).unwrap();

        unsafe {
            // FIXME: if `as_ptr()` is used, then wrong data is passed to
            // emscripten
            ffi::emscripten_GetProcAddress(addr.into_raw() as *const _)
                as *const _
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        Ok(())
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
}

impl<T: ContextCurrentState> Drop for Context<T> {
    fn drop(&mut self) {
        if let Some(inner) = Takeable::try_take(&mut self.inner) {
            unsafe {
                ffi::emscripten_webgl_destroy_context(inner.raw_handle());
            }
        }
    }
}

fn error_to_str(code: ffi::EMSCRIPTEN_RESULT) -> &'static str {
    match code {
        ffi::EMSCRIPTEN_RESULT_SUCCESS | ffi::EMSCRIPTEN_RESULT_DEFERRED => {
            "Internal error in the library (success detected as failure)"
        }

        ffi::EMSCRIPTEN_RESULT_NOT_SUPPORTED => "Not supported",
        ffi::EMSCRIPTEN_RESULT_FAILED_NOT_DEFERRED => "Failed not deferred",
        ffi::EMSCRIPTEN_RESULT_INVALID_TARGET => "Invalid target",
        ffi::EMSCRIPTEN_RESULT_UNKNOWN_TARGET => "Unknown target",
        ffi::EMSCRIPTEN_RESULT_INVALID_PARAM => "Invalid parameter",
        ffi::EMSCRIPTEN_RESULT_FAILED => "Failed",
        ffi::EMSCRIPTEN_RESULT_NO_DATA => "No data",

        _ => "Undocumented error",
    }
}
