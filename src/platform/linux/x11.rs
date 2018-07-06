pub use winit::os::unix::x11::{XError, XNotSupported, XConnection};

use std::{mem, ptr, fmt, error};
use std::sync::Arc;

use winit;
use winit::os::unix::{EventsLoopExt, WindowExt, WindowBuilderExt};

use {Api, ContextError, CreationError, GlAttributes, GlRequest, PixelFormat, PixelFormatRequirements};

use std::ffi::CString;

use api::glx::{ffi, Context as GlxContext};
use api::{dlopen, egl};
use api::egl::Context as EglContext;
use api::glx::ffi::glx::Glx;
use api::egl::ffi::egl::Egl;

#[derive(Debug)]
struct NoX11Connection;

impl error::Error for NoX11Connection {
    fn description(&self) -> &str {
        "failed to get x11 connection"
    }
}

impl fmt::Display for NoX11Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(error::Error::description(self))
    }
}

struct GlxOrEgl {
    glx: Option<Glx>,
    egl: Option<Egl>,
}

impl GlxOrEgl {
    fn new() -> GlxOrEgl {
        // TODO: use something safer than raw "dlopen"
        let glx = {
            let mut libglx = unsafe {
                dlopen::dlopen(b"libGL.so.1\0".as_ptr() as *const _, dlopen::RTLD_NOW)
            };
            if libglx.is_null() {
                libglx = unsafe {
                    dlopen::dlopen(b"libGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW)
                };
            }
            if libglx.is_null() {
                None
            } else {
                Some(Glx::load_with(|sym| {
                    let sym = CString::new(sym).unwrap();
                    unsafe { dlopen::dlsym(libglx, sym.as_ptr()) }
                }))
            }
        };
        // TODO: use something safer than raw "dlopen"
        let egl = {
            let mut libegl = unsafe {
                dlopen::dlopen(b"libEGL.so.1\0".as_ptr() as *const _, dlopen::RTLD_NOW)
            };
            if libegl.is_null() {
                libegl = unsafe {
                    dlopen::dlopen(b"libEGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW)
                };
            }
            if libegl.is_null() {
                None
            } else {
                Some(Egl::load_with(|sym| {
                    let sym = CString::new(sym).unwrap();
                    unsafe { dlopen::dlsym(libegl, sym.as_ptr()) }
                }))
            }
        };
        GlxOrEgl {
            glx: glx,
            egl: egl,
        }
    }
}

pub enum GlContext {
    Glx(GlxContext),
    Egl(EglContext),
    None,
}

pub struct Context {
    display: Arc<XConnection>,
    colormap: ffi::Colormap,
    context: GlContext,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // we don't call MakeCurrent(0, 0) because we are not sure that the context
            // is still the current one
            self.context = GlContext::None;

            (self.display.xlib.XFreeColormap)(self.display.display, self.colormap);
        }
    }
}

impl Context {
    pub fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError>
    {
        let display = match events_loop.get_xlib_xconnection() {
            Some(display) => display,
            None => return Err(CreationError::NoBackendAvailable(Box::new(NoX11Connection))),
        };

        // Get the screen_id for the window being built.
        let screen_id = unsafe { (display.xlib.XDefaultScreen)(display.display) };

        // start the context building process
        enum Prototype<'a> {
            Glx(::api::glx::ContextPrototype<'a>),
            Egl(::api::egl::ContextPrototype<'a>),
        }

        let builder = gl_attr.clone();

        let builder_glx_u;
        let builder_egl_u;

        let backend = GlxOrEgl::new();
        let context = match gl_attr.version {
            GlRequest::Latest |
            GlRequest::Specific(Api::OpenGl, _) |
            GlRequest::GlThenGles { .. } => {
                // GLX should be preferred over EGL, otherwise crashes may occur
                // on X11 – issue #314
                if let Some(ref glx) = backend.glx {
                    builder_glx_u = builder.map_sharing(|c| match c.context {
                        GlContext::Glx(ref c) => c,
                        _ => panic!(),
                    });
                    Prototype::Glx(GlxContext::new(
                        glx.clone(),
                        &display.xlib,
                        pf_reqs,
                        &builder_glx_u,
                        display.display,
                        screen_id,
                        window_builder.window.transparent,
                    )?)
                } else if let Some(ref egl) = backend.egl {
                    builder_egl_u = builder.map_sharing(|c| match c.context {
                        GlContext::Egl(ref c) => c,
                        _ => panic!(),
                    });
                    let native_display = egl::NativeDisplay::X11(Some(display.display as *const _));
                    Prototype::Egl(EglContext::new(
                        egl.clone(),
                        pf_reqs,
                        &builder_egl_u,
                        native_display,
                    )?)
                } else {
                    return Err(CreationError::NotSupported("both libglx and libEGL not present"));
                }
            },
            GlRequest::Specific(Api::OpenGlEs, _) => {
                if let Some(ref egl) = backend.egl {
                    builder_egl_u = builder.map_sharing(|c| match c.context {
                        GlContext::Egl(ref c) => c,
                        _ => panic!(),
                    });
                    Prototype::Egl(EglContext::new(
                        egl.clone(),
                        pf_reqs,
                        &builder_egl_u,
                        egl::NativeDisplay::X11(Some(display.display as *const _)),
                    )?)
                } else {
                    return Err(CreationError::NotSupported("libEGL not present"));
                }
            },
            GlRequest::Specific(_, _) => {
                return Err(CreationError::NotSupported("requested specific without gl or gles"));
            },
        };

        // getting the `visual_infos` (a struct that contains information about the visual to use)
        let visual_infos = match context {
            Prototype::Glx(ref p) => p.get_visual_infos().clone(),
            Prototype::Egl(ref p) => {
                unsafe {
                    let mut template: ffi::XVisualInfo = mem::zeroed();
                    template.visualid = p.get_native_visual_id() as ffi::VisualID;

                    let mut num_visuals = 0;
                    let vi = (display.xlib.XGetVisualInfo)(display.display, ffi::VisualIDMask,
                                                           &mut template, &mut num_visuals);
                    display.check_errors().expect("Failed to call XGetVisualInfo");
                    assert!(!vi.is_null());
                    assert!(num_visuals == 1);

                    let vi_copy = ptr::read(vi as *const _);
                    (display.xlib.XFree)(vi as *mut _);
                    vi_copy
                }
            },
        };

        let window = window_builder
                .with_x11_visual(&visual_infos as *const _)
                .with_x11_screen(screen_id)
                .build(events_loop)?;

        let xlib_window = window.get_xlib_window().unwrap();
        // finish creating the OpenGL context
        let context = match context {
            Prototype::Glx(ctxt) => {
                GlContext::Glx(ctxt.finish(xlib_window)?)
            },
            Prototype::Egl(ctxt) => {
                GlContext::Egl(ctxt.finish(xlib_window as _)?)
            },
        };

        // getting the root window
        let root = unsafe { (display.xlib.XDefaultRootWindow)(display.display) };
        display.check_errors().expect("Failed to get root window");

        // creating the color map
        let cmap = unsafe {
            let cmap = (display.xlib.XCreateColormap)(display.display, root,
                                                      visual_infos.visual as *mut _,
                                                      ffi::AllocNone);
            display.check_errors().expect("Failed to call XCreateColormap");
            cmap
        };

        let context = Context {
            display: display.clone(),
            context: context,
            colormap: cmap,
        };

        Ok((window, context))
    }

    #[inline]
    pub fn resize(&self, w: ffi::Window, width: u32, height: u32) {
        unsafe {
            match self.context {
                GlContext::Egl(_) | GlContext::Glx(_) => {
                    assert_eq!((self.display.xlib.XResizeWindow)(
                        self.display.display,
                        w,
                        width as _,
                        height as _
                    ), 0);
                }
                GlContext::None => ()
            }
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self.context {
            GlContext::Glx(ref ctxt) => ctxt.make_current(),
            GlContext::Egl(ref ctxt) => ctxt.make_current(),
            GlContext::None => Ok(())
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self.context {
            GlContext::Glx(ref ctxt) => ctxt.is_current(),
            GlContext::Egl(ref ctxt) => ctxt.is_current(),
            GlContext::None => panic!()
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match self.context {
            GlContext::Glx(ref ctxt) => ctxt.get_proc_address(addr),
            GlContext::Egl(ref ctxt) => ctxt.get_proc_address(addr),
            GlContext::None => ptr::null()
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match self.context {
            GlContext::Glx(ref ctxt) => ctxt.swap_buffers(),
            GlContext::Egl(ref ctxt) => ctxt.swap_buffers(),
            GlContext::None => Ok(())
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self.context {
            GlContext::Glx(ref ctxt) => ctxt.get_api(),
            GlContext::Egl(ref ctxt) => ctxt.get_api(),
            GlContext::None => panic!()
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match self.context {
            GlContext::Glx(ref ctxt) => ctxt.get_pixel_format(),
            GlContext::Egl(ref ctxt) => ctxt.get_pixel_format(),
            GlContext::None => panic!()
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> &GlContext {
        &self.context
    }
}
