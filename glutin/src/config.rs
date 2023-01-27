//! Api config picking and creating utils.
#![allow(unreachable_patterns)]

use std::num::NonZeroU32;

use bitflags::bitflags;
use raw_window_handle::RawWindowHandle;

use crate::display::{Display, GetGlDisplay};
use crate::private::{gl_api_dispatch, Sealed};

#[cfg(x11_platform)]
use crate::platform::x11::{X11GlConfigExt, X11VisualInfo};

#[cfg(cgl_backend)]
use crate::api::cgl::config::Config as CglConfig;
#[cfg(egl_backend)]
use crate::api::egl::config::Config as EglConfig;
#[cfg(glx_backend)]
use crate::api::glx::config::Config as GlxConfig;
#[cfg(wgl_backend)]
use crate::api::wgl::config::Config as WglConfig;

/// The trait to group all common config option.
pub trait GlConfig: Sealed {
    /// The type of the underlying color buffer.
    ///
    /// `None` is returned when the format can not be identified.
    fn color_buffer_type(&self) -> Option<ColorBufferType>;

    /// Whether the config uses floating pixels.
    fn float_pixels(&self) -> bool;

    /// The size of the alpha.
    fn alpha_size(&self) -> u8;

    /// The size of the depth buffer.
    fn depth_size(&self) -> u8;

    /// The size of the stencil buffer.
    fn stencil_size(&self) -> u8;

    /// The number of samples in multisample buffer.
    ///
    /// Zero would mean that there're no samples.
    fn num_samples(&self) -> u8;

    /// Whether the config supports creating srgb capable [`Surface`].
    ///
    /// [`Surface`]: crate::surface::Surface
    fn srgb_capable(&self) -> bool;

    /// Whether the config supports creating transparent surfaces.
    ///
    /// This function will return `None` when the property couldn't be
    /// identified, in that case transparent window could still work.
    fn supports_transparency(&self) -> Option<bool>;

    /// Whether the config is hardware accelerated.
    ///
    /// The meaning of this may vary from system to system. On some it could
    /// mean that you're using a software backend renderer, it could mean
    /// that you're using not the fastest available GPU, like in laptops
    /// with hybrid graphics.
    fn hardware_accelerated(&self) -> bool;

    /// The type of the surfaces that can be created with this config.
    fn config_surface_types(&self) -> ConfigSurfaceTypes;

    /// The [`crate::config::Api`] supported by the configuration.
    fn api(&self) -> Api;
}

/// The trait to
pub trait GetGlConfig: Sealed {
    /// The config type.
    type Target: GlConfig;

    /// Get the GL config used to create a particular GL object.
    fn config(&self) -> Self::Target;
}

/// Get the raw config.
pub trait AsRawConfig {
    /// Obtain the [`RawConfig`] of the underlying Api.
    fn raw_config(&self) -> RawConfig;
}

/// Builder for the [`ConfigTemplate`].
#[derive(Debug, Default, Clone)]
pub struct ConfigTemplateBuilder {
    template: ConfigTemplate,
}

impl ConfigTemplateBuilder {
    /// Create a new configuration template builder.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Number of alpha bits in the color buffer.
    ///
    /// By default `8` is requested.
    #[inline]
    pub fn with_alpha_size(mut self, alpha_size: u8) -> Self {
        self.template.alpha_size = alpha_size;
        self
    }

    /// Wether the floating pixel formats should be used.
    ///
    /// By default `false` is requested.
    #[inline]
    pub fn with_float_pixels(mut self, float_pixels: bool) -> Self {
        self.template.float_pixels = float_pixels;
        self
    }

    /// Number of bits in the stencil buffer.
    ///
    /// By default `0` is requested.
    #[inline]
    pub fn with_stencil_size(mut self, stencil_size: u8) -> Self {
        self.template.stencil_size = stencil_size;
        self
    }

    /// Number of bits in the depth buffer.
    ///
    /// By default `0` is requested.
    #[inline]
    pub fn with_depth_size(mut self, depth_size: u8) -> Self {
        self.template.depth_size = depth_size;
        self
    }

    /// Whether multisampling configurations should be picked. The `num_samples`
    /// must be a power of two.
    ///
    /// By default multisampling is not specified.
    #[inline]
    pub fn with_multisampling(mut self, num_samples: u8) -> Self {
        debug_assert!(num_samples.is_power_of_two());
        self.template.num_samples = Some(num_samples);
        self
    }

    /// The types of the surfaces that must be supported by the configuration.
    ///
    /// By default only the `WINDOW` bit is set.
    #[inline]
    pub fn with_surface_type(mut self, config_surface_types: ConfigSurfaceTypes) -> Self {
        self.template.config_surface_types = config_surface_types;
        self
    }

    /// The type of the color buffer.
    ///
    /// By default `RGB` buffer with all components sizes of `8` is requested.
    #[inline]
    pub fn with_buffer_type(mut self, color_buffer_type: ColorBufferType) -> Self {
        self.template.color_buffer_type = color_buffer_type;
        self
    }

    /// The set of apis that are supported by configuration.
    ///
    /// By default api isn't specified when requesting the configuration.
    #[inline]
    pub fn with_api(mut self, api: Api) -> Self {
        self.template.api = Some(api);
        self
    }

    /// Wether the stereo pairs should be present.
    ///
    /// By default it isn't specified.
    #[inline]
    pub fn with_stereoscopy(mut self, stereoscopy: Option<bool>) -> Self {
        self.template.stereoscopy = stereoscopy;
        self
    }

    /// Wether the single buffer should be used.
    ///
    /// By default `false` is requested.
    #[inline]
    pub fn with_single_buffering(mut self, single_buffering: bool) -> Self {
        self.template.single_buffering = single_buffering;
        self
    }

    /// Wether the configuration should support transparency.
    ///
    /// The default is `false`.
    ///
    /// # Api-specific
    ///
    /// EGL on X11 doesn't provide a way to create a transparent surface at the
    /// time of writing. Use GLX for that instead.
    #[inline]
    pub fn with_transparency(mut self, transparency: bool) -> Self {
        self.template.transparency = transparency;
        self
    }

    /// With the maximum sizes of pbuffer.
    #[inline]
    pub fn with_pbuffer_sizes(mut self, width: NonZeroU32, height: NonZeroU32) -> Self {
        self.template.max_pbuffer_width = Some(width.into());
        self.template.max_pbuffer_height = Some(height.into());
        self
    }

    /// Wether the configuration should prefer hardware accelerated formats or
    /// not.
    ///
    /// By default hardware acceleration or its absence is not requested.
    pub fn prefer_hardware_accelerated(mut self, hardware_accerelated: Option<bool>) -> Self {
        self.template.hardware_accelerated = hardware_accerelated;
        self
    }

    /// Request config that can render to a particular native window.
    ///
    /// # Platform-specific
    ///
    /// This will use native window when matching the config to get the best one
    /// suitable for rendering into that window.
    ///
    /// When using WGL it's the most reliable way to get a working
    /// configuration. With GLX it'll use the visual passed in
    /// `native_window` to match the config.
    pub fn compatible_with_native_window(mut self, native_window: RawWindowHandle) -> Self {
        self.template.native_window = Some(native_window);
        self
    }

    /// With supported swap intervals.
    ///
    /// By default the value isn't specified.
    ////
    /// # Api-specific
    ///
    /// Only supported with `EGL`.
    #[inline]
    pub fn with_swap_interval(
        mut self,
        min_swap_interval: Option<u16>,
        max_swap_interval: Option<u16>,
    ) -> Self {
        self.template.min_swap_interval = min_swap_interval;
        self.template.max_swap_interval = max_swap_interval;
        self
    }

    /// Build the template to match the configs against.
    #[must_use]
    pub fn build(self) -> ConfigTemplate {
        self.template
    }
}

/// The context configuration template that is used to find desired config.
#[derive(Debug, Clone)]
pub struct ConfigTemplate {
    /// The type of the backing buffer and ancillary buffers.
    pub(crate) color_buffer_type: ColorBufferType,

    /// Bits of alpha in the color buffer.
    pub(crate) alpha_size: u8,

    /// Bits of depth in the depth buffer.
    pub(crate) depth_size: u8,

    /// Bits of stencil in the stencil buffer.
    pub(crate) stencil_size: u8,

    /// The amount of samples in multisample buffer.
    pub(crate) num_samples: Option<u8>,

    /// The minimum swap interval supported by the configuration.
    pub(crate) min_swap_interval: Option<u16>,

    /// The maximum swap interval supported by the configuration.
    pub(crate) max_swap_interval: Option<u16>,

    /// The types of the surfaces supported by the configuration.
    pub(crate) config_surface_types: ConfigSurfaceTypes,

    /// The rendering Api's supported by the configuration.
    pub(crate) api: Option<Api>,

    /// The config should support transparency.
    pub(crate) transparency: bool,

    /// The config should prefer single buffering.
    pub(crate) single_buffering: bool,

    /// The config supports stereoscopy.
    pub(crate) stereoscopy: Option<bool>,

    /// The config uses floating pixels.
    pub(crate) float_pixels: bool,

    /// The maximum width of the pbuffer.
    pub(crate) max_pbuffer_width: Option<u32>,

    /// The config should prefer hardware accelerated formats.
    pub(crate) hardware_accelerated: Option<bool>,

    /// The maximum height of the pbuffer.
    pub(crate) max_pbuffer_height: Option<u32>,

    /// The native window config should support rendering into.
    pub(crate) native_window: Option<RawWindowHandle>,
}

impl Default for ConfigTemplate {
    fn default() -> Self {
        ConfigTemplate {
            color_buffer_type: ColorBufferType::Rgb { r_size: 8, g_size: 8, b_size: 8 },

            alpha_size: 8,

            depth_size: 24,

            stencil_size: 8,

            num_samples: None,

            transparency: false,

            stereoscopy: None,

            min_swap_interval: None,

            max_swap_interval: None,

            single_buffering: false,

            float_pixels: false,

            config_surface_types: ConfigSurfaceTypes::WINDOW,

            max_pbuffer_width: None,
            max_pbuffer_height: None,

            native_window: None,
            hardware_accelerated: None,

            api: None,
        }
    }
}

bitflags! {
    /// The types of the surface supported by the config.
    pub struct ConfigSurfaceTypes: u8 {
        /// Context must support windows.
        const WINDOW  = 0b00000001;

        /// Context must support pixmaps.
        const PIXMAP  = 0b00000010;

        /// Context must support pbuffers.
        const PBUFFER = 0b00000100;
    }
}

bitflags! {
    /// The Api supported by the config.
    pub struct Api : u8 {
        /// Context supports OpenGL API.
        const OPENGL = 0b00000001;

        /// Context supports OpenGL ES 1 API.
        const GLES1  = 0b00000010;

        /// Context supports OpenGL ES 2 API.
        const GLES2  = 0b00000100;

        /// Context supports OpenGL ES 3 API.
        const GLES3  = 0b00001000;
    }
}

/// The buffer type baked by the config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBufferType {
    /// The backing buffer is using RGB format.
    Rgb {
        /// Size of the red component in bits.
        r_size: u8,
        /// Size of the green component in bits.
        g_size: u8,
        /// Size of the blue component in bits.
        b_size: u8,
    },

    /// The backing buffer is using Luminance.
    Luminance(u8),
}

/// The GL configuration used to create [`Surface`] and [`Context`] in a cross
/// platform way.
///
/// The config could be accessed from any thread.
///
/// ```no_run
/// fn test_send<T: Send>() {}
/// fn test_sync<T: Sync>() {}
/// test_send::<glutin::config::Config>();
/// test_sync::<glutin::config::Config>();
/// ```
///
/// [`Surface`]: crate::surface::Surface
/// [`Context`]: crate::context::NotCurrentContext
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Config {
    /// The EGL config.
    #[cfg(egl_backend)]
    Egl(EglConfig),

    /// The GLX config.
    #[cfg(glx_backend)]
    Glx(GlxConfig),

    /// The WGL config.
    #[cfg(wgl_backend)]
    Wgl(WglConfig),

    /// The CGL config.
    #[cfg(cgl_backend)]
    Cgl(CglConfig),
}

impl GlConfig for Config {
    fn color_buffer_type(&self) -> Option<ColorBufferType> {
        gl_api_dispatch!(self; Self(config) => config.color_buffer_type())
    }

    fn float_pixels(&self) -> bool {
        gl_api_dispatch!(self; Self(config) => config.float_pixels())
    }

    fn alpha_size(&self) -> u8 {
        gl_api_dispatch!(self; Self(config) => config.alpha_size())
    }

    fn depth_size(&self) -> u8 {
        gl_api_dispatch!(self; Self(config) => config.depth_size())
    }

    fn stencil_size(&self) -> u8 {
        gl_api_dispatch!(self; Self(config) => config.stencil_size())
    }

    fn num_samples(&self) -> u8 {
        gl_api_dispatch!(self; Self(config) => config.num_samples())
    }

    fn srgb_capable(&self) -> bool {
        gl_api_dispatch!(self; Self(config) => config.srgb_capable())
    }

    fn config_surface_types(&self) -> ConfigSurfaceTypes {
        gl_api_dispatch!(self; Self(config) => config.config_surface_types())
    }

    fn hardware_accelerated(&self) -> bool {
        gl_api_dispatch!(self; Self(config) => config.hardware_accelerated())
    }

    fn supports_transparency(&self) -> Option<bool> {
        gl_api_dispatch!(self; Self(config) => config.supports_transparency())
    }

    fn api(&self) -> Api {
        gl_api_dispatch!(self; Self(config) => config.api())
    }
}

impl GetGlDisplay for Config {
    type Target = Display;

    fn display(&self) -> Self::Target {
        gl_api_dispatch!(self; Self(config) => config.display(); as Display)
    }
}

#[cfg(x11_platform)]
impl X11GlConfigExt for Config {
    fn x11_visual(&self) -> Option<X11VisualInfo> {
        gl_api_dispatch!(self; Self(config) => config.x11_visual())
    }
}

impl Sealed for Config {}

/// Raw config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawConfig {
    /// Raw EGL config.
    #[cfg(egl_backend)]
    Egl(*const std::ffi::c_void),

    /// Raw GLX config.
    #[cfg(glx_backend)]
    Glx(*const std::ffi::c_void),

    /// WGL pixel format index.
    #[cfg(wgl_backend)]
    Wgl(i32),

    /// NSOpenGLPixelFormat.
    #[cfg(cgl_backend)]
    Cgl(*const std::ffi::c_void),
}

impl AsRawConfig for Config {
    fn raw_config(&self) -> RawConfig {
        gl_api_dispatch!(self; Self(config) => config.raw_config())
    }
}
