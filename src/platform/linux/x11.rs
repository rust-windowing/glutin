pub use winit::os::unix::x11::{XError, XNotSupported, XConnection};

use std::{mem, ptr, fmt, error};
use std::ffi::CString;
use std::sync::Arc;

use winit;
use winit::os::unix::{EventsLoopExt, WindowExt, WindowBuilderExt};

use {Api, ContextError, CreationError, GlAttributes, GlRequest, PixelFormat, PixelFormatRequirements};

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
    xconn: Arc<XConnection>,
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

            (self.xconn.xlib.XFreeColormap)(self.xconn.display, self.colormap);
        }
    }
}

impl Context {
    pub unsafe fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError>
    {
        let xconn = match events_loop.get_xlib_xconnection() {
            Some(xconn) => xconn,
            None => return Err(CreationError::NoBackendAvailable(Box::new(NoX11Connection))),
        };

        // Get the screen_id for the window being built.
        let screen_id = (xconn.xlib.XDefaultScreen)(xconn.display);

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
                        Arc::clone(&xconn),
                        pf_reqs,
                        &builder_glx_u,
                        screen_id,
                        window_builder.window.transparent,
                        None,
                    )?)
                } else if let Some(ref egl) = backend.egl {
                    builder_egl_u = builder.map_sharing(|c| match c.context {
                        GlContext::Egl(ref c) => c,
                        _ => panic!(),
                    });
                    let native_display = egl::NativeDisplay::X11(Some(xconn.display as *const _));
                    Prototype::Egl(EglContext::new(
                        egl.clone(),
                        pf_reqs,
                        &builder_egl_u,
                        native_display,
                        None,
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
                        egl::NativeDisplay::X11(Some(xconn.display as *const _)),
                        None,
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
                let mut template: ffi::XVisualInfo = mem::zeroed();
                template.visualid = p.get_native_visual_id() as ffi::VisualID;

                let mut num_visuals = 0;
                let vi = (xconn.xlib.XGetVisualInfo)(xconn.display, ffi::VisualIDMask,
                                                       &mut template, &mut num_visuals);
                xconn.check_errors().expect("Failed to call `XGetVisualInfo`");
                assert!(!vi.is_null());
                assert!(num_visuals == 1);

                let vi_copy = ptr::read(vi as *const _);
                (xconn.xlib.XFree)(vi as *mut _);
                vi_copy
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
        let root = (xconn.xlib.XDefaultRootWindow)(xconn.display);
        xconn.check_errors().expect("Failed to get root window");

        // creating the color map
        let colormap = {
            let cmap = (xconn.xlib.XCreateColormap)(xconn.display, root,
                                                      visual_infos.visual as *mut _,
                                                      ffi::AllocNone);
            xconn.check_errors().expect("Failed to call XCreateColormap");
            cmap
        };

        let context = Context {
            xconn: Arc::clone(&xconn),
            context,
            colormap,
        };

        Ok((window, context))
    }

    pub unsafe fn new_raw(
        rwindow: &super::RawWindow,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError>
    {
        match gl_attr.version {
            GlRequest::Specific(Api::OpenGl, _) | GlRequest::Specific(Api::OpenGlEs, _) => (),
            GlRequest::Specific(_, _) => {
                return Err(CreationError::NotSupported("requested specific without gl or gles"));
            },
            _ => (),
        }

        let backend = GlxOrEgl::new();
        let builder = gl_attr.clone();
        let (context, xconn, visual) = match *rwindow {
            super::RawWindow::EGLX11(ref xconn, window, visual) => {
                let builder = builder.map_sharing(|c| match c.context {
                    GlContext::Egl(ref c) => c,
                    _ => panic!(),
                });

                let native_display = egl::NativeDisplay::X11(Some(xconn.display as *const _));
                (
                    GlContext::Egl(EglContext::new(
                        backend.egl.ok_or(CreationError::NotSupported("libEGL not present"))?.clone(),
                        pf_reqs,
                        &builder,
                        native_display,
                        Some(visual),
                    )?.finish(window)?),
                    xconn,
                    visual,
                )
            },
            super::RawWindow::GLXX11(ref xconn, screen, window, visual, transparent) => {
                match gl_attr.version {
                    GlRequest::Specific(Api::OpenGlEs, _) => panic!(),
                    _ => (),
                }

                let builder = builder.map_sharing(|c| match c.context {
                    GlContext::Glx(ref c) => c,
                    _ => panic!(),
                });

                (
                    GlContext::Glx(GlxContext::new(
                        backend.glx.ok_or(CreationError::NotSupported("libglx not present"))?.clone(),
                        Arc::clone(&xconn),
                        pf_reqs,
                        &builder,
                        screen,
                        transparent,
                        Some(visual),
                    )?.finish(window as _)?),
                    xconn,
                    visual,
                )
            },
            _ => unimplemented!(),
        };

        // getting the root window
        let root = (xconn.xlib.XDefaultRootWindow)(xconn.display);
        xconn.check_errors().expect("Failed to get root window");

        // creating the color map
        let colormap = {
            let cmap = (xconn.xlib.XCreateColormap)(xconn.display, root,
                                                      visual as *mut _,
                                                      ffi::AllocNone);
            xconn.check_errors().expect("Failed to call XCreateColormap");
            cmap
        };

        let context = Context {
            xconn: Arc::clone(&xconn),
            context,
            colormap,
        };

        Ok(context)
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
