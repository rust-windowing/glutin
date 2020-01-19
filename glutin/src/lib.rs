//! The purpose of this library is to provide an OpenGL [`Context`] on as many
//! platforms as possible, as well as a [`Surface`] to go along with it. Before
//! you can do that, however, you need to decide on a [`Config`] for your
//! [`Context`]s and [`Surface`]s.
//!
//! You can use a [`ConfigsFinder`] to get a selection of [`Config`]s
//! that match your criteria. Among many things, you must specify in advance
//! what types of [`Surface`]s you're going to use the [`Config`] with.
//!
//! After settling on a [`Config`], you can make your [`Context`]s and
//! [`Surface`]s in any order you want, as long as your [`Surface`]'s and
//! [`Context`]'s [`Config`] are the same.
//!
//! Similar to how [`Config`]s are acquired via a [`ConfigsFinder`], so
//! too are [`Context`]s from a [`ContextBuilder`]. At this stage if you decide
//! to make multiple [`Context`]s you can also choose to share them. Some
//! platform specific restrictions are mentioned in [`ContextBuilderWrapper`]'s
//! [`with_sharing`] function.
//!
//! [`Surface`]s come in three flavors, [`Pixmap`]s, [`PBuffer`]s, and
//! [`Window`]s. They are created with the [`Surface::new_pixmap`],
//! [`Surface::new_pbuffer`], and [`Surface::new_window`] functions,
//! respectively. Alternatively, if you have already created your [`Window`]'s
//! or [`Pixmap`]'s native API's object, you can use
//! [`Surface::new_from_existing_window`] and
//! [`Surface::new_from_existing_pixmap`] to create your [`Surface`],
//! respectively.
//!
//! Once you've made a [`Context`] and a [`Surface`], you can make them current
//! with the [`Context::make_current`] function. Alternatively, you can use
//! [`Context::make_current_surfaceless`] if you don't want to make a
//! [`Surface`], but make sure that the [`Config`] you made the [`Context`]
//! with supported surfaceless.
//!
//! [`Context`]: crate::context::Context
//! [`ContextBuilder`]: crate::context::ContextBuilder
//! [`ContextBuilderWrapper`]: crate::context::ContextBuilderWrapper
//! [`with_sharing`]: crate::context::ContextBuilderWrapper::with_sharing()
//! [`Surface`]: crate::surface::Surface
//! [`Config`]: crate::config::ConfigWrapper
//! [`ConfigsFinder`]: crate::config::ConfigsFinder
//! [`Window`]: crate::surface::Window
//! [`PBuffer`]: crate::surface::PBuffer
//! [`Pixmap`]: crate::surface::Pixmap
//! [`Surface::new_pixmap`]: crate::surface::Surface::new_pixmap()
//! [`Surface::new_pbuffer`]: crate::surface::Surface::new_pbuffer()
//! [`Surface::new_window`]: crate::surface::Surface::new_window()
//! [`Surface::new_from_existing_pixmap`]: crate::surface::Surface::new_from_existing_window()
//! [`Surface::new_from_existing_window`]: crate::surface::Surface::new_from_existing_pixmap()
//! [`Context::make_current`]: crate::context::Context::make_current()
//! [`Context::make_current_surfaceless`]: crate::context::Context::make_current_surfaceless()

#![deny(
    missing_debug_implementations,
    //missing_docs,
)]
// FIXME: Remove before 0.23
#![allow(unused_imports, unused_variables)]

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate lazy_static;
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[macro_use]
extern crate objc;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate winit_types;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[macro_use]
extern crate glutin_x11_sym;

pub mod platform;

mod api;
pub mod config;
pub mod context;
mod platform_impl;
pub mod surface;
mod utils;
