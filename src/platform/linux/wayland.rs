use std::ffi::CString;
use winit;
use winit::os::unix::WindowExt;
use {ContextError, CreationError, GlAttributes, GlContext, PixelFormat, PixelFormatRequirements};
use api::dlopen;
use api::egl;
use api::egl::Context as EglContext;
use wayland_client::egl as wegl;

pub struct Window {
    egl_surface: wegl::WlEglSurface,
    context: EglContext,
}

impl Window {
    pub fn new(
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        winit_window: &winit::Window,
    ) -> Result<Window, CreationError> {
        let (w, h) = winit_window.get_inner_size().unwrap();
        let surface = winit_window.get_wayland_client_surface().unwrap();
        let egl_surface = wegl::WlEglSurface::new(surface, w as i32, h as i32);
        let context = {
            let libegl = unsafe { dlopen::dlopen(b"libEGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
            if libegl.is_null() {
                return Err(CreationError::NotSupported);
            }
            let egl = ::api::egl::ffi::egl::Egl::load_with(|sym| {
                let sym = CString::new(sym).unwrap();
                unsafe { dlopen::dlsym(libegl, sym.as_ptr()) }
            });
            try!(EglContext::new(
                egl,
                pf_reqs, &opengl.clone().map_sharing(|_| unimplemented!()),        // TODO:
                egl::NativeDisplay::Wayland(Some(winit_window.get_wayland_display().unwrap())))
                .and_then(|p| p.finish(egl_surface.ptr() as *const _))
            )
        };
        Ok(Window {
            egl_surface: egl_surface,
            context: context
        })
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.context.make_current()
    }

    #[inline]
    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        self.context.get_proc_address(addr)
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        self.context.swap_buffers()
    }

    #[inline]
    fn get_api(&self) -> ::Api {
        self.context.get_api()
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.context.get_pixel_format().clone()
    }
}
