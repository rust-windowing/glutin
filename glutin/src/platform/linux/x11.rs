use crate::api::egl::{
    Context as EglContext, NativeDisplay, SurfaceType as EglSurfaceType, EGL,
};
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
pub enum Context {
    Surfaceless(ContextInner),
    PBuffer(ContextInner),
    Windowed(ContextInner),
}

impl Deref for Context {
    type Target = ContextInner;

    fn deref(&self) -> &Self::Target {
        match self {
            Context::Surfaceless(ctx) => ctx,
            Context::PBuffer(ctx) => ctx,
            Context::Windowed(ctx) => ctx,
        }
    }
}

impl DerefMut for Context {
    fn deref_mut(&mut self) -> &mut ContextInner {
        match self {
            Context::Surfaceless(ctx) => ctx,
            Context::PBuffer(ctx) => ctx,
            Context::Windowed(ctx) => ctx,
        }
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    fn try_then_fallback<F, T>(mut f: F) -> Result<T, CreationError>
    where
        F: FnMut(bool) -> Result<T, CreationError>,
    {
        match f(false) {
            Ok(ok) => Ok(ok),
            Err(err1) => match f(true) {
                Ok(ok) => Ok(ok),
                Err(err2) => Err(err1.append(err2)),
            },
        }
    }

    #[inline]
    pub fn new_headless(
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: Option<dpi::PhysicalSize>,
    ) -> Result<Self, CreationError> {
        Self::try_then_fallback(|fallback| {
            Self::new_headless_impl(
                el,
                pf_reqs,
                gl_attr,
                size.clone(),
                fallback,
            )
        })
    }

    fn new_headless_impl(
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: Option<dpi::PhysicalSize>,
        fallback: bool,
    ) -> Result<Self, CreationError> {
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
        if let Some(size) = size {
            let context = Self::new_first_stage(
                &xconn,
                pf_reqs,
                gl_attr,
                screen_id,
                &mut builder_glx_u,
                &mut builder_egl_u,
                EglSurfaceType::PBuffer,
                fallback,
                fallback,
            )?;

            // finish creating the OpenGL context
            let context = match context {
                Prototype::Glx(ctx) => {
                    X11Context::Glx(ctx.finish_pbuffer(size)?)
                }
                Prototype::Egl(ctx) => {
                    X11Context::Egl(ctx.finish_pbuffer(size)?)
                }
            };

            let context = Context::PBuffer(ContextInner {
                xconn: Arc::clone(&xconn),
                context,
            });

            Ok(context)
        } else {
            // Surfaceless
            let context = Self::new_first_stage(
                &xconn,
                pf_reqs,
                gl_attr,
                screen_id,
                &mut builder_glx_u,
                &mut builder_egl_u,
                EglSurfaceType::Surfaceless,
                !fallback,
                fallback,
            )?;

            // finish creating the OpenGL context
            let context = match context {
                // TODO: glx impl
                //
                // According to GLX_EXT_no_config_context
                // > 2) Are no-config contexts constrained to those GL & ES
                // > implementations which can support them?
                // >
                // > RESOLVED: Yes. ES2 + OES_surfaceless_context, ES 3.0, and
                // > GL 3.0 all support binding a context without a drawable.
                // > This implies that they don't need to know drawable
                // > attributes at context creation time.
                // >
                // > In principle, equivalent functionality could be possible
                // > with ES 1.x + OES_surfaceless_context. This extension
                // > makes no promises about that. An implementation wishing to
                // > reliably support this combination, or a similarly
                // > permissive combination for GL < 3.0, should indicate so
                // > with an additional GLX extension.

                // Prototype::Glx(ctx) =>
                // X11Context::Glx(ctx.finish_surfaceless(xwin)?),
                Prototype::Egl(ctx) => {
                    X11Context::Egl(ctx.finish_surfaceless()?)
                }
                _ => unimplemented!(),
            };

            let context = Context::Surfaceless(ContextInner {
                xconn: Arc::clone(&xconn),
                context,
            });

            Ok(context)
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
        surface_type: EglSurfaceType,
        prefer_egl: bool,
        force_prefer_unless_only: bool,
    ) -> Result<Prototype<'a>, CreationError> {
        Ok(match gl_attr.version {
            GlRequest::Latest
            | GlRequest::Specific(Api::OpenGl, _)
            | GlRequest::GlThenGles { .. } => {
                // GLX should be preferred over EGL, otherwise crashes may occur
                // on X11 – issue #314
                //
                // However, with surfaceless, GLX isn't really there, so we
                // should prefer EGL.
                let glx = |builder_u: &'a mut Option<_>| {
                    let builder = gl_attr.clone();
                    *builder_u =
                        Some(builder.map_sharing(|c| match c.context {
                            X11Context::Glx(ref c) => c,
                            _ => panic!(),
                        }));
                    Ok(Prototype::Glx(GlxContext::new(
                        Arc::clone(&xconn),
                        pf_reqs,
                        builder_u.as_ref().unwrap(),
                        screen_id,
                    )?))
                };

                let egl = |builder_u: &'a mut Option<_>| {
                    let builder = gl_attr.clone();
                    *builder_u =
                        Some(builder.map_sharing(|c| match c.context {
                            X11Context::Egl(ref c) => c,
                            _ => panic!(),
                        }));
                    let native_display =
                        NativeDisplay::X11(Some(xconn.display as *const _));
                    Ok(Prototype::Egl(EglContext::new(
                        pf_reqs,
                        builder_u.as_ref().unwrap(),
                        native_display,
                        surface_type,
                    )?))
                };

                // force_prefer_unless_only does what it says on the tin, it
                // forces only the prefered method to happen unless it's the
                // only method available.
                //
                // Users of this function should first call with `prefer_egl`
                // as `<status of their choice>`, with
                // `force_prefer_unless_only` as `false`.
                //
                // Then, if those users want to fallback and try the other
                // method, they should call us with `prefer_egl` equal to
                // `!<status of their choice>` and `force_prefer_unless_only`
                // as true.
                //
                // That way, they'll try their fallback if available, unless
                // it was their only option and they have already tried it.
                if !force_prefer_unless_only {
                    match (&*GLX, &*EGL, prefer_egl) {
                        (Some(_), _, false) => return glx(builder_glx_u),
                        (_, Some(_), true) => return egl(builder_egl_u),
                        _ => (),
                    }

                    match (&*GLX, &*EGL, prefer_egl) {
                        (_, Some(_), false) => return egl(builder_egl_u),
                        (Some(_), _, true) => return glx(builder_glx_u),
                        _ => (),
                    }

                    return Err(CreationError::NotSupported(
                        "both libGL and libEGL are not present".to_string(),
                    ));
                } else {
                    match (&*GLX, &*EGL, prefer_egl) {
                        (Some(_), Some(_), true) => return egl(builder_egl_u),
                        (Some(_), Some(_), false) => return glx(builder_glx_u),
                        _ => (),
                    }

                    return Err(CreationError::NotSupported(
                        "lacking either libGL or libEGL so could not fallback to other".to_string(),
                    ));
                }
            }
            GlRequest::Specific(Api::OpenGlEs, _) => {
                if let Some(_) = *EGL {
                    let builder = gl_attr.clone();
                    *builder_egl_u =
                        Some(builder.map_sharing(|c| match c.context {
                            X11Context::Egl(ref c) => c,
                            _ => panic!(),
                        }));
                    Prototype::Egl(EglContext::new(
                        pf_reqs,
                        builder_egl_u.as_ref().unwrap(),
                        NativeDisplay::X11(Some(xconn.display as *const _)),
                        surface_type,
                    )?)
                } else {
                    return Err(CreationError::NotSupported(
                        "libEGL not present".to_string(),
                    ));
                }
            }
            GlRequest::Specific(_, _) => {
                return Err(CreationError::NotSupported(
                    "requested specific without gl or gles".to_string(),
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
        Self::try_then_fallback(|fallback| {
            Self::new_impl(wb.clone(), el, pf_reqs, gl_attr, fallback)
        })
    }

    fn new_impl(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        fallback: bool,
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
            EglSurfaceType::Window,
            fallback,
            fallback,
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
        Self::try_then_fallback(|fallback| {
            Self::new_raw_context_impl(&xconn, xwin, pf_reqs, gl_attr, fallback)
        })
    }

    fn new_raw_context_impl(
        xconn: &Arc<XConnection>,
        xwin: raw::c_ulong,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        fallback: bool,
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
            EglSurfaceType::Window,
            fallback,
            fallback,
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
