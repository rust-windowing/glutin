//! A cross platform OpenGL surface representation.
#![allow(unreachable_patterns)]

use std::marker::PhantomData;
use std::num::NonZeroU32;

use raw_window_handle::RawWindowHandle;

use crate::context::{PossiblyCurrentContext, PossiblyCurrentGlContext};
use crate::display::{Display, GetGlDisplay};
use crate::error::Result;
use crate::private::{gl_api_dispatch, Sealed};

#[cfg(cgl_backend)]
use crate::api::cgl::surface::Surface as CglSurface;
#[cfg(egl_backend)]
use crate::api::egl::surface::Surface as EglSurface;
#[cfg(glx_backend)]
use crate::api::glx::surface::Surface as GlxSurface;
#[cfg(wgl_backend)]
use crate::api::wgl::surface::Surface as WglSurface;

/// A trait to group common operations on the surface.
pub trait GlSurface<T: SurfaceTypeTrait>: Sealed {
    /// The type of the surface.
    type SurfaceType: SurfaceTypeTrait;
    /// The context to access surface data.
    type Context: PossiblyCurrentGlContext;

    /// The age of the back buffer of that surface. The `0` indicates that the
    /// buffer is either a new one or we failed to get the information about
    /// its age. In both cases you must redraw the entire buffer.
    fn buffer_age(&self) -> u32;

    /// The **physical** width of the underlying surface.
    fn width(&self) -> Option<u32>;

    /// The **physical** height of the underlying surface.
    ///
    /// # Platform specific
    ///
    /// **macOS:** - **This will block if your main thread is blocked.**
    fn height(&self) -> Option<u32>;

    /// Check whether the surface is single buffered.
    ///
    /// # Platform specific
    ///
    /// **macOS:** - **This will block if your main thread is blocked.**
    fn is_single_buffered(&self) -> bool;

    /// Swaps the underlying back buffers when the surface is not single
    /// buffered.
    fn swap_buffers(&self, context: &Self::Context) -> Result<()>;

    /// Check whether the surface is current on to the current thread.
    fn is_current(&self, context: &Self::Context) -> bool;

    /// Check whether the surface is the current draw surface to the current
    /// thread.
    fn is_current_draw(&self, context: &Self::Context) -> bool;

    /// Check whether the surface is the current read surface to the current
    /// thread.
    fn is_current_read(&self, context: &Self::Context) -> bool;

    /// Set swap interval for the surface.
    ///
    /// See [`crate::surface::SwapInterval`] for details.
    fn set_swap_interval(&self, context: &Self::Context, interval: SwapInterval) -> Result<()>;

    /// Resize the surface to a new size.
    ///
    /// This call is for compatibility reasons, on most platforms it's a no-op.
    ///
    /// # Platform specific
    ///
    /// **Wayland:** - resizes the surface.
    /// **macOS:** - **This will block if your main thread is blocked.**
    /// **Other:** - no op.
    fn resize(&self, context: &Self::Context, width: NonZeroU32, height: NonZeroU32)
    where
        Self::SurfaceType: ResizeableSurface;
}

/// The marker trait to indicate the type of the surface.
pub trait SurfaceTypeTrait: Sealed {
    /// Get the type of the surface.
    fn surface_type() -> SurfaceType;
}

/// Marker indicating that the surface could be resized.
pub trait ResizeableSurface: Sealed {}

/// Trait for accessing the raw GL surface.
pub trait AsRawSurface {
    /// Get the raw handle to the surface.
    fn raw_surface(&self) -> RawSurface;
}

/// Builder to get the required set of attributes initialized before hand.
#[derive(Default, Debug, Clone)]
pub struct SurfaceAttributesBuilder<T: SurfaceTypeTrait + Default> {
    attributes: SurfaceAttributes<T>,
}

impl<T: SurfaceTypeTrait + Default> SurfaceAttributesBuilder<T> {
    /// Get new surface attributes.
    pub fn new() -> Self {
        Default::default()
    }

    /// Specify whether the surface should support srgb or not. Passing `None`
    /// means you don't care.
    ///
    /// # Api-specific.
    ///
    /// This only controls EGL surfaces, other platforms use the context for
    /// that.
    pub fn with_srgb(mut self, srgb: Option<bool>) -> Self {
        self.attributes.srgb = srgb;
        self
    }
}

impl SurfaceAttributesBuilder<WindowSurface> {
    /// Specify whether the single buffer should be used instead of double
    /// buffering. This doesn't guarantee that the resulted buffer will have
    /// only single buffer, to know that the single buffer is actually used
    /// query the created surface with [`Surface::is_single_buffered`].
    ///
    /// The surface is requested as double buffered by default.
    ///
    /// # Api-specific.
    ///
    /// This is EGL specific, other platforms use the context for that.
    pub fn with_single_buffer(mut self, single_buffer: bool) -> Self {
        self.attributes.single_buffer = single_buffer;
        self
    }

    /// Build the surface attributes suitable to create a window surface.
    pub fn build(
        mut self,
        raw_window_handle: RawWindowHandle,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> SurfaceAttributes<WindowSurface> {
        self.attributes.raw_window_handle = Some(raw_window_handle);
        self.attributes.width = Some(width);
        self.attributes.height = Some(height);
        self.attributes
    }
}

impl SurfaceAttributesBuilder<PbufferSurface> {
    /// Request the largest pbuffer.
    pub fn with_largest_pbuffer(mut self, largest_pbuffer: bool) -> Self {
        self.attributes.largest_pbuffer = largest_pbuffer;
        self
    }

    /// The same as in
    /// [`SurfaceAttributesBuilder::<WindowSurface>::with_single_buffer`].
    pub fn with_single_buffer(mut self, single_buffer: bool) -> Self {
        self.attributes.single_buffer = single_buffer;
        self
    }

    /// Build the surface attributes suitable to create a pbuffer surface.
    pub fn build(
        mut self,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> SurfaceAttributes<PbufferSurface> {
        self.attributes.width = Some(width);
        self.attributes.height = Some(height);
        self.attributes
    }
}

impl SurfaceAttributesBuilder<PixmapSurface> {
    /// Build the surface attributes suitable to create a pixmap surface.
    pub fn build(mut self, native_pixmap: NativePixmap) -> SurfaceAttributes<PixmapSurface> {
        self.attributes.native_pixmap = Some(native_pixmap);
        self.attributes
    }
}

/// Attributes which are used for creating a particular surface.
#[derive(Default, Debug, Clone)]
pub struct SurfaceAttributes<T: SurfaceTypeTrait> {
    pub(crate) srgb: Option<bool>,
    pub(crate) single_buffer: bool,
    pub(crate) width: Option<NonZeroU32>,
    pub(crate) height: Option<NonZeroU32>,
    pub(crate) largest_pbuffer: bool,
    pub(crate) raw_window_handle: Option<RawWindowHandle>,
    pub(crate) native_pixmap: Option<NativePixmap>,
    _ty: PhantomData<T>,
}

/// Marker that used to type-gate methods for window.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSurface;

impl SurfaceTypeTrait for WindowSurface {
    fn surface_type() -> SurfaceType {
        SurfaceType::Window
    }
}

impl ResizeableSurface for WindowSurface {}

impl Sealed for WindowSurface {}

/// Marker that used to type-gate methods for pbuffer.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PbufferSurface;

impl SurfaceTypeTrait for PbufferSurface {
    fn surface_type() -> SurfaceType {
        SurfaceType::Pbuffer
    }
}

impl Sealed for PbufferSurface {}

/// Marker that used to type-gate methods for pixmap.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixmapSurface;

impl SurfaceTypeTrait for PixmapSurface {
    fn surface_type() -> SurfaceType {
        SurfaceType::Pixmap
    }
}

impl Sealed for PixmapSurface {}

/// The underlying type of the surface.
#[derive(Debug, Clone, Copy)]
pub enum SurfaceType {
    /// The window surface.
    Window,

    /// Pixmap surface.
    Pixmap,

    /// Pbuffer surface.
    Pbuffer,
}

/// The GL surface that is used for rendering.
///
/// The GL surface is not thread safe, it can neither be [`Send`] nor [`Sync`],
/// so it should be created on the thread it'll be used to render.
///
/// ```compile_fail
/// fn test_send<T: Send>() {}
/// test_send::<glutin::surface::Surface<glutin::surface::WindowSurface>>();
/// ```
/// ```compile_fail
/// fn test_sync<T: Sync>() {}
/// test_sync::<glutin::surface::Surface<glutin::surface::WindowSurface>>();
/// ```
#[derive(Debug)]
pub enum Surface<T: SurfaceTypeTrait> {
    /// The EGL surface.
    #[cfg(egl_backend)]
    Egl(EglSurface<T>),

    /// The GLX surface.
    #[cfg(glx_backend)]
    Glx(GlxSurface<T>),

    /// The WGL surface.
    #[cfg(wgl_backend)]
    Wgl(WglSurface<T>),

    /// The CGL surface.
    #[cfg(cgl_backend)]
    Cgl(CglSurface<T>),
}

impl<T: SurfaceTypeTrait> GlSurface<T> for Surface<T> {
    type Context = PossiblyCurrentContext;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        gl_api_dispatch!(self; Self(surface) => surface.buffer_age())
    }

    fn width(&self) -> Option<u32> {
        gl_api_dispatch!(self; Self(surface) => surface.width())
    }

    fn height(&self) -> Option<u32> {
        gl_api_dispatch!(self; Self(surface) => surface.height())
    }

    fn is_single_buffered(&self) -> bool {
        gl_api_dispatch!(self; Self(surface) => surface.is_single_buffered())
    }

    fn swap_buffers(&self, context: &Self::Context) -> Result<()> {
        match (self, context) {
            #[cfg(egl_backend)]
            (Self::Egl(surface), PossiblyCurrentContext::Egl(context)) => {
                surface.swap_buffers(context)
            },
            #[cfg(glx_backend)]
            (Self::Glx(surface), PossiblyCurrentContext::Glx(context)) => {
                surface.swap_buffers(context)
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(surface), PossiblyCurrentContext::Cgl(context)) => {
                surface.swap_buffers(context)
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(surface), PossiblyCurrentContext::Wgl(context)) => {
                surface.swap_buffers(context)
            },
            _ => unreachable!(),
        }
    }

    fn set_swap_interval(&self, context: &Self::Context, interval: SwapInterval) -> Result<()> {
        match (self, context) {
            #[cfg(egl_backend)]
            (Self::Egl(surface), PossiblyCurrentContext::Egl(context)) => {
                surface.set_swap_interval(context, interval)
            },
            #[cfg(glx_backend)]
            (Self::Glx(surface), PossiblyCurrentContext::Glx(context)) => {
                surface.set_swap_interval(context, interval)
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(surface), PossiblyCurrentContext::Cgl(context)) => {
                surface.set_swap_interval(context, interval)
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(surface), PossiblyCurrentContext::Wgl(context)) => {
                surface.set_swap_interval(context, interval)
            },
            _ => unreachable!(),
        }
    }

    fn is_current(&self, context: &Self::Context) -> bool {
        match (self, context) {
            #[cfg(egl_backend)]
            (Self::Egl(surface), PossiblyCurrentContext::Egl(context)) => {
                surface.is_current(context)
            },
            #[cfg(glx_backend)]
            (Self::Glx(surface), PossiblyCurrentContext::Glx(context)) => {
                surface.is_current(context)
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(surface), PossiblyCurrentContext::Cgl(context)) => {
                surface.is_current(context)
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(surface), PossiblyCurrentContext::Wgl(context)) => {
                surface.is_current(context)
            },
            _ => unreachable!(),
        }
    }

    fn is_current_draw(&self, context: &Self::Context) -> bool {
        match (self, context) {
            #[cfg(egl_backend)]
            (Self::Egl(surface), PossiblyCurrentContext::Egl(context)) => {
                surface.is_current_draw(context)
            },
            #[cfg(glx_backend)]
            (Self::Glx(surface), PossiblyCurrentContext::Glx(context)) => {
                surface.is_current_draw(context)
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(surface), PossiblyCurrentContext::Cgl(context)) => {
                surface.is_current_draw(context)
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(surface), PossiblyCurrentContext::Wgl(context)) => {
                surface.is_current_draw(context)
            },
            _ => unreachable!(),
        }
    }

    fn is_current_read(&self, context: &Self::Context) -> bool {
        match (self, context) {
            #[cfg(egl_backend)]
            (Self::Egl(surface), PossiblyCurrentContext::Egl(context)) => {
                surface.is_current_read(context)
            },
            #[cfg(glx_backend)]
            (Self::Glx(surface), PossiblyCurrentContext::Glx(context)) => {
                surface.is_current_read(context)
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(surface), PossiblyCurrentContext::Cgl(context)) => {
                surface.is_current_read(context)
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(surface), PossiblyCurrentContext::Wgl(context)) => {
                surface.is_current_read(context)
            },
            _ => unreachable!(),
        }
    }

    fn resize(&self, context: &Self::Context, width: NonZeroU32, height: NonZeroU32)
    where
        Self::SurfaceType: ResizeableSurface,
    {
        match (self, context) {
            #[cfg(egl_backend)]
            (Self::Egl(surface), PossiblyCurrentContext::Egl(context)) => {
                surface.resize(context, width, height)
            },
            #[cfg(glx_backend)]
            (Self::Glx(surface), PossiblyCurrentContext::Glx(context)) => {
                surface.resize(context, width, height)
            },
            #[cfg(cgl_backend)]
            (Self::Cgl(surface), PossiblyCurrentContext::Cgl(context)) => {
                surface.resize(context, width, height)
            },
            #[cfg(wgl_backend)]
            (Self::Wgl(surface), PossiblyCurrentContext::Wgl(context)) => {
                surface.resize(context, width, height)
            },
            _ => unreachable!(),
        }
    }
}

impl<T: SurfaceTypeTrait> GetGlDisplay for Surface<T> {
    type Target = Display;

    fn display(&self) -> Self::Target {
        gl_api_dispatch!(self; Self(surface) => surface.display(); as Display)
    }
}

impl<T: SurfaceTypeTrait> AsRawSurface for Surface<T> {
    fn raw_surface(&self) -> RawSurface {
        gl_api_dispatch!(self; Self(surface) => surface.raw_surface())
    }
}

impl<T: SurfaceTypeTrait> Sealed for Surface<T> {}

/// A swap interval.
///
/// The default swap interval for your [`Surface`] is platform-dependent. For
/// example, on EGL it is `1` by default, but on GLX it is `0` by default.
///
/// Please note that your application's desired swap interval may be overridden
/// by external, driver-specific configuration, which means that you can't know
/// in advance whether [`crate::surface::GlSurface::swap_buffers`] will block
/// or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapInterval {
    /// When this variant is used calling
    /// `[crate::surface::GlSurface::swap_buffers]` will not block.
    DontWait,

    /// The swap is synchronized to the `n`'th video frame. This is typically
    /// set to `1` to enable vsync and prevent screen tearing.
    Wait(NonZeroU32),
}

/// A platform native pixmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePixmap {
    /// XID of X11 pixmap.
    XlibPixmap(std::os::raw::c_ulong),

    /// XID of X11 pixmap from xcb.
    XcbPixmap(u32),

    /// HBITMAP handle for windows bitmap.
    WindowsPixmap(isize),
}

/// Handle to the raw OpenGL surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSurface {
    /// A pointer to EGLSurface.
    #[cfg(egl_backend)]
    Egl(*const std::ffi::c_void),

    /// GLXDrawable.
    #[cfg(glx_backend)]
    Glx(u64),

    /// HWND
    #[cfg(wgl_backend)]
    Wgl(*const std::ffi::c_void),

    /// Pointer to `NSView`.
    #[cfg(cgl_backend)]
    Cgl(*const std::ffi::c_void),
}

/// The rect that is being used in various surface operations.
///
/// The origin is in the bottom left of the surface.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    /// `X` of the origin.
    pub x: i32,
    /// `Y` of the origin.
    pub y: i32,
    /// Rect width.
    pub width: i32,
    /// Rect height.
    pub height: i32,
}

impl Rect {
    /// Helper to simplify rectangle creation.
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }
}
