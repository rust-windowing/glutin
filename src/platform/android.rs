#![cfg(any(target_os = "android"))]
use crate::config::Config;
use crate::context::Context;
use crate::surface::{Surface, SurfaceTypeTrait};
use std::os::raw;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConfigPlatformAttributes;

pub trait ConfigExt {
    unsafe fn raw_config(&self) -> *const raw::c_void;
    unsafe fn raw_display(&self) -> *mut raw::c_void;
}

impl ConfigExt for Config {
    unsafe fn raw_config(&self) -> *const raw::c_void {
        self.config.raw_config()
    }

    unsafe fn raw_display(&self) -> *mut raw::c_void {
        self.config.raw_display()
    }
}

pub trait SurfaceExt {
    unsafe fn raw_surface(&self) -> *const raw::c_void;
}

impl<T: SurfaceTypeTrait> SurfaceExt for Surface<T> {
    unsafe fn raw_surface(&self) -> *const raw::c_void {
        self.0.raw_surface()
    }
}

pub trait ContextExt {
    unsafe fn raw_context(&self) -> *mut raw::c_void;
}

impl ContextExt for Context {
    unsafe fn raw_context(&self) -> *mut raw::c_void {
        self.0.raw_context()
    }
}
