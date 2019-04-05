use crate::api::egl::{Context as EglContext, NativeDisplay, EGL};
use crate::api::glx::{Context as GlxContext, GLX};
use crate::{
    Api, ContextError, CreationError, GlAttributes, GlRequest, PixelFormat,
    PixelFormatRequirements,
};

use glutin_glx_sys as ffi;
use winit;
use winit::dpi;
pub use winit::os::unix::x11::{XConnection, XError, XNotSupported};
use winit::os::unix::{EventsLoopExt, WindowBuilderExt, WindowExt};

use std::ops::{Deref, DerefMut};
use std::os::raw;
use std::sync::Arc;

#[derive(Debug)]
struct NoX11Connection;

impl std::error::Error for NoX11Connection {
    fn description(&self) -> &str {
        "failed to get x11 connection"
    }
}

impl std::fmt::Display for NoX11Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(std::error::Error::description(self))
    }
}

#[derive(Debug)]
pub enum X11Context {
    Glx(GlxContext),
    Egl(EglContext),
}

#[derive(Debug)]
pub struct ContextInner {
    xconn: Arc<XConnection>,
    context: X11Context,
}

enum Prototype<'a> {
    Glx(crate::api::glx::ContextPrototype<'a>),
    Egl(crate::api::egl::ContextPrototype<'a>),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Context {
    Headless(ContextInner, winit::Window),
    Windowed(ContextInner),
}

impl Deref for Context {
    type Target = ContextInner;

    fn deref(&self) -> &Self::Target {
        match self {
            Context::Headless(ctx, _) => ctx,
            Context::Windowed(ctx) => ctx,
        }
    }
}

impl DerefMut for Context {
    fn deref_mut(&mut self) -> &mut ContextInner {
        match self {
            Context::Headless(ctx, _) => ctx,
            Context::Windowed(ctx) => ctx,
        }
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    #[inline]
    pub fn new_headless(
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: Option<dpi::PhysicalSize>,
    ) -> Result<Self, CreationError> {
        if let Some(size) = size {
            let xconn = match el.get_xlib_xconnection() {
                Some(xconn) => xconn,
                None => {
                    return Err(CreationError::NoBackendAvailable(Box::new(
                        NoX11Connection,
                    )));
                }
            };

            // Get the screen_id for the window being built.
            let screen_id =
                unsafe { (xconn.xlib.XDefaultScreen)(xconn.display) };

            let mut builder_glx_u = None;
            let mut builder_egl_u = None;

            // start the context building process
            let context = Self::new_first_stage(
                &xconn,
                pf_reqs,
                gl_attr,
                screen_id,
                &mut builder_glx_u,
                &mut builder_egl_u,
                true,
            )?;

            // finish creating the OpenGL context
            let context = match context {
                // Prototype::Glx(ctx) =>
                // X11Context::Glx(ctx.finish_pbuffer(xwin)?),
                Prototype::Egl(ctx) => {
                    X11Context::Egl(ctx.finish_pbuffer(size)?)
                }
                _ => unimplemented!(),
            };

            let context = Context::Windowed(ContextInner {
                xconn: Arc::clone(&xconn),
                context,
            });

            Ok(context)
        } else {
            // Surfaceless
            unimplemented!()
        }
    }

    #[inline]
    fn new_first_stage<'a>(
        xconn: &Arc<XConnection>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &'a GlAttributes<&'a Context>,
        screen_id: raw::c_int,
        builder_glx_u: &'a mut Option<GlAttributes<&'a GlxContext>>,
        builder_egl_u: &'a mut Option<GlAttributes<&'a EglContext>>,
        pbuffer_bit: bool,
    ) -> Result<Prototype<'a>, CreationError> {
        let builder = gl_attr.clone();

        Ok(match gl_attr.version {
            GlRequest::Latest
            | GlRequest::Specific(Api::OpenGl, _)
            | GlRequest::GlThenGles { .. } => {
                // GLX should be preferred over EGL, otherwise crashes may occur
                // on X11 – issue #314
                if let Some(_) = *GLX {
                    *builder_glx_u =
                        Some(builder.map_sharing(|c| match c.context {
                            X11Context::Glx(ref c) => c,
                            _ => panic!(),
                        }));
                    Prototype::Glx(GlxContext::new(
                        Arc::clone(&xconn),
                        pf_reqs,
                        builder_glx_u.as_ref().unwrap(),
                        screen_id,
                    )?)
                } else if let Some(_) = *EGL {
                    *builder_egl_u =
                        Some(builder.map_sharing(|c| match c.context {
                            X11Context::Egl(ref c) => c,
                            _ => panic!(),
                        }));
                    let native_display =
                        NativeDisplay::X11(Some(xconn.display as *const _));
                    Prototype::Egl(EglContext::new(
                        pf_reqs,
                        builder_egl_u.as_ref().unwrap(),
                        native_display,
                        pbuffer_bit,
                    )?)
                } else {
                    return Err(CreationError::NotSupported(
                        "both libglx and libEGL not present",
                    ));
                }
            }
            GlRequest::Specific(Api::OpenGlEs, _) => {
                if let Some(_) = *EGL {
                    *builder_egl_u =
                        Some(builder.map_sharing(|c| match c.context {
                            X11Context::Egl(ref c) => c,
                            _ => panic!(),
                        }));
                    Prototype::Egl(EglContext::new(
                        pf_reqs,
                        builder_egl_u.as_ref().unwrap(),
                        NativeDisplay::X11(Some(xconn.display as *const _)),
                        pbuffer_bit,
                    )?)
                } else {
                    return Err(CreationError::NotSupported(
                        "libEGL not present",
                    ));
                }
            }
            GlRequest::Specific(_, _) => {
                return Err(CreationError::NotSupported(
                    "requested specific without gl or gles",
                ));
            }
        })
    }

    #[inline]
    pub fn new(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError> {
        let xconn = match el.get_xlib_xconnection() {
            Some(xconn) => xconn,
            None => {
                return Err(CreationError::NoBackendAvailable(Box::new(
                    NoX11Connection,
                )));
            }
        };

        // Get the screen_id for the window being built.
        let screen_id = unsafe { (xconn.xlib.XDefaultScreen)(xconn.display) };

        let mut builder_glx_u = None;
        let mut builder_egl_u = None;

        // start the context building process
        let context = Self::new_first_stage(
            &xconn,
            pf_reqs,
            gl_attr,
            screen_id,
            &mut builder_glx_u,
            &mut builder_egl_u,
            false,
        )?;

        // getting the `visual_infos` (a struct that contains information about
        // the visual to use)
        let visual_infos = match context {
            Prototype::Glx(ref p) => p.get_visual_infos().clone(),
            Prototype::Egl(ref p) => {
                let mut template: ffi::XVisualInfo =
                    unsafe { std::mem::zeroed() };
                template.visualid = p.get_native_visual_id() as ffi::VisualID;

                let mut num_visuals = 0;
                let vi = unsafe {
                    (xconn.xlib.XGetVisualInfo)(
                        xconn.display,
                        ffi::VisualIDMask,
                        &mut template,
                        &mut num_visuals,
                    )
                };
                xconn
                    .check_errors()
                    .expect("Failed to call `XGetVisualInfo`");
                assert!(!vi.is_null());
                assert!(num_visuals == 1);

                let vi_copy = unsafe { std::ptr::read(vi as *const _) };
                unsafe {
                    (xconn.xlib.XFree)(vi as *mut _);
                }
                vi_copy
            }
        };

        let win = wb
            .with_x11_visual(&visual_infos as *const _)
            .with_x11_screen(screen_id)
            .build(el)?;

        let xwin = win.get_xlib_window().unwrap();
        // finish creating the OpenGL context
        let context = match context {
            Prototype::Glx(ctx) => X11Context::Glx(ctx.finish(xwin)?),
            Prototype::Egl(ctx) => X11Context::Egl(ctx.finish(xwin as _)?),
        };

        let context = Context::Windowed(ContextInner {
            xconn: Arc::clone(&xconn),
            context,
        });

        Ok((win, context))
    }

    #[inline]
    pub fn new_raw_context(
        xconn: Arc<XConnection>,
        xwin: raw::c_ulong,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        let attrs = unsafe {
            let mut attrs = ::std::mem::uninitialized();
            (xconn.xlib.XGetWindowAttributes)(xconn.display, xwin, &mut attrs);
            attrs
        };

        // Not particularly efficient, but it's the only method I can find.
        let mut screen_id = 0;
        unsafe {
            while attrs.screen
                != (xconn.xlib.XScreenOfDisplay)(xconn.display, screen_id)
            {
                screen_id += 1;
            }
        }

        let attrs = {
            let mut attrs = unsafe { std::mem::uninitialized() };
            unsafe {
                (xconn.xlib.XGetWindowAttributes)(
                    xconn.display,
                    xwin,
                    &mut attrs,
                );
            }
            attrs
        };

        let visual_xid =
            unsafe { (xconn.xlib.XVisualIDFromVisual)(attrs.visual) };
        let mut pf_reqs = pf_reqs.clone();
        pf_reqs.x11_visual_xid = Some(visual_xid);
        pf_reqs.depth_bits = Some(attrs.depth as _);

        let mut builder_glx_u = None;
        let mut builder_egl_u = None;

        // start the context building process
        let context = Self::new_first_stage(
            &xconn,
            &pf_reqs,
            gl_attr,
            screen_id,
            &mut builder_glx_u,
            &mut builder_egl_u,
            false,
        )?;

        // finish creating the OpenGL context
        let context = match context {
            Prototype::Glx(ctx) => X11Context::Glx(ctx.finish(xwin)?),
            Prototype::Egl(ctx) => X11Context::Egl(ctx.finish(xwin as _)?),
        };

        let context = Context::Windowed(ContextInner {
            xconn: Arc::clone(&xconn),
            context,
        });

        Ok(context)
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self.context {
            X11Context::Glx(ref ctx) => ctx.make_current(),
            X11Context::Egl(ref ctx) => ctx.make_current(),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match self.context {
            X11Context::Glx(ref ctx) => ctx.make_not_current(),
            X11Context::Egl(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match self.context {
            X11Context::Glx(ref ctx) => ctx.is_current(),
            X11Context::Egl(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match self.context {
            X11Context::Glx(ref ctx) => ctx.get_api(),
            X11Context::Egl(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> &X11Context {
        &self.context
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match self.context {
            X11Context::Egl(ref ctx) => Some(ctx.get_egl_display()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match self.context {
            X11Context::Glx(ref ctx) => ctx.get_proc_address(addr),
            X11Context::Egl(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match self.context {
            X11Context::Glx(ref ctx) => ctx.swap_buffers(),
            X11Context::Egl(ref ctx) => ctx.swap_buffers(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match self.context {
            X11Context::Glx(ref ctx) => ctx.get_pixel_format(),
            X11Context::Egl(ref ctx) => ctx.get_pixel_format(),
        }
    }
}
