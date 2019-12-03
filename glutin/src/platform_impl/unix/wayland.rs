use crate::api::egl;
use crate::config::{Api, ConfigAttribs, ConfigBuilder, ConfigWrapper};
use crate::context::ContextBuilderWrapper;
use crate::platform_impl::RawHandle;
use crate::surface::{PBuffer, Pixmap, Rect, SurfaceTypeTrait, Window};
use crate::utils::NoPrint;

use glutin_egl_sys as ffi;
use glutin_winit_interface::{NativeDisplaySource, NativePixmapSource, NativeWindowSource};
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
    pub fn new<NDS: NativeDisplaySource>(nds: &NDS) -> Result<Self, Error> {
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

    fn inner<'a, 'b>(
        conf: ConfigWrapper<&'a Config, &'b ConfigAttribs>,
    ) -> ConfigWrapper<&'a egl::Config, &'b ConfigAttribs> {
        conf.map_config(|conf| &conf.0)
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
        nws: &NWS,
    ) -> Result<(NWS::Window, Self), Error> {
        let win = nws.build_wayland()?;

        let dpi_factor = win.hidpi_factor();
        let size = win.inner_size().to_physical(dpi_factor);
        let (width, height): (u32, u32) = size.into();

        let surface = win.wayland_surface();
        let surface = match surface {
            Some(s) => s,
            None => {
                return Err(make_error!(ErrorType::NotSupported(
                    "Wayland not found".to_string(),
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
            Config::inner(conf),
            wsurface.ptr() as *const _,
        )
        .map(|surface| (win, Surface { wsurface: Some(NoPrint(wsurface)), surface }))
    }

    #[inline]
    pub fn update_after_resize(&self, size: dpi::PhysicalSize) {
        let (width, height): (u32, u32) = size.into();
        self.wsurface.unwrap().resize(width as i32, height as i32, 0, 0)
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
        unimplemented!()
    }
}

impl Surface<Pixmap> {
    #[inline]
    pub unsafe fn new<NPS: NativePixmapSource>(
        disp: &Display,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nps: &NPS,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Context(egl::Context);

impl Context {
    #[inline]
    pub(crate) fn new(
        disp: &Display,
        cb: ContextBuilderWrapper<&Context>,
        supports_surfaceless: bool,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        unimplemented!()
        //let context = {
        //    let cb = cb.map_sharing(|c| &c.context);
        //    egl::Context::new(
        //        &cb,
        //        supports_surfaceless,
        //        |c, _| Ok(c[0]),
        //        conf.with_config(conf.config),
        //    )?
        //};
        //Ok(Context { context })
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
