use crate::api::egl;
use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::ContextBuilderWrapper;
use crate::platform_impl::RawHandle;
use crate::surface::{PBuffer, Pixmap, Rect, SurfaceTypeTrait, Window};
use crate::utils::NoPrint;

use glutin_egl_sys as ffi;
use glutin_interface::{NativeDisplay, NativePixmapSource, NativeWindowSource, NativeWindow, RawWindow};
use wayland_client::egl as wegl;
pub use wayland_client::sys::client::wl_display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};

use std::ffi::c_void;
use std::ops::Deref;
use std::os::raw;
use std::sync::Arc;

#[derive(Debug)]
pub struct Display(egl::Display);

impl Display {
    pub fn new<NDS: NativeDisplay>(nds: &NDS) -> Result<Self, Error> {
        egl::Display::new(nds).map(Display)
    }
}

#[derive(Debug)]
pub struct Config(egl::Config);

impl Config {
    pub fn new(
        disp: &Display,
        cb: ConfigBuilder,
    ) -> Result<(ConfigAttribs, Config), Error> {
        egl::Config::new(&disp.0, cb, |confs, _| Ok(confs[0]))
            .map(|(attribs, conf)| (attribs, Config(conf)))
    }
}

#[derive(Debug)]
pub struct Surface<T: SurfaceTypeTrait> {
    wsurface: Option<NoPrint<wegl::WlEglSurface>>,
    surface: egl::Surface<T>,
}

impl<T: SurfaceTypeTrait> Surface<T> {
    #[inline]
    pub fn is_current(&self) -> bool {
        self.surface.is_current()
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.surface.get_config().map_config(|conf| Config(conf))
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.surface.make_not_current()
    }
}

impl Surface<Window> {
    #[inline]
    pub unsafe fn new<NWS: NativeWindowSource>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nws: NWS,
    ) -> Result<(NWS::Window, Self), Error> {
        let win = nws.build_wayland()?;

        let (width, height): (u32, u32) = win.size().into();

        let surface = win.raw_window();
        let surface = match surface {
            RawWindow::Wayland{ wl_surface } => wl_surface,
            _ => {
                return Err(make_error!(ErrorType::NotSupported(
                    "Wayland surface not found".to_string(),
                )));
            }
        };

        let wsurface = unsafe {
            wegl::WlEglSurface::new_from_raw(
                surface as *mut _,
                width as i32,
                height as i32,
            )
        };

        egl::Surface::<Window>::new(
            &disp.0,
            conf.map_config(|conf| &conf.0),
            wsurface.ptr() as *const _,
        )
        .map(|surface| (win, Surface { wsurface: Some(NoPrint(wsurface)), surface }))
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        let (width, height): (u32, u32) = size.into();
        self.wsurface.as_ref().unwrap().resize(width as i32, height as i32, 0, 0)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.surface.swap_buffers()
    }

    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[Rect]) -> Result<(), Error> {
        self.surface.swap_buffers_with_damage(rects)
    }
}

impl Surface<PBuffer> {
    #[inline]
    pub unsafe fn new(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize,
    ) -> Result<Self, Error> {
        egl::Surface::<PBuffer>::new(
            &disp.0,
            conf.map_config(|conf| &conf.0),
            size,
        )
        .map(|surface| Surface { wsurface: None, surface })
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new<NPS: NativePixmapSource>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nps: NPS,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        return Err(make_error!(ErrorType::NotSupported(
            "Wayland does not support pixmaps.".to_string(),
        )));
    }
}

#[derive(Debug)]
pub struct Context(egl::Context);

impl Context {
    #[inline]
    pub(crate) fn new(
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        egl::Context::new(
            &disp.0,
            cb.map_sharing(|ctx| &ctx.0),
            conf.map_config(|conf| &conf.0),
        ).map(Context)
    }

    #[inline]
    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        self.0.make_current_surfaceless()
    }

    #[inline]
    pub unsafe fn make_current<T: SurfaceTypeTrait>(&self, surf: &Surface<T>) -> Result<(), Error> {
        self.0.make_current(&surf.surface)
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.0.make_not_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.0.get_api()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> RawHandle {
        self.0.raw_handle()
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> Option<*const raw::c_void> {
        Some(self.0.get_egl_display())
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const c_void {
        self.0.get_proc_address(addr)
    }

    #[inline]
    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        self.0.get_config().map_config(Config)
    }
}
