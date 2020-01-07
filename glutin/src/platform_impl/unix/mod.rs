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
    NotCurrent, PixelFormat, PixelFormatRequirements, Rect,
};
pub use x11::utils as x11_utils;

use crate::platform::unix::x11::XConnection;
use crate::platform::unix::EventLoopWindowTargetExtUnix;
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder};

use std::marker::PhantomData;
use std::os::raw;
use std::sync::Arc;

/// Context handles available on Unix-like platforms.
#[derive(Clone, Debug)]
pub enum RawHandle {
    /// Context handle for a glx context.
    Glx(glutin_glx_sys::GLXContext),
    /// Context handle for a egl context.
    Egl(glutin_egl_sys::EGLContext),
}

#[derive(Debug)]
pub enum ContextType {
    X11,
    Wayland,
    OsMesa,
}

#[derive(Debug)]
pub enum Context {
    X11(x11::Context),
    Wayland(wayland::Context),
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
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
                ContextType::X11 => match *c {
                    Context::X11(_) => Ok(()),
                    _ => {
                        let msg = "Cannot share an X11 context with a non-X11 context";
                        return Err(CreationError::PlatformSpecific(
                            msg.into(),
                        ));
                    }
                },
                ContextType::Wayland => match *c {
                    Context::Wayland(_) => Ok(()),
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
    pub fn new_windowed<T>(
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(Window, Self), CreationError> {
        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;

            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::Wayland(ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new(wb, el, pf_reqs, &gl_attr)
                .map(|(win, context)| (win, Context::Wayland(context)))
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::X11(ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new(wb, el, pf_reqs, &gl_attr)
                .map(|(win, context)| (win, Context::X11(context)))
        }
    }

    #[inline]
    pub fn new_headless<T>(
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: dpi::PhysicalSize<u32>,
    ) -> Result<Self, CreationError> {
        Self::new_headless_impl(el, pf_reqs, gl_attr, Some(size))
    }

    pub fn new_headless_impl<T>(
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: Option<dpi::PhysicalSize<u32>>,
    ) -> Result<Self, CreationError> {
        if el.is_wayland() {
            Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::Wayland(ref ctx) => ctx,
                _ => unreachable!(),
            });
            wayland::Context::new_headless(&el, pf_reqs, &gl_attr, size)
                .map(|ctx| Context::Wayland(ctx))
        } else {
            Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
            let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
                Context::X11(ref ctx) => ctx,
                _ => unreachable!(),
            });
            x11::Context::new_headless(&el, pf_reqs, &gl_attr, size)
                .map(|ctx| Context::X11(ctx))
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.make_current(),
            Context::Wayland(ref ctx) => ctx.make_current(),
            Context::OsMesa(ref ctx) => ctx.make_current(),
        }
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.make_not_current(),
            Context::Wayland(ref ctx) => ctx.make_not_current(),
            Context::OsMesa(ref ctx) => ctx.make_not_current(),
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::X11(ref ctx) => ctx.is_current(),
            Context::Wayland(ref ctx) => ctx.is_current(),
            Context::OsMesa(ref ctx) => ctx.is_current(),
        }
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        match *self {
            Context::X11(ref ctx) => ctx.get_api(),
            Context::Wayland(ref ctx) => ctx.get_api(),
            Context::OsMesa(ref ctx) => ctx.get_api(),
        }
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        match *self {
            Context::X11(ref ctx) => match *ctx.raw_handle() {
                X11Context::Glx(ref ctx) => RawHandle::Glx(ctx.raw_handle()),
                X11Context::Egl(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
            },
            Context::Wayland(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
            Context::OsMesa(ref ctx) => RawHandle::Egl(ctx.raw_handle()),
        }
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        match *self {
            Context::X11(ref ctx) => ctx.get_egl_display(),
            Context::Wayland(ref ctx) => ctx.get_egl_display(),
            _ => None,
        }
    }

    #[inline]
    pub fn resize(&self, width: u32, height: u32) {
        match *self {
            Context::X11(_) => (),
            Context::Wayland(ref ctx) => ctx.resize(width, height),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        match *self {
            Context::X11(ref ctx) => ctx.get_proc_address(addr),
            Context::Wayland(ref ctx) => ctx.get_proc_address(addr),
            Context::OsMesa(ref ctx) => ctx.get_proc_address(addr),
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.swap_buffers(),
            Context::Wayland(ref ctx) => ctx.swap_buffers(),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        match *self {
            Context::X11(ref ctx) => ctx.swap_buffers_with_damage(rects),
            Context::Wayland(ref ctx) => ctx.swap_buffers_with_damage(rects),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        match *self {
            Context::X11(ref ctx) => ctx.swap_buffers_with_damage_supported(),
            Context::Wayland(ref ctx) => {
                ctx.swap_buffers_with_damage_supported()
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::X11(ref ctx) => ctx.get_pixel_format(),
            Context::Wayland(ref ctx) => ctx.get_pixel_format(),
            _ => unreachable!(),
        }
    }
}

/// A unix-specific extension to the [`ContextBuilder`] which allows building
/// unix-specific headless contexts.
///
/// [`ContextBuilder`]: ../../struct.ContextBuilder.html
pub trait HeadlessContextExt {
    /// Builds an OsMesa context.
    ///
    /// Errors can occur if the OpenGL [`Context`] could not be created. This
    /// generally happens because the underlying platform doesn't support a
    /// requested feature.
    ///
    /// [`Context`]: struct.Context.html
    fn build_osmesa(
        self,
        size: dpi::PhysicalSize<u32>,
    ) -> Result<crate::Context<NotCurrent>, CreationError>
    where
        Self: Sized;

    /// Builds an EGL-surfaceless context.
    ///
    /// Errors can occur if the OpenGL [`Context`] could not be created. This
    /// generally happens because the underlying platform doesn't support a
    /// requested feature.
    ///
    /// [`Context`]: struct.Context.html
    fn build_surfaceless<TE>(
        self,
        el: &EventLoopWindowTarget<TE>,
    ) -> Result<crate::Context<NotCurrent>, CreationError>
    where
        Self: Sized;
}

impl<'a, T: ContextCurrentState> HeadlessContextExt
    for crate::ContextBuilder<'a, T>
{
    #[inline]
    fn build_osmesa(
        self,
        size: dpi::PhysicalSize<u32>,
    ) -> Result<crate::Context<NotCurrent>, CreationError>
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
        osmesa::OsMesaContext::new(&pf_reqs, &gl_attr, size)
            .map(|context| Context::OsMesa(context))
            .map(|context| crate::Context {
                context,
                phantom: PhantomData,
            })
    }

    #[inline]
    fn build_surfaceless<TE>(
        self,
        el: &EventLoopWindowTarget<TE>,
    ) -> Result<crate::Context<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::new_headless_impl(el, &pf_reqs, &gl_attr, None).map(
            |context| crate::Context {
                context,
                phantom: PhantomData,
            },
        )
    }
}

/// A unix-specific extension for the [`ContextBuilder`] which allows
/// assembling [`RawContext<T>`]s.
///
/// [`RawContext<T>`]: ../../type.RawContext.html
/// [`ContextBuilder`]: ../../struct.ContextBuilder.html
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
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized;

    /// Creates a raw context on the provided window.
    ///
    /// Unsafe behaviour might happen if you:
    ///   - Provide us with invalid parameters.
    ///   - The xwin is destroyed before the context
    unsafe fn build_raw_x11_context(
        self,
        xconn: Arc<XConnection>,
        xwin: raw::c_ulong,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
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
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::is_compatible(&gl_attr.sharing, ContextType::Wayland)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
            Context::Wayland(ref ctx) => ctx,
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
        .map(|context| Context::Wayland(context))
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
        xconn: Arc<XConnection>,
        xwin: raw::c_ulong,
    ) -> Result<crate::RawContext<NotCurrent>, CreationError>
    where
        Self: Sized,
    {
        let crate::ContextBuilder { pf_reqs, gl_attr } = self;
        let gl_attr = gl_attr.map_sharing(|ctx| &ctx.context);
        Context::is_compatible(&gl_attr.sharing, ContextType::X11)?;
        let gl_attr = gl_attr.clone().map_sharing(|ctx| match *ctx {
            Context::X11(ref ctx) => ctx,
            _ => unreachable!(),
        });
        x11::Context::new_raw_context(xconn, xwin, &pf_reqs, &gl_attr)
            .map(|context| Context::X11(context))
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
