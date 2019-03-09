#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#![allow(unused_variables, dead_code)]

mod ffi;

use crate::api::osmesa::OsMesaContext;
use crate::{
    Api, ContextError, CreationError, GlAttributes, PixelFormat,
    PixelFormatRequirements,
};

use libc;
use winit::dpi;

use std::path::Path;

pub struct Context {
    opengl: OsMesaContext,
    libcaca: ffi::LibCaca,
    display: *mut ffi::caca_display_t,
    dither: *mut ffi::caca_dither_t,
}

impl Context {
    pub fn new(
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        dims: dpi::PhysicalSize,
    ) -> Result<Self, CreationError> {
        let gl_attr = gl_attr.clone().map_sharing(|w| &w.opengl);
        let opengl = OsMesaContext::new(pf_reqs, &gl_attr, dims)?;

        let dims = opengl.get_dimensions();

        let libcaca = match ffi::LibCaca::open(&Path::new("libcaca.so.0")) {
            Err(_) => {
                return Err(CreationError::NotSupported(
                    "could not find libcaca.so",
                ));
            }
            Ok(l) => l,
        };

        let display =
            unsafe { (libcaca.caca_create_display)(std::ptr::null_mut()) };

        if display.is_null() {
            return Err(CreationError::OsError(
                "caca_create_display failed".to_string(),
            ));
        }

        let dither = unsafe {
            #[cfg(target_endian = "little")]
            fn get_masks() -> (u32, u32, u32, u32) {
                (0xff, 0xff00, 0xff0000, 0xff000000)
            }
            #[cfg(target_endian = "big")]
            fn get_masks() -> (u32, u32, u32, u32) {
                (0xff000000, 0xff0000, 0xff00, 0xff)
            }

            let masks = get_masks();
            (libcaca.caca_create_dither)(
                32,
                dims.0 as libc::c_int,
                dims.1 as libc::c_int,
                dims.0 as libc::c_int * 4,
                masks.0,
                masks.1,
                masks.2,
                masks.3,
            )
        };

        if dither.is_null() {
            unsafe { (libcaca.caca_free_display)(display) };
            return Err(CreationError::OsError(
                "caca_create_dither failed".to_string(),
            ));
        }

        Ok(Context {
            libcaca,
            display,
            opengl,
            dither,
        })
    }

    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        self.opengl.make_current()
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        self.opengl.is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        self.opengl.get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        unsafe {
            let canvas = (self.libcaca.caca_get_canvas)(self.display);
            let width = (self.libcaca.caca_get_canvas_width)(canvas);
            let height = (self.libcaca.caca_get_canvas_height)(canvas);

            let buffer = self
                .opengl
                .get_framebuffer()
                .chunks(self.opengl.get_dimensions().0 as usize)
                .flat_map(|i| i.iter().cloned())
                .rev()
                .collect::<Vec<u32>>();

            (self.libcaca.caca_dither_bitmap)(
                canvas,
                0,
                0,
                width as libc::c_int,
                height as libc::c_int,
                self.dither,
                buffer.as_ptr() as *const _,
            );
            (self.libcaca.caca_refresh_display)(self.display);
        };

        Ok(())
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.opengl.get_api()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.opengl.get_pixel_format()
    }
}

impl Drop for Context {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            (self.libcaca.caca_free_dither)(self.dither);
            (self.libcaca.caca_free_display)(self.display);
        }
    }
}
