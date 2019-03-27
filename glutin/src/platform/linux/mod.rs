#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod wayland;
mod x11;

use self::x11::X11Context;
use crate::api::osmesa;
use crate::{
    Api, ContextCurrentState, ContextError, CreationError, GlAttributes,
    NotCurrentContext, PixelFormat, PixelFormatRequirements,
    PossiblyCurrentContext,
};

use winit::dpi;
use winit::os::unix::EventsLoopExt;

use std::marker::PhantomData;
use std::os::raw;
use std::sync::Arc;

/// Context handles available on Unix-like platforms.
#[derive(Clone, Debug)]
pub enum RawHandle {
    Glx(glutin_glx_sys::GLXContext),
    Egl(glutin_egl_sys::EGLContext),
}

#[derive(Debug)]
pub enum ContextType {
    X11,
    Wayland,
    OsMesa,
}

#[derive(Debug)]
pub enum Context<T: ContextCurrentState> {
    WindowedX11(x11::Context<T>),
    HeadlessX11(x11::Context<T>, winit::Window),
    WindowedWayland(wayland::Context<T>),
    HeadlessWayland(wayland::Context<T>, winit::Window),
    OsMesa(osmesa::OsMesaContext<T>),
}

impl<T: ContextCurrentState> Context<T> {
    fn is_compatible(
        c: &Option<&Context<T>>,
        ct: ContextType,
    ) -> Result<(), CreationError> {
        if let Some(c) = *c {
            match ct {
                ContextType::OsMesa => match *c {
                    Context::OsMesa(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share an OSMesa context with a non-OSMesa context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
                ContextType::X11 => match *c {
                    Context::WindowedX11(_) | Context::HeadlessX11(_, _) => {
                        Ok(())
                    }
                    _ => {
                        let msg = "Cannot share an X11 context with a non-X11 context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
                ContextType::Wayland => match *c {
                    Context::WindowedWayland(_)
                    | Context::HeadlessWayland(_, _) => Ok(()),
                    _ => {
                        let msg = "Cannot share a Wayland context with a non-Wayland context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn new_windowed(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context<T>>,
    ) -> Result<(winit::Window, Context<NotCurrentContext>), CreationError>
    {
        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;

            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::WindowedWayland(ref ctx)
                | Context::HeadlessWayland(ref ctx, _) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new(wb, el, pf_reqs, &gl_attr)
                .map(|(win, context)| (win, Context::WindowedWayland(context)))
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::WindowedX11(ref ctx)
                | Context::HeadlessX11(ref ctx, _) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new(wb, el, pf_reqs, &gl_attr)
                .map(|(win, context)| (win, Context::WindowedX11(context)))
        }
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

        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::WindowedWayland(ref ctx)
                | Context::HeadlessWayland(ref ctx, _) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new(wb, &el, pf_reqs, &gl_attr)
                .map(|(win, ctx)| Context::HeadlessWayland(ctx, win))
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::WindowedX11(ref ctx)
                | Context::HeadlessX11(ref ctx, _) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new(wb, &el, pf_reqs, &gl_attr)
                .map(|(win, ctx)| Context::HeadlessX11(ctx, win))
        }
    }

    fn state_sub<T2, E, FX, FW, FO>(
        self,
        fx: FX,
        fw: FW,
        fo: FO,
    ) -> Result<Context<T2>, (Self, E)>
    where
        T2: ContextCurrentState,
        FX: FnOnce(
            x11::Context<T>,
        ) -> Result<x11::Context<T2>, (x11::Context<T>, E)>,
        FW: FnOnce(
            wayland::Context<T>,
        )
            -> Result<wayland::Context<T2>, (wayland::Context<T>, E)>,
        FO: FnOnce(
            osmesa::OsMesaContext<T>,
        ) -> Result<
            osmesa::OsMesaContext<T2>,
            (osmesa::OsMesaContext<T>, E),
        >,
    {
        match self {
            Context::WindowedX11(ctx) => match fx(ctx) {
                Ok(ctx) => Ok(Context::WindowedX11(ctx)),
                Err((ctx, e)) => Err((Context::WindowedX11(ctx), e)),
            },
            Context::HeadlessX11(ctx, w) => match fx(ctx) {
                Ok(ctx) => Ok(Context::HeadlessX11(ctx, w)),
                Err((ctx, e)) => Err((Context::HeadlessX11(ctx, w), e)),
            },
            Context::WindowedWayland(ctx) => match fw(ctx) {
                Ok(ctx) => Ok(Context::WindowedWayland(ctx)),
                Err((ctx, e)) => Err((Context::WindowedWayland(ctx), e)),
            },
            Context::HeadlessWayland(ctx, w) => match fw(ctx) {
                Ok(ctx) => Ok(Context::HeadlessWayland(ctx, w)),
                Err((ctx, e)) => Err((Context::HeadlessWayland(ctx, w), e)),
            },
            Context::OsMesa(ctx) => match fo(ctx) {
                Ok(ctx) => Ok(Context::OsMesa(ctx)),
                Err((ctx, e)) => Err((Context::OsMesa(ctx), e)),
            },
        }
    }

    #[inline]
    pub unsafe fn make_current(
        self,
    ) -> Result<Context<PossiblyCurrentContext>, (Self, ContextError)> {
        self.state_sub(
            |ctx| ctx.make_current(),
            |ctx| ctx.make_current(),
            |ctx| ctx.make_current(),
        )
    }

    #[inline]
    pub unsafe fn make_not_current(
        self,
    ) -> Result<Context<NotCurrentContext>, (Self, ContextError)> {
        self.state_sub(
            |ctx| ctx.make_not_current(),
            |ctx| ctx.make_not_current(),
            |ctx| ctx.make_not_current(),
        )
    }

    #[inline]
    pub unsafe fn treat_as_not_current(self) -> Context<NotCurrentContext> {
        self.state_sub::<_, (), _, _, _>(
            |ctx| Ok(ctx.treat_as_not_current()),
            |ctx| Ok(ctx.treat_as_not_current()),
            |ctx| Ok(ctx.treat_as_not_current()),
        )
        .unwrap()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(ref ctx, _) => ctx.is_current(),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(ref ctx, _) => ctx.is_current(),
            Context::OsMesa(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(ref ctx, _) => ctx.get_api(),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(ref ctx, _) => ctx.get_api(),
            Context::OsMesa(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(ref ctx, _) => match *ctx.raw_handle() {
                X11Context::Glx(ref ctx) => RawHandle::Glx(ctx.raw_handle()),
                X11Context::Egl(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
            },
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(ref ctx, _) => {
                RawHandle::Egl(ctx.raw_handle())
            }
            Context::OsMesa(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(ref ctx, _) => ctx.get_egl_display(),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(ref ctx, _) => ctx.get_egl_display(),
            _ => None,
        }
    }
}

impl Context<PossiblyCurrentContext> {
    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        match *self {
            Context::WindowedX11(_) => (),
            Context::WindowedWayland(ref ctx) => ctx.resize(width, height),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(ref ctx, _) => ctx.get_proc_address(addr),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(ref ctx, _) => {
                ctx.get_proc_address(addr)
            }
            Context::OsMesa(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::WindowedX11(ref ctx) => ctx.swap_buffers(),
            Context::WindowedWayland(ref ctx) => ctx.swap_buffers(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::WindowedX11(ref ctx) => ctx.get_pixel_format(),
            Context::WindowedWayland(ref ctx) => ctx.get_pixel_format(),
            _ => unreachable!(),
        }
    }
}

pub trait OsMesaContextExt {
    fn build_osmesa(
        self,
        dims: dpi::PhysicalSize,
    ) -> Result<crate::Context<NotCurrentContext>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextCurrentState> OsMesaContextExt
    for crate::ContextBuilder<'a, T>
{
    /// Builds the given OsMesa context.
    ///
    /// Errors can occur if the OpenGL context could not be created. This
    /// generally happens because the underlying platform doesn't support a
    /// requested feature.
    #[inline]
    fn build_osmesa(
        self,
        dims: dpi::PhysicalSize,
    ) -> Result<crate::Context<NotCurrentContext>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::is_compatible(&gl_attr.sharing, ContextType::OsMesa)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
            Context::OsMesa(ref ctx) => ctx,
            _ => unreachable!(),
        });
        osmesa::OsMesaContext::new(&pf_reqs, &gl_attr, dims)
            .map(|context| Context::OsMesa(context))
            .map(|context| crate::Context {
                context,
                phantom: PhantomData,
            })
    }
}

pub trait RawContextExt {
    /// Creates a raw context on the provided surface.
    ///
    /// Unsafe behaviour might happen if you:
    ///   - Provide us with invalid parameters.
    ///   - The surface/display_ptr is destroyed before the context
    unsafe fn build_raw_wayland_context(
        self,
        display_ptr: *const wayland::wl_display,
        surface: *mut raw::c_void,
        width: u32,
        height: u32,
    ) -> Result<crate::RawContext<NotCurrentContext>, CreationError>
    where
        Self: Sized;

    /// Creates a raw context on the provided window.
    ///
    /// Unsafe behaviour might happen if you:
    ///   - Provide us with invalid parameters.
    ///   - The xconn/xwin is destroyed before the context
    unsafe fn build_raw_x11_context(
        self,
        xconn: Arc<x11::XConnection>,
        xwin: raw::c_ulong,
    ) -> Result<crate::RawContext<NotCurrentContext>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextCurrentState> RawContextExt
    for crate::ContextBuilder<'a, T>
{
    #[inline]
    unsafe fn build_raw_wayland_context(
        self,
        display_ptr: *const wayland::wl_display,
        surface: *mut raw::c_void,
        width: u32,
        height: u32,
    ) -> Result<crate::RawContext<NotCurrentContext>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(ref ctx, _) => ctx,
            _ => unreachable!(),
        });
        wayland::Context::new_raw_context(
            display_ptr,
            surface,
            width,
            height,
            &pf_reqs,
            &gl_attr,
        )
        .map(|context| Context::WindowedWayland(context))
        .map(|context| crate::Context {
            context,
            phantom: PhantomData,
        })
        .map(|context| crate::RawContext {
            context,
            window: (),
        })
    }

    #[inline]
    unsafe fn build_raw_x11_context(
        self,
        xconn: Arc<x11::XConnection>,
        xwin: raw::c_ulong,
    ) -> Result<crate::RawContext<NotCurrentContext>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(ref ctx, _) => ctx,
            _ => unreachable!(),
        });
        x11::Context::new_raw_context(xconn, xwin, &pf_reqs, &gl_attr)
            .map(|context| Context::WindowedX11(context))
            .map(|context| crate::Context {
                context,
                phantom: PhantomData,
            })
            .map(|context| crate::RawContext {
                context,
                window: (),
            })
    }
}
