use crate::platform_impl;

use glutin_interface::NativeDisplay;
use winit_types::error::{Error, ErrorType};

use std::ops::Range;

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
pub struct Version(pub u8, pub u8);

/// The behavior of the driver when you change the current context.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReleaseBehavior {
    /// Doesn't do anything. Most notably doesn't flush.
    None,

    /// Flushes the context that was previously current as if `glFlush` was
    /// called.
    Flush,
}

impl Default for ReleaseBehavior {
    fn default() -> Self {
        ReleaseBehavior::Flush
    }
}

/// The swap interval.
///
/// If the swap interval is `DontWait`, calling `swap_buffers` will not
/// block.
///
/// When using `Wait(n)` or `AdaptiveWait(n)`, `n` may not equal zero.
///
/// When using `Wait(n)`, the swap is synchronized to the `n`'th video frame.
/// This is typically set to `1` to enable vsync and prevent screen tearing.
///
/// When using `AdaptiveWait(n)`, the swap is synchronized to the `n`th video
/// frame as long as the frame rate is higher than the sync rate. If the frame
/// rate is less than the sync rate synchronization is disabled and
/// `AdaptiveWait(n)` behaves as `DontWait`. This is only supported by WGL/GLX
/// drivers that implement `EXT_swap_control_tear`.
///
/// Please note that your application's desired swap interval may be overridden
/// by external, driver-specific configuration, which means that you can't know
/// in advance whether `swap_buffers`/`swap_buffers_with_damage` will block or
/// not.
#[derive(Debug, Clone, Copy)]
pub enum SwapInterval {
    DontWait,
    Wait(u32),
    AdaptiveWait(u32),
}

impl SwapInterval {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        match self {
            SwapInterval::Wait(n) | SwapInterval::AdaptiveWait(n) if *n == 0 => {
                Err(make_error!(ErrorType::BadApiUsage(
                    "SwapInterval of `0` not allowed. Use `SwapInterval::DontWait`.".to_string()
                )))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SwapIntervalRange {
    DontWait,
    Wait(Range<u32>),
    AdaptiveWait(Range<u32>),
}

impl SwapIntervalRange {
    fn contains(&self, swap_interval: &SwapInterval) -> bool {
        match (self, swap_interval) {
            (SwapIntervalRange::DontWait, SwapInterval::DontWait) => true,
            (SwapIntervalRange::Wait(nr), SwapInterval::Wait(n)) => nr.contains(n),
            (SwapIntervalRange::AdaptiveWait(nr), SwapInterval::AdaptiveWait(n)) => nr.contains(n),
            _ => false,
        }
    }
}

/// Describes a possible format.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ConfigAttribs {
    pub version: (Api, Version),
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
    pub desired_swap_interval: SwapInterval,
    pub swap_interval_ranges: Vec<SwapIntervalRange>,
    pub pbuffer_surface_support: bool,
    pub pixmap_surface_support: bool,
    pub window_surface_support: bool,
    pub surfaceless_support: bool,
    pub release_behavior: ReleaseBehavior,
}

/// Describes a possible format.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ConfigWrapper<T, CA> {
    pub(crate) attribs: CA,
    pub(crate) config: T,
}

pub type Config = ConfigWrapper<platform_impl::Config, ConfigAttribs>;

impl<T: Clone, CA: Clone> ConfigWrapper<&T, &CA> {
    #[inline]
    pub(crate) fn clone_inner(self) -> ConfigWrapper<T, CA> {
        ConfigWrapper {
            config: self.config.clone(),
            attribs: self.attribs.clone(),
        }
    }
}

impl<T, CA> ConfigWrapper<T, CA> {
    /// Turns the `config` parameter into another type by calling a closure.
    #[inline]
    pub(crate) fn map_config<F, T2>(self, f: F) -> ConfigWrapper<T2, CA>
    where
        F: FnOnce(T) -> T2,
    {
        ConfigWrapper {
            config: f(self.config),
            attribs: self.attribs,
        }
    }
}

impl Config {
    #[inline]
    pub fn attribs(&self) -> &ConfigAttribs {
        &self.attribs
    }

    #[inline]
    pub fn set_desired_swap_interval(
        &mut self,
        desired_swap_interval: SwapInterval,
    ) -> Result<(), Error> {
        if self
            .attribs
            .swap_interval_ranges
            .iter()
            .find(|r| r.contains(&desired_swap_interval))
            .is_none()
        {
            return Err(make_error!(ErrorType::BadApiUsage(format!(
                "SwapInterval of {:?} not in ranges {:?}.",
                desired_swap_interval, self.attribs.swap_interval_ranges
            ))));
        }
        self.attribs.desired_swap_interval = desired_swap_interval;
        Ok(())
    }

    /// Turns the `config` parameter into another type.
    #[inline]
    pub(crate) fn as_ref(&self) -> ConfigWrapper<&platform_impl::Config, &ConfigAttribs> {
        ConfigWrapper {
            config: &self.config,
            attribs: &self.attribs,
        }
    }
}

/// Describes how the backend should choose a pixel format.
// TODO: swap method? (swap, copy)
#[derive(Clone, Debug)]
pub struct ConfigBuilder {
    /// Version to try create.
    ///
    /// The default is `(Api::OpenGl, (3, 3))'.
    pub version: (Api, Version),

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

    pub desired_swap_interval: Option<SwapInterval>,
    pub pbuffer_surface_support: bool,
    pub window_surface_support: bool,
    pub pixmap_surface_support: bool,
    pub surfaceless_support: bool,

    pub plat_attr: platform_impl::ConfigPlatformAttributes,
}

impl Default for ConfigBuilder {
    #[inline]
    fn default() -> Self {
        ConfigBuilder {
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
            desired_swap_interval: None,
            pbuffer_surface_support: false,
            surfaceless_support: false,
            pixmap_surface_support: false,
            window_surface_support: true,
            version: (Api::OpenGl, Version(3, 3)),
            release_behavior: Default::default(),
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
    pub fn with_gl(mut self, version: (Api, Version)) -> Self {
        self.version = version;
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
    pub fn with_stereoscopy(mut self, stereo: bool) -> Self {
        self.stereoscopy = stereo;
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

    #[inline]
    pub fn with_desired_swap_interval(
        mut self,
        desired_swap_interval: Option<SwapInterval>,
    ) -> Self {
        self.desired_swap_interval = desired_swap_interval;
        self
    }

    #[inline]
    pub fn with_pbuffer_surface_support(mut self, pbss: bool) -> Self {
        self.pbuffer_surface_support = pbss;
        self
    }

    #[inline]
    pub fn with_pixmap_surface_support(mut self, pss: bool) -> Self {
        self.pixmap_surface_support = pss;
        self
    }

    #[inline]
    pub fn with_window_surface_support(mut self, wss: bool) -> Self {
        self.window_surface_support = wss;
        self
    }

    #[inline]
    pub fn with_surfaceless_support(mut self, ss: bool) -> Self {
        self.surfaceless_support = ss;
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
    pub fn with_double_buffer(mut self, db: Option<bool>) -> Self {
        self.double_buffer = db;
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
    pub fn with_hardware_acceleration(mut self, accel: Option<bool>) -> Self {
        self.hardware_accelerated = accel;
        self
    }

    #[inline]
    pub fn build<ND: NativeDisplay>(self, nd: &ND) -> Result<Vec<Config>, Error> {
        self.desired_swap_interval
            .unwrap_or(SwapInterval::DontWait)
            .validate()?;
        let configs = platform_impl::Config::new(&self, nd)?;
        assert!(!configs.is_empty());

        Ok(configs
            .into_iter()
            .map(|(attribs, config)| Config { attribs, config })
            .collect())
    }
}