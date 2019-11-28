use super::*;
use crate::display::Display;

/// All APIs related to OpenGL that you can possibly get while using glutin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    /// The classical OpenGL. Available on Windows, Unix operating systems,
    /// OS/X.
    OpenGl,
    /// OpenGL embedded system. Available on Unix operating systems, Android.
    OpenGlEs,
    /// OpenGL for the web. Very similar to OpenGL ES.
    WebGl,
}

#[derive(Debug, Copy, Clone)]
pub struct GlVersion(pub u8, pub u8);

/// Describes the OpenGL API and version that are being requested when a context
/// is created.
#[derive(Debug, Copy, Clone)]
pub enum GlRequest {
    /// Request the latest version of the "best" API of this platform.
    ///
    /// On desktop, will try OpenGL.
    Latest,

    /// Request a specific version of a specific API.
    ///
    /// Example: `GlRequest::Specific(Api::OpenGl, (3, 3))`.
    Specific(Api, GlVersion),

    /// If OpenGL is available, create an OpenGL [`Context`] with the specified
    /// `opengl_version`. Else if OpenGL ES or WebGL is available, create a
    /// context with the specified `opengles_version`.
    ///
    /// [`Context`]: struct.Context.html
    GlThenGles {
        /// The version to use for OpenGL.
        opengl_version: GlVersion,
        /// The version to use for OpenGL ES.
        opengles_version: GlVersion,
    },
}

impl Default for GlRequest {
    fn default() -> Self {
        GlRequest::Latest
    }
}

/// The minimum core profile GL context. Useful for getting the minimum
/// required GL version while still running on OSX, which often forbids
/// the compatibility profile features.
pub static GL_CORE: GlRequest =
    GlRequest::Specific(Api::OpenGl, GlVersion(3, 2));

/// The behavior of the driver when you change the current context.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReleaseBehavior {
    /// Doesn't do anything. Most notably doesn't flush.
    None,

    /// Flushes the context that was previously current as if `glFlush` was
    /// called.
    Flush,
}

/// Describes a possible format.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ConfigAttribs {
    pub version: Option<GlVersion>,
    pub api: Api,

    pub hardware_accelerated: bool,
    /// The number of color bits. Does not include alpha bits.
    pub color_bits: u8,
    pub alpha_bits: u8,
    pub depth_bits: u8,
    pub stencil_bits: u8,
    pub stereoscopy: bool,
    pub double_buffer: bool,
    /// `None` if multisampling is disabled, otherwise `Some(N)` where `N` is
    /// the multisampling level.
    pub multisampling: Option<u16>,
    pub srgb: bool,
    pub vsync: bool,
    pub pbuffer_support: bool,
    pub window_surface_support: bool,
}

/// Describes a possible format.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ConfigWrapper<T> {
    pub attribs: ConfigAttribs,
    pub(crate) config: T,
}

pub type Config = ConfigWrapper<platform_impl::Config>;

impl<T> ConfigWrapper<T> {
    /// Turns the `config` parameter into another type by calling a closure.
    #[inline]
    pub(crate) fn map_config<F, T2>(self, f: F) -> ConfigWrapper<T2>
    where
        F: FnOnce(T) -> T2,
    {
        ConfigWrapper {
            config: f(self.config),
            attribs: self.attribs,
        }
    }

    /// Turns the `config` parameter into another type.
    #[inline]
    pub(crate) fn with_config<T2>(&self, config: T2) -> ConfigWrapper<T2> {
        ConfigWrapper {
            config,
            attribs: self.attribs.clone(),
        }
    }
}

/// Describes how the backend should choose a pixel format.
// TODO: swap method? (swap, copy)
#[derive(Clone, Debug)]
pub struct ConfigBuilder {
    /// Version to try create. See [`GlRequest`] for more infos.
    ///
    /// The default is [`Latest`].
    ///
    /// [`Latest`]: enum.GlRequest.html#variant.Latest
    /// [`GlRequest`]: enum.GlRequest.html
    pub version: GlRequest,

    /// If true, only hardware-accelerated formats will be considered. If
    /// false, only software renderers. `None` means "don't care". Default
    /// is `Some(true)`.
    pub hardware_accelerated: Option<bool>,

    /// Minimum number of bits for the color buffer, excluding alpha. `None`
    /// means "don't care". The default is `Some(24)`.
    pub color_bits: Option<u8>,

    /// If true, the color buffer must be in a floating point format. Default
    /// is `false`.
    ///
    /// Using floating points allows you to write values outside of the `[0.0,
    /// 1.0]` range.
    pub float_color_buffer: bool,

    /// Minimum number of bits for the alpha in the color buffer. `None` means
    /// "don't care". The default is `Some(8)`.
    pub alpha_bits: Option<u8>,

    /// Minimum number of bits for the depth buffer. `None` means "don't care".
    /// The default value is `Some(24)`.
    pub depth_bits: Option<u8>,

    /// Minimum number of stencil bits. `None` means "don't care".
    /// The default value is `Some(8)`.
    pub stencil_bits: Option<u8>,

    /// If true, only double-buffered formats will be considered. If false,
    /// only single-buffer formats. `None` means "don't care". The default
    /// is `Some(true)`.
    pub double_buffer: Option<bool>,

    /// Contains the minimum number of samples per pixel in the color, depth
    /// and stencil buffers. `None` means "don't care". Default is `None`.
    /// A value of `Some(0)` indicates that multisampling must not be enabled.
    pub multisampling: Option<u16>,

    /// If true, only stereoscopic formats will be considered. If false, only
    /// non-stereoscopic formats. The default is `false`.
    pub stereoscopy: bool,

    /// If sRGB-capable formats will be considered. If `None`, don't care.
    /// The default is `None`.
    pub srgb: Option<bool>,

    /// The behavior when changing the current context. Default is `Flush`.
    pub release_behavior: ReleaseBehavior,

    /// Whether to use vsync. If vsync is enabled, calling `swap_buffers` will
    /// block until the screen refreshes. This is typically used to prevent
    /// screen tearing.
    ///
    /// The default is `None`.
    pub vsync: Option<bool>,

    /// FIXME: missing docs
    pub pbuffer_support: bool,
    /// FIXME: missing docs
    pub window_surface_support: bool,

    pub plat_attr: platform_impl::SurfacePlatformAttributes,
}

impl Default for ConfigBuilder {
    #[inline]
    fn default() -> Self {
        ConfigBuilder {
            version: GlRequest::Latest,
            hardware_accelerated: Some(true),
            color_bits: Some(24),
            float_color_buffer: false,
            alpha_bits: Some(8),
            depth_bits: Some(24),
            stencil_bits: Some(8),
            double_buffer: None,
            multisampling: None,
            stereoscopy: false,
            srgb: None,
            vsync: None,
            pbuffer_support: false,
            window_surface_support: true,
            release_behavior: ReleaseBehavior::Flush,
            plat_attr: Default::default(),
        }
    }
}

impl ConfigBuilder {
    fn new() -> Self {
        Default::default()
    }

    /// Sets how the backend should choose the OpenGL API and version.
    #[inline]
    pub fn with_gl(mut self, request: GlRequest) -> Self {
        self.version = request;
        self
    }

    /// Sets the multisampling level to request. A value of `0` indicates that
    /// multisampling must not be enabled.
    ///
    /// # Panic
    ///
    /// Will panic if `samples` is not a power of two.
    #[inline]
    pub fn with_multisampling(mut self, samples: u16) -> Self {
        self.multisampling = match samples {
            0 => None,
            _ => {
                assert!(samples.is_power_of_two());
                Some(samples)
            }
        };
        self
    }

    /// Sets the number of bits in the depth buffer.
    #[inline]
    pub fn with_depth_buffer(mut self, bits: u8) -> Self {
        self.depth_bits = Some(bits);
        self
    }

    /// Sets the number of bits in the stencil buffer.
    #[inline]
    pub fn with_stencil_buffer(mut self, bits: u8) -> Self {
        self.stencil_bits = Some(bits);
        self
    }

    /// Sets the number of bits in the color buffer.
    #[inline]
    pub fn with_pixel_format(mut self, color_bits: u8, alpha_bits: u8) -> Self {
        self.color_bits = Some(color_bits);
        self.alpha_bits = Some(alpha_bits);
        self
    }

    /// Request the backend to be stereoscopic.
    #[inline]
    pub fn with_stereoscopy(mut self) -> Self {
        self.stereoscopy = true;
        self
    }

    /// Sets whether sRGB should be enabled on the window.
    ///
    /// The default value is `None`.
    #[inline]
    pub fn with_srgb(mut self, srgb: Option<bool>) -> Self {
        self.srgb = srgb;
        self
    }

    /// Requests that the window has vsync enabled.
    ///
    /// By default, vsync is not enabled.
    #[inline]
    pub fn with_vsync(mut self, vsync: Option<bool>) -> Self {
        self.vsync = vsync;
        self
    }

    #[inline]
    pub fn with_pbuffer_support(mut self, pbuffer_support: bool) -> Self {
        self.pbuffer_support = pbuffer_support;
        self
    }

    #[inline]
    pub fn with_window_surface_support(
        mut self,
        window_surface_support: bool,
    ) -> Self {
        self.window_surface_support = window_surface_support;
        self
    }

    /// Sets whether double buffering should be enabled.
    ///
    /// The default value is `None`.
    ///
    /// ## Platform-specific
    ///
    /// This option will be taken into account on the following platforms:
    ///
    ///   * MacOS
    ///   * Unix operating systems using GLX with X
    ///   * Windows using WGL
    #[inline]
    pub fn with_double_buffer(mut self, double_buffer: Option<bool>) -> Self {
        self.double_buffer = double_buffer;
        self
    }

    /// Sets whether hardware acceleration is required.
    ///
    /// The default value is `Some(true)`
    ///
    /// ## Platform-specific
    ///
    /// This option will be taken into account on the following platforms:
    ///
    ///   * MacOS
    ///   * Unix operating systems using EGL with either X or Wayland
    ///   * Windows using EGL or WGL
    ///   * Android using EGL
    #[inline]
    pub fn with_hardware_acceleration(
        mut self,
        acceleration: Option<bool>,
    ) -> Self {
        self.hardware_accelerated = acceleration;
        self
    }

    #[inline]
    pub fn build(self, el: &Display) -> Result<Config, CreationError> {
        platform_impl::Config::new(el, self)
            .map(|(attribs, config)| Config { attribs, config })
    }
}

