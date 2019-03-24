#![allow(non_camel_case_types)]

use std::os::raw;

pub type caca_display_t = raw::c_void;
pub type caca_canvas_t = raw::c_void;
pub type caca_dither_t = raw::c_void;

shared_library!(LibCaca, "libcaca.so.0",
    pub fn caca_create_display(cv: *mut caca_canvas_t) -> *mut caca_display_t,
    pub fn caca_free_display(dp: *mut caca_display_t) -> raw::c_int,
    pub fn caca_get_canvas(dp: *mut caca_display_t) -> *mut caca_canvas_t,
    pub fn caca_refresh_display(dp: *mut caca_display_t) -> raw::c_int,
    pub fn caca_dither_bitmap(cv: *mut caca_canvas_t, x: raw::c_int, y: raw::c_int,
                              w: raw::c_int, h: raw::c_int, d: *const caca_dither_t,
                              pixels: *const raw::c_void) -> raw::c_int,
    pub fn caca_free_dither(d: *mut caca_dither_t) -> raw::c_int,
    pub fn caca_create_dither(bpp: raw::c_int, w: raw::c_int, h: raw::c_int,
                              pitch: raw::c_int, rmask: u32, gmask: u32,
                              bmask: u32, amask: u32) -> *mut caca_dither_t,
    pub fn caca_get_canvas_width(cv: *mut caca_canvas_t) -> raw::c_int,
    pub fn caca_get_canvas_height(cv: *mut caca_canvas_t) -> raw::c_int,
);
