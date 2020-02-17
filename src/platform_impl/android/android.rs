#![cfg(target_os = "android")]

use crate::api::egl;
use crate::config::{ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapInterval};
use crate::context::ContextBuilderWrapper;
pub use crate::platform::android::ConfigPlatformAttributes;
use crate::surface::{PBuffer, Pixmap, SurfaceTypeTrait, Window};
use glutin_interface::{
    AndroidWindowParts, NativeDisplay, NativePixmap, NativePixmapSource, NativeWindow,
    NativeWindowSource, RawWindow, Seal,
};
use std::ops::Deref;
use std::os::raw;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config(egl::Config);

impl Deref for Config {
    type Target = egl::Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Config {
    pub fn new<ND: NativeDisplay>(
        cf: &ConfigsFinder,
        nd: &ND,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error> {
        Ok(
            egl::Config::new(cf, nd, |confs, _| confs.into_iter().map(Ok).collect())?
                .into_iter()
                .map(|(attribs, config)| (attribs, Config(config)))
                .collect(),
        )
    }

    pub fn raw_config(&self) -> *const raw::c_void {
        (**self).raw_config()
    }

    pub fn raw_display(&self) -> *mut raw::c_void {
        (**self).raw_display()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Surface<T: SurfaceTypeTrait>(egl::Surface<T>);

impl<T: SurfaceTypeTrait> Deref for Surface<T> {
    type Target = egl::Surface<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: SurfaceTypeTrait> Surface<T> {
    pub fn is_current(&self) -> bool {
        (&**self).is_current()
    }

    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        (&**self).get_config().map_config(Config)
    }

    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        (&**self).make_not_current()
    }

    pub fn raw_surface(&self) -> *const raw::c_void {
        (&**self).raw_surface()
    }

    pub fn size(&self) -> Result<dpi::PhysicalSize<u32>, Error> {
        (&**self).size()
    }
}

impl Surface<Window> {
    pub fn build_window<NWS: NativeWindowSource>(
        _conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nws: &NWS,
        wb: NWS::WindowBuilder,
    ) -> Result<NWS::Window, Error> {
        #[allow(deprecated)]
        nws.build_android(
            wb,
            AndroidWindowParts {
                _non_exhaustive_do_not_use: Seal,
            },
        )
    }

    pub unsafe fn new_existing<NW: NativeWindow>(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        nw: &NW,
    ) -> Result<Self, Error> {
        let a_native_window = match nw.raw_window() {
            RawWindow::Android {
                a_native_window, ..
            } => a_native_window,
            _ => {
                return Err(make_error!(ErrorType::NotSupported(
                    "Expected android window.".to_string(),
                )))
            }
        };
        let config = conf.map_config(|conf| &conf.0);
        let surface = egl::Surface::<Window>::new(config, a_native_window)?;
        Ok(Surface(surface))
    }

    pub fn swap_buffers(&self) -> Result<(), Error> {
        (&**self).swap_buffers()
    }

    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        (&**self).swap_buffers_with_damage(rects)
    }

    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        (&**self).modify_swap_interval(swap_interval)
    }
}

impl Surface<PBuffer> {
    pub unsafe fn new(
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
        size: dpi::PhysicalSize<u32>,
        largest: bool,
    ) -> Result<Self, Error> {
        let config = conf.clone().map_config(|conf| &**conf);
        let surface = egl::Surface::<PBuffer>::new(config, size, largest)?;
        Ok(Surface(surface))
    }
}

impl Surface<Pixmap> {
    pub unsafe fn build_pixmap<NPS: NativePixmapSource>(
        _conf: ConfigWrapper<&Config, &ConfigAttribs>,
        _nps: &NPS,
        _pb: NPS::PixmapBuilder,
    ) -> Result<NPS::Pixmap, Error> {
        Err(make_error!(ErrorType::NotSupported(
            "pixmaps not supported on android".into()
        )))
    }

    pub unsafe fn new_existing<NP: NativePixmap>(
        _conf: ConfigWrapper<&Config, &ConfigAttribs>,
        _np: &NP,
    ) -> Result<Self, Error> {
        Err(make_error!(ErrorType::NotSupported(
            "pixmaps not supported on android".into()
        )))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Context(egl::Context);

impl Deref for Context {
    type Target = egl::Context;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context {
    pub(crate) fn new(
        cb: ContextBuilderWrapper<&Context>,
        conf: ConfigWrapper<&Config, &ConfigAttribs>,
    ) -> Result<Self, Error> {
        let context = egl::Context::new(
            cb.map_sharing(|ctx| &**ctx),
            conf.map_config(|conf| &**conf),
        )?;
        Ok(Context(context))
    }

    pub unsafe fn make_current_surfaceless(&self) -> Result<(), Error> {
        (**self).make_current_surfaceless()
    }

    pub(crate) unsafe fn make_current<T: SurfaceTypeTrait>(
        &self,
        surf: &Surface<T>,
    ) -> Result<(), Error> {
        (**self).make_current(&**surf)
    }

    pub(crate) unsafe fn make_current_rw<TR: SurfaceTypeTrait, TW: SurfaceTypeTrait>(
        &self,
        read_surf: &Surface<TR>,
        write_surf: &Surface<TW>,
    ) -> Result<(), Error> {
        (**self).make_current_rw(&**read_surf, &**write_surf)
    }

    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        (**self).make_not_current()
    }

    pub fn is_current(&self) -> bool {
        (**self).is_current()
    }

    pub fn get_proc_address(&self, addr: &str) -> Result<*const raw::c_void, Error> {
        (**self).get_proc_address(addr)
    }

    pub fn get_config(&self) -> ConfigWrapper<Config, ConfigAttribs> {
        (**self).get_config().map_config(Config)
    }

    pub fn raw_context(&self) -> *mut raw::c_void {
        (**self).raw_context()
    }
}
