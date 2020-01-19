//! Everything related to creating and manipulating [`Surface`]s.
//!
//! You can use the [`new_pbuffer`], [`new_window`], and [`new_pixmap`] types
//! to create a [`Surface`]. Alternatively, if you have already created your
//! [`Window`]'s or [`Pixmap`]'s native API's object, you can use
//! [`new_from_existing_window`] and
//! [`new_from_existing_pixmap`] to create the [`Surface`], respectively.
//!
//! [`Surface`]: crate::surface::Surface
//! [`Window`]: crate::surface::Window
//! [`PBuffer`]: crate::surface::PBuffer
//! [`Pixmap`]: crate::surface::Pixmap
//! [`new_pixmap`]: crate::surface::Surface::new_pixmap()
//! [`new_pbuffer`]: crate::surface::Surface::new_pbuffer()
//! [`new_window`]: crate::surface::Surface::new_window()
//! [`new_from_existing_pixmap`]: crate::surface::Surface::new_from_existing_window()
//! [`new_from_existing_window`]: crate::surface::Surface::new_from_existing_pixmap()
use crate::config::{Config, SwapInterval};
use crate::platform_impl;

use glutin_interface::{NativePixmap, NativePixmapSource, NativeWindow, NativeWindowSource};
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};

/// A [`Surface`]'s type. Returned from calling
/// [`SurfaceTypeTrait::surface_type()`] on the type specializing your
/// [`Surface`].
///
/// [`Surface`]: crate::surface::Surface
/// [`SurfaceTypeTrait::surface_type()`]: crate::surface::SurfaceTypeTrait::surface_type()
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum SurfaceType {
    /// A [`Window`](crate::surface::Window)
    Window,
    /// A [`PBuffer`](crate::surface::PBuffer)
    PBuffer,
    /// A [`Pixmap`](crate::surface::Pixmap)
    Pixmap,
}

/// A trait implemented on all allowed specializations of [`Surface`].
///
/// [`Surface`]s can only be specialized by the [`PBuffer`], [`Pixmap`], and
/// [`Window`] types.
///
/// [`Surface`]: crate::surface::Surface
/// [`Window`]: crate::surface::Window
/// [`PBuffer`]: crate::surface::PBuffer
/// [`Pixmap`]: crate::surface::Pixmap
pub trait SurfaceTypeTrait {
    /// Returns the [`SurfaceType`] of the specialization.
    ///
    /// [`SurfaceType`]: crate::surface::SurfaceType
    fn surface_type() -> SurfaceType;
}

/// A type that specializes the [`Surface`] type when the [`Surface`] is backed
/// by a window.
///
/// For more info, refer to [`Surface`]'s docs.
///
/// [`Surface`]: crate::surface::Surface
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Window {}

/// A type that specializes the [`Surface`] type when the [`Surface`] is backed
/// by a pixel buffer (or `PBuffer` for short).
///
/// For more info, refer to [`Surface`]'s docs.
///
/// [`Surface`]: crate::surface::Surface
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PBuffer {}

/// A type that specializes the [`Surface`] type when the [`Surface`] is backed
/// by a pixel map (or `Pixmap` for short).
///
/// For more info, refer to [`Surface`]'s docs.
///
/// [`Surface`]: crate::surface::Surface
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Pixmap {}

impl SurfaceTypeTrait for Window {
    #[inline]
    fn surface_type() -> SurfaceType {
        SurfaceType::Window
    }
}

impl SurfaceTypeTrait for PBuffer {
    #[inline]
    fn surface_type() -> SurfaceType {
        SurfaceType::PBuffer
    }
}

impl SurfaceTypeTrait for Pixmap {
    #[inline]
    fn surface_type() -> SurfaceType {
        SurfaceType::Pixmap
    }
}

/// Represents an OpenGL surface.
///
/// Surfaces come in three flavours:
///  * [`Window`] surfaces: Surfaces that are allocated by the native API used
///  for onscreen rendering.
///  * [`PBuffer`] surfaces: Surfaces that are allocated offscreen by and
///  manipulated by the OpenGL driver.
///  * [`Pixmap`] surfaces: Surfaces that are allocated offscreen by the native
///  API that can be manipulated by the OpenGL driver.
///
/// The `Surface` type is specialized by one of the rust types linked to above.
/// By doing so, Glutin stops you from calling functions unavailable due to the
/// `Surface`'s type at compile-time.
///
/// Clients writing libraries that want to accept all surfaces irrespective of
/// what's backing them should make their functions/structs/ect generic over the
/// [`SurfaceTypeTrait`] trait.
///
/// **WARNING** `Surface`s that are backed by some native API object must outlive
/// the object they are backed by.
///
/// **WARNING** `Surface`s cannot be used from threads they are not current on.
/// If dropped from a different thread than the one they are currently on, UB can
/// happen. If a surface is current, please call [`make_not_current`] before
/// moving it between two threads.
///
/// [`Window`]: crate::surface::Window
/// [`PBuffer`]: crate::surface::PBuffer
/// [`Pixmap`]: crate::surface::Pixmap
/// [`SurfaceTypeTrait`]: crate::surface::SurfaceTypeTrait
/// [`make_not_current`]: crate::surface::Surface::make_not_current
#[derive(Debug, PartialEq, Eq)]
pub struct Surface<T: SurfaceTypeTrait>(pub(crate) platform_impl::Surface<T>);

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    fn drop(&mut self) {
        unsafe {
            self.make_not_current().unwrap();
        }
    }
}

impl<T: SurfaceTypeTrait> Surface<T> {
    /// Returns `true` if this context is the current one in this thread.
    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.is_current()
    }

    /// Returns the [`Config`] that the surface was created with.
    ///
    /// [`Config`]: crate::config::ConfigWrapper
    #[inline]
    pub fn get_config(&self) -> Config {
        self.0.get_config()
    }

    /// If this surface is current, makes this surface not current. If this
    /// surface is not current, however, then this function does nothing.
    ///
    /// The current [`Context`], if any, will also become not current.
    ///
    /// The previously current [`Context`] might get `glFlush`ed if its
    /// [`ReleaseBehaviour`] is equal to [`Flush`].
    ///
    /// For how to handle errors, refer to [`Context`]'s [`make_current`].
    ///
    /// [`Context`]: crate::context::Context
    /// [`make_current`]: crate::context::Context::make_current
    /// [`ReleaseBehaviour`]: crate::context::ReleaseBehaviour
    /// [`Flush`]: crate::context::ReleaseBehaviour::Flush
    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
        self.0.make_not_current()
    }
}

impl Surface<Pixmap> {
    /// Takes an `NPS` and its `NPS::PixmapBuilder` type, returning a
    /// `NPS::Pixmap` plus a `Surface<`[`Pixmap`]`>`.
    ///
    /// **WARNING** The `Surface` must be dropped before the `NPS::Pixmap`.
    ///
    /// [`Pixmap`]: crate::surface::Pixmap
    #[inline]
    pub unsafe fn new_pixmap<NPS: NativePixmapSource>(
        conf: &Config,
        nps: &NPS,
        pb: NPS::PixmapBuilder,
    ) -> Result<(NPS::Pixmap, Self), Error> {
        if !conf.attribs().supports_pixmaps {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Tried to make pixmap surface with config without `supports_pixmaps`.".to_string()
            )));
        }

        platform_impl::Surface::<Pixmap>::new(conf.as_ref(), nps, pb)
            .map(|(pix, surf)| (pix, Surface(surf)))
    }

    /// Takes an pre-existing pixmap, returning a `Surface<`[`Pixmap`]`>`.
    ///
    /// The pixmap can not be currently in use by any other `Surface`.
    ///
    /// Please prefer to use [`new_pixmap`] when possible.
    ///
    /// The pixmap and [`Config`] must have been made with the same `EventLoop`.
    ///
    /// Some platforms place additional restrictions on what [`Config`]s can be
    /// used with the pixmap:
    ///  * Wayland: Does not support pixmaps.
    ///  * X11: FIXME determine when implemented
    ///  FIXME missing plats
    ///
    /// **WARNING** The `Surface` must be dropped before the pixmap.
    ///
    /// [`new_pixmap`]: crate::surface::Surface::new_pixmap()
    /// [`Pixmap`]: crate::surface::Pixmap
    /// [`Config`]: crate::config::Config
    #[inline]
    pub unsafe fn new_from_existing_pixmap<NP: NativePixmap>(
        conf: &Config,
        np: &NP,
    ) -> Result<Self, Error> {
        if !conf.attribs().supports_pixmaps {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Tried to make pixmap surface with config without `supports_pixmaps`.".to_string()
            )));
        }

        platform_impl::Surface::<Pixmap>::new_existing(conf.as_ref(), np).map(Surface)
    }
}

impl Surface<PBuffer> {
    /// Creates a [`PBuffer`] surface.
    ///
    /// [`PBuffer`]: crate::surface::PBuffer
    #[inline]
    pub unsafe fn new_pbuffer(conf: &Config, size: &dpi::PhysicalSize<u32>) -> Result<Self, Error> {
        if !conf.attribs().supports_pbuffers {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Tried to make pbuffer surface with config without `supports_pbuffers`."
                    .to_string()
            )));
        }

        platform_impl::Surface::<PBuffer>::new(conf.as_ref(), size).map(Surface)
    }
}

impl Surface<Window> {
    /// Takes an `NWS` and its `NWS::WindowBuilder` type, returning a
    /// `NWS::Window` plus a `Surface<`[`Window`]`>`.
    ///
    /// **WARNING** The `Surface` must be dropped before the `NWS::Window`.
    ///
    /// [`Window`]: crate::surface::Window
    #[inline]
    pub unsafe fn new_window<NWS: NativeWindowSource>(
        conf: &Config,
        nws: &NWS,
        wb: NWS::WindowBuilder,
    ) -> Result<(NWS::Window, Self), Error> {
        if !conf.attribs().supports_windows {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Tried to make window surface with config without `supports_windows`.".to_string()
            )));
        }

        platform_impl::Surface::<Window>::new(conf.as_ref(), nws, wb)
            .map(|(win, surf)| (win, Surface(surf)))
    }

    /// Takes an pre-existing window, returning a `Surface<`[`Window`]`>`.
    ///
    /// The window can not be currently in use by any other `Surface`.
    ///
    /// Please prefer to use [`new_window`] when possible.
    ///
    /// The window and [`Config`] must have been made with the same `EventLoop`.
    ///
    /// Some platforms place additional restrictions on what [`Config`]s can be
    /// used with the window:
    ///  * Wayland: No known additional restrictions.
    ///  * X11: The [`Config`]'s `XVisualInfo`'s `depth` and `visual` must match
    ///  the window's.
    ///  FIXME missing plats
    ///
    /// **WARNING** The `Surface` must be dropped before the window.
    ///
    /// [`new_window`]: crate::surface::Surface::new_window()
    /// [`Window`]: crate::surface::Window
    /// [`Config`]: crate::config::Config
    #[inline]
    pub unsafe fn new_from_existing_window<NW: NativeWindow>(
        conf: &Config,
        nw: &NW,
    ) -> Result<Self, Error> {
        if !conf.attribs().supports_windows {
            return Err(make_error!(ErrorType::BadApiUsage(
                "Tried to make window surface with config without `supports_windows`.".to_string()
            )));
        }

        platform_impl::Surface::<Window>::new_existing(conf.as_ref(), nw).map(Surface)
    }

    /// Swaps the buffers in case of double or triple buffering.
    ///
    /// You should call this function every time you have finished rendering, or
    /// the image may not be displayed on the screen.
    ///
    /// This `Surface` must be current.
    ///
    /// **WARNING**: if the swap interval when creating the surface was not
    /// `DontWait` or your graphics driver decided to override your requested
    /// behaviour, this function may block. Please refer to [`SwapInterval`].
    ///
    /// [`SwapInterval`]: crate::config::SwapInterval
    #[inline]
    pub fn swap_buffers(&self) -> Result<(), Error> {
        if cfg!(debug_assertions) && !self.is_current() {
            return Err(make_error!(ErrorType::BadApiUsage(
                "`swap_buffers` called on surface that is not current.".to_string()
            )));
        }
        self.0.swap_buffers()
    }

    /// Similiar to [`Surface::swap_buffers`] but allows specifying damage
    /// rectangles.
    ///
    /// [`Surface::swap_buffers`]: crate::surface::Surface::swap_buffers()
    #[inline]
    pub fn swap_buffers_with_damage(&self, rects: &[dpi::Rect]) -> Result<(), Error> {
        if cfg!(debug_assertions) && !self.is_current() {
            return Err(make_error!(ErrorType::BadApiUsage(
                "`swap_buffers_with_damage` called on surface that is not current.".to_string()
            )));
        }
        self.0.swap_buffers_with_damage(rects)
    }

    /// On Wayland, Glutin clients must call `update_after_resize`, on the
    /// `Surface` whenever the backing [`Window`]'s size changes.
    ///
    /// No-ops on other platforms. Please make sure to also call your
    /// [`Context`]'s [`update_after_resize`].
    ///
    /// [`Window`]: crate::surface::Window
    /// [`Context`]: crate::context::Context
    /// [`update_after_resize`]: crate::context::Context::update_after_resize
    #[inline]
    pub fn update_after_resize(&self, size: &dpi::PhysicalSize<u32>) {
        #![cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        self.0.update_after_resize(size);
    }

    /// Modifies the `Surface`'s [`SwapInterval`] to the requested one.
    ///
    /// This `Surface` must be current.
    ///
    /// The [`SwapInterval`] must lie in the [`SwapIntervalRange`] specified in
    /// the [`Config`] with which your [`Surface`] was made with.
    ///
    /// As mentioned in [`SwapInterval`], your request may be silently ignored
    /// by the OpenGL driver. For more information, refer to [`SwapInterval`].
    ///
    /// [`SwapInterval`]: crate::config::SwapInterval
    /// [`SwapIntervalRange`]: crate::config::SwapIntervalRange
    /// [`Config`]: crate::config::Config
    #[inline]
    pub fn modify_swap_interval(&self, swap_interval: SwapInterval) -> Result<(), Error> {
        if cfg!(debug_assertions) {
            if !self.is_current() {
                return Err(make_error!(ErrorType::BadApiUsage(
                    "`modify_swap_interval` called on surface that is not current.".to_string()
                )));
            }
            let conf = self.get_config();
            let attribs = conf.attribs();
            if attribs
                .swap_interval_ranges
                .iter()
                .find(|r| r.contains(&swap_interval))
                .is_none()
            {
                return Err(make_error!(ErrorType::BadApiUsage(format!(
                    "SwapInterval of {:?} not in ranges {:?}.",
                    swap_interval, attribs.swap_interval_ranges
                ))));
            }
        }
        swap_interval.validate()?;

        self.0.modify_swap_interval(swap_interval)
    }
}
