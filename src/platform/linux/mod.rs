#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]

use self::x11::X11Context;
use api::egl;
use api::glx;
use {
    ContextError, CreationError, GlAttributes, PixelFormat,
    PixelFormatRequirements,
};

use winit;
use winit::os::unix::EventsLoopExt;

mod wayland;
mod x11;
use api::osmesa;

use std::os::raw;

/// Context handles available on Unix-like platforms.
#[derive(Clone, Debug)]
pub enum RawHandle {
    Glx(glx::ffi::GLXContext),
    Egl(egl::ffi::EGLContext),
}

pub enum ContextType {
    X11,
    Wayland,
    OsMesa,
}

pub enum Context {
    WindowedX11(x11::Context),
    HeadlessX11(winit::Window, x11::Context),
    WindowedWayland(wayland::Context),
    HeadlessWayland(winit::Window, wayland::Context),
    OsMesa(osmesa::OsMesaContext),
}

impl Context {
    fn is_compatible(
        c: &Option<&Context>,
        ct: ContextType,
    ) -> Result<(), CreationError> {
        if let Some(c) = *c {
            match ct {
                ContextType::OsMesa => match *c {
                    Context::OsMesa(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share an OSMesa context with a non-OSMesa context";
                        return Err(CreationError::PlatformSpecific(msg.into()));
                    }
                },
                ContextType::X11 => match *c {
                    Context::WindowedX11(_) | Context::HeadlessX11(_, _) => {
                        Ok(())
                    }
                    _ => {
                        let msg = "Cannot share an X11 context with a non-X11 context";
                        return Err(CreationError::PlatformSpecific(msg.into()));
                    }
                },
                ContextType::Wayland => match *c {
                    Context::WindowedWayland(_)
                    | Context::HeadlessWayland(_, _) => Ok(()),
                    _ => {
                        let msg = "Cannot share a Wayland context with a non-Wayland context";
                        return Err(CreationError::PlatformSpecific(msg.into()));
                    }
                },
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn new(
        wb: winit::WindowBuilder,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError> {
        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;

            let gl_attr = gl_attr.clone().map_sharing(|ctx| match ctx {
                &Context::WindowedWayland(ref ctx)
                | &Context::HeadlessWayland(_, ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new(wb, el, pf_reqs, &gl_attr).map(
                |(window, context)| (window, Context::WindowedWayland(context)),
            )
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match ctx {
                &Context::WindowedX11(ref ctx)
                | &Context::HeadlessX11(_, ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new(wb, el, pf_reqs, &gl_attr).map(
                |(window, context)| (window, Context::WindowedX11(context)),
            )
        }
    }

    #[inline]
    pub fn new_context(
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        let wb = winit::WindowBuilder::new().with_visibility(false);

        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match ctx {
                &Context::WindowedWayland(ref ctx)
                | &Context::HeadlessWayland(_, ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new(wb, &el, pf_reqs, &gl_attr).map(
                |(window, context)| Context::HeadlessWayland(window, context),
            )
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match ctx {
                &Context::WindowedX11(ref ctx)
                | &Context::HeadlessX11(_, ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new(wb, &el, pf_reqs, &gl_attr)
                .map(|(window, context)| Context::HeadlessX11(window, context))
        }
    }

    #[inline]
    pub fn new_separated(
        window: &winit::Window,
        el: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;

            let gl_attr = gl_attr.clone().map_sharing(|ctx| match ctx {
                &Context::WindowedWayland(ref ctx)
                | &Context::HeadlessWayland(_, ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new_separated(window, el, pf_reqs, &gl_attr)
                .map(|context| Context::WindowedWayland(context))
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match ctx {
                &Context::WindowedX11(ref ctx)
                | &Context::HeadlessX11(_, ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new_separated(window, el, pf_reqs, &gl_attr)
                .map(|context| Context::WindowedX11(context))
        }
    }

    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        match *self {
            Context::WindowedX11(_) => (),
            Context::WindowedWayland(ref ctx) => ctx.resize(width, height),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(_, ref ctx) => ctx.make_current(),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(_, ref ctx) => ctx.make_current(),
            Context::OsMesa(ref ctx) => ctx.make_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(_, ref ctx) => ctx.is_current(),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(_, ref ctx) => ctx.is_current(),
            Context::OsMesa(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(_, ref ctx) => ctx.get_proc_address(addr),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(_, ref ctx) => {
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
    pub fn get_api(&self) -> ::Api {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(_, ref ctx) => ctx.get_api(),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(_, ref ctx) => ctx.get_api(),
            Context::OsMesa(ref ctx) => ctx.get_api(),
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

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(_, ref ctx) => match *ctx.raw_handle() {
                X11Context::Glx(ref ctx) => RawHandle::Glx(ctx.raw_handle()),
                X11Context::Egl(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
                X11Context::None => panic!(),
            },
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(_, ref ctx) => {
                RawHandle::Egl(ctx.raw_handle())
            }
            Context::OsMesa(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match *self {
            Context::WindowedX11(ref ctx)
            | Context::HeadlessX11(_, ref ctx) => ctx.get_egl_display(),
            Context::WindowedWayland(ref ctx)
            | Context::HeadlessWayland(_, ref ctx) => ctx.get_egl_display(),
            _ => None,
        }
    }

    #[inline]
    fn new_osmesa(
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<Self, CreationError> {
        Context::is_compatible(&gl_attr.sharing, ContextType::OsMesa)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match ctx {
            &Context::OsMesa(ref ctx) => ctx,
            _ => unreachable!(),
        });
        osmesa::OsMesaContext::new((1, 1), pf_reqs, &gl_attr)
            .map(|context| Context::OsMesa(context))
    }
}

pub trait OsMesaContextExt {
    fn new_osmesa(cb: crate::ContextBuilder) -> Result<Self, CreationError>
    where
        Self: Sized;
}

impl OsMesaContextExt for crate::Context {
    /// Builds the given OsMesa context.
    ///
    /// Errors can occur if the OpenGL context could not be created. This
    /// generally happens because the underlying platform doesn't support a
    /// requested feature.
    #[inline]
    fn new_osmesa(cb: crate::ContextBuilder) -> Result<Self, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = cb;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::new_osmesa(&pf_reqs, &gl_attr)
            .map(|context| crate::Context { context })
    }
}
