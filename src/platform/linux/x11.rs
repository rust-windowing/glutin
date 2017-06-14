pub use winit::os::unix::x11::{XError, XNotSupported, XConnection};

use std::{mem, ptr, fmt, error};
use std::sync::Arc;

use winit;
use winit::os::unix::{WindowExt, get_x11_xconnection};

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

enum GlContext {
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
        window: &winit::Window,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError>
    {
        let display = match get_x11_xconnection() {
            Some(display) => display,
            None => return Err(CreationError::NoBackendAvailable(Box::new(NoX11Connection))),
        };

        // Get the screen_id for the current window.
        //
        // TODO: re-add this when the `get_monitor_id` method is added.
        // let screen_id = match window.get_monitor_id().get_native_identifier() {
        //     winit::NativeMonitorId::Numeric(monitor) => monitor as i32,
        //     _ => unreachable!(),
        // };
        let screen_id = unsafe { (display.xlib.XDefaultScreen)(display.display) };

        // start the context building process
        enum Prototype<'a> {
            Glx(::api::glx::ContextPrototype<'a>),
            Egl(::api::egl::ContextPrototype<'a>),
        }

        let builder_clone_opengl_glx = gl_attr.clone().map_sharing(|_| unimplemented!());      // FIXME:
        let builder_clone_opengl_egl = gl_attr.clone().map_sharing(|_| unimplemented!());      // FIXME:
        let backend = GlxOrEgl::new();
        let context = match gl_attr.version {
            GlRequest::Latest |
            GlRequest::Specific(Api::OpenGl, _) |
            GlRequest::GlThenGles { .. } => {
                // GLX should be preferred over EGL, otherwise crashes may occur
                // on X11 – issue #314
                if let Some(ref glx) = backend.glx {
                    Prototype::Glx(try!(GlxContext::new(
                        glx.clone(),
                        &display.xlib,
                        pf_reqs,
                        &builder_clone_opengl_glx,
                        display.display,
                        screen_id,
                        gl_attr.transparent,
                    )))
                } else if let Some(ref egl) = backend.egl {
                    let native_display = egl::NativeDisplay::X11(Some(display.display as *const _));
                    Prototype::Egl(try!(EglContext::new(
                        egl.clone(),
                        pf_reqs,
                        &builder_clone_opengl_egl,
                        native_display,
                    )))
                } else {
                    return Err(CreationError::NotSupported);
                }
            },
            GlRequest::Specific(Api::OpenGlEs, _) => {
                if let Some(ref egl) = backend.egl {
                    Prototype::Egl(try!(EglContext::new(
                        egl.clone(),
                        pf_reqs,
                        &builder_clone_opengl_egl,
                        egl::NativeDisplay::X11(Some(display.display as *const _)),
                    )))
                } else {
                    return Err(CreationError::NotSupported);
                }
            },
            GlRequest::Specific(_, _) => {
                return Err(CreationError::NotSupported);
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

        let xlib_window = window.get_xlib_window().unwrap();
        // finish creating the OpenGL context
        let context = match context {
            Prototype::Glx(ctxt) => {
                GlContext::Glx(try!(ctxt.finish(xlib_window as _)))
            },
            Prototype::Egl(ctxt) => {
                GlContext::Egl(try!(ctxt.finish(xlib_window)))
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

        Ok(Context {
            display: display.clone(),
            context: context,
            colormap: cmap,
        })
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
}
