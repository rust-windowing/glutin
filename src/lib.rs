#![feature(tuple_indexing)]
#![feature(unsafe_destructor)]
#![feature(globs)]
#![feature(phase)]
#![unstable]

//! The purpose of this library is to provide an OpenGL context on as many
//!  platforms as possible.
//!
//! # Building a window
//!
//! There are two ways to create a window:
//!
//!  - Calling `Window::new()`.
//!  - Calling `let builder = WindowBuilder::new()` then `builder.build()`.
//!
//! The first way is the simpliest way and will give you default values.
//!
//! The second way allows you to customize the way your window and GL context
//!  will look and behave.
//!
//! # Features
//!
//! This crate has two Cargo features: `window` and `headless`.
//!
//!  - `window` allows you to create regular windows and enables the `WindowBuilder` object.
//!  - `headless` allows you to do headless rendering, and enables
//!     the `HeadlessRendererBuilder` object.
//!
//! By default only `window` is enabled.

#[phase(plugin)] extern crate compile_msg;
#[phase(plugin)] extern crate gl_generator;
extern crate libc;

#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate core_foundation;

pub use events::*;

#[cfg(feature = "window")]
pub use window::*;
#[cfg(feature = "headless")]
pub use headless::*;

#[cfg(target_os = "windows")]
use win32 as winimpl;
#[cfg(target_os = "linux")]
use x11 as winimpl;
#[cfg(target_os = "macos")]
use osx as winimpl;
#[cfg(target_os = "android")]
use android as winimpl;

mod events;

#[cfg(feature = "window")]
mod window;
#[cfg(feature = "headless")]
mod headless;

#[cfg(target_os = "windows")]
mod win32;
#[cfg(target_os = "linux")]
mod x11;
#[cfg(target_os = "macos")]
mod osx;
#[cfg(target_os = "android")]
mod android;

#[cfg(all(not(target_os = "windows"), not(target_os = "linux"), not(target_os = "macos"), not(target_os = "android")))]
compile_error!("Only the `windows`, `linux` and `macos` platforms are supported")
