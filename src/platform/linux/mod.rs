#![cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]

use {Api, ContextError, CreationError, GlAttributes, PixelFormat, PixelFormatRequirements};

use api::osmesa::OsMesaContext;
use wayland_client;
use winit;

mod wayland;
mod x11;

pub enum Context {
    X(x11::Context),
    Wayland(wayland::Context)
}

impl Context {
    #[inline]
    pub fn new(
        window_builder: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
    ) -> Result<(winit::Window, Self), CreationError>
    {
        // winit allows use of XWayland, in which case will use an X11 backend
        // even if a wayland connection is available
        let use_wayland = winit::os::unix::get_x11_xconnection().is_none() &&
             wayland_client::default_connect().is_ok();

        if use_wayland {
            if let Some(&Context::X(_)) = gl_attr.sharing {
                let msg = "Cannot share a wayland context with an X11 context";
                return Err(CreationError::PlatformSpecific(msg.into()));
            }
            let gl_attr = gl_attr.clone().map_sharing(|ctxt| match ctxt {
                &Context::X(_) => unreachable!(),
                &Context::Wayland(ref ctxt) => ctxt,
            });
            wayland::Context::new(window_builder, events_loop, pf_reqs, &gl_attr)
                .map(|(window, context)| (window, Context::Wayland(context)))
        }
        else {
            if let Some(&Context::Wayland(_)) = gl_attr.sharing {
                let msg = "Cannot share a X11 context with an wayland context";
                return Err(CreationError::PlatformSpecific(msg.into()));
            }
            let gl_attr = gl_attr.clone().map_sharing(|ctxt| match ctxt {
                &Context::Wayland(_) => unreachable!(),
                &Context::X(ref ctxt) => ctxt,
            });
            x11::Context::new(window_builder, events_loop, pf_reqs, &gl_attr)
                .map(|(window, context)| (window, Context::X(context)))
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        match *self {
            Context::X(ref _ctxt) => (),
            Context::Wayland(ref ctxt) => ctxt.resize(width, height),
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        match *self {
            Context::X(ref ctxt) => ctxt.make_current(),
            Context::Wayland(ref ctxt) => ctxt.make_current()
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        match *self {
            Context::X(ref ctxt) => ctxt.is_current(),
            Context::Wayland(ref ctxt) => ctxt.is_current()
        }
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        match *self {
            Context::X(ref ctxt) => ctxt.get_proc_address(addr),
            Context::Wayland(ref ctxt) => ctxt.get_proc_address(addr)
        }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        match *self {
            Context::X(ref ctxt) => ctxt.swap_buffers(),
            Context::Wayland(ref ctxt) => ctxt.swap_buffers()
        }
    }

    #[inline]
    pub fn get_api(&self) -> ::Api {
        match *self {
            Context::X(ref ctxt) => ctxt.get_api(),
            Context::Wayland(ref ctxt) => ctxt.get_api()
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        match *self {
            Context::X(ref ctxt) => ctxt.get_pixel_format(),
            Context::Wayland(ref ctxt) => ctxt.get_pixel_format()
        }
    }
}

#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

pub struct HeadlessContext(OsMesaContext);

impl HeadlessContext {
    fn from(mesa: OsMesaContext) -> Self {
        HeadlessContext(mesa)
    }
}

impl HeadlessContext {
    pub fn new(dimensions: (u32, u32), pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&HeadlessContext>,
               _: &PlatformSpecificHeadlessBuilderAttributes)
               -> Result<HeadlessContext, CreationError>
    {
        let opengl = opengl.clone().map_sharing(|c| &c.0);

        OsMesaContext::new(dimensions, pf_reqs, &opengl).map(HeadlessContext::from)
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.0.make_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.0.get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        self.0.swap_buffers()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.0.get_api()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.0.get_pixel_format()
    }
}
