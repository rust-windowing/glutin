//! You can use a [`ConfigsFinder`] to get a selection of [`Config`eration]s
//! that match your criteria. Among many things, you must specify in advance
//! what types of [`Surface`]s you're going to use the [`Config`eration] with.
//!
//! Once a [`Config`eration] is made, you can also modify its
//! [`desired_swap_interval`]. Please refer to [`ConfigAttribs`] for more info.
//!
//! **WARNING:** Glutin clients should use the [`Config`] type in their code, not
//! [`ConfigWrapper`]. If I had a choice, I'd hide that type, but alas, due to
//! limitations in rustdoc, I cannot. Unfortunately, all of [`Config`]'s methods
//! are only visible on [`ConfigWrapper`].
//!
//! [`ConfigAttribs`]: crate::config::ConfigAttribs
//! [`Config`eration]: crate::config::ConfigWrapper
//! [`Config`]: crate::config::Config
//! [`ConfigWrapper`]: crate::config::ConfigWrapper
//! [`desired_swap_interval`]: crate::config::ConfigAttribs::desired_swap_interval
//! [`Surface`]: crate::surface::Surface

use crate::platform_impl;

use glutin_interface::NativeDisplay;
use winit_types::error::{Error, ErrorType};

use std::ops::Range;

/// All OpenGL APIs that you can get while using glutin.
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

/// The OpenGL version you want. Major then Minor, so `Version(3, 2)` equals
/// OpenGL 3.2.
#[derive(Debug, Copy, Clone)]
pub struct Version(pub u8, pub u8);

/// A swap interval.
///
/// Please note that your application's desired swap interval may be overridden
/// by external, driver-specific configuration, which means that you can't know
/// in advance whether [`swap_buffers`]/[`swap_buffers_with_damage`] will block or
/// not.
///
/// [`swap_buffers`]: crate::surface::Surface::swap_buffers
/// [`swap_buffers_with_damage`]: crate::surface::Surface::swap_buffers_with_damage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapInterval {
    /// If the swap interval is `DontWait`, calling `swap_buffers` will not
    /// block.
    DontWait,

    /// When using `Wait(n)`, `n` may not equal zero.
    ///
    /// The swap is synchronized to the `n`'th video frame. This is typically
    /// set to `1` to enable vsync and prevent screen tearing.
    ///
    Wait(u32),

    /// When using `AdaptiveWait(n)`, `n` may not equal zero.
    ///
    /// The swap is synchronized to the `n`th video frame as long as the frame
    /// rate is higher than the sync rate.
    ///
    /// If the frame rate is less than the sync ratem synchronization is disabled
    /// and `AdaptiveWait(n)` behaves as `DontWait`. This is only supported by
    /// WGL/GLX drivers that implement `EXT_swap_control_tear`.
    AdaptiveWait(u32),
}

impl SwapInterval {
    #[inline]
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

/// A range of swap intervals
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapIntervalRange {
    /// [`DontWait`](crate::config::SwapInterval::DontWait) is in range.
    DontWait,
    /// [`Wait(n)`](crate::config::SwapInterval::Wait) is in range, as long as
    /// `n` is in this `Range<u32>`.
    Wait(Range<u32>),
    /// [`AdaptiveWait(n)`](crate::config::SwapInterval::AdaptiveWait) is in
    /// range, as long as `n` is in this `Range<u32>`.
    AdaptiveWait(Range<u32>),
}

impl SwapIntervalRange {
    /// Returns `true` if the [`SwapInterval`](crate::config::SwapInterval) is
    /// in range of this `SwapIntervalRange`, else `false`.
    #[inline]
    fn contains(&self, swap_interval: &SwapInterval) -> bool {
        match (self, swap_interval) {
            (SwapIntervalRange::DontWait, SwapInterval::DontWait) => true,
            (SwapIntervalRange::Wait(nr), SwapInterval::Wait(n)) => nr.contains(n),
            (SwapIntervalRange::AdaptiveWait(nr), SwapInterval::AdaptiveWait(n)) => nr.contains(n),
            _ => false,
        }
    }
}

/// Describes the attributes of a possible [`Config`eration]. Immutably accessed
/// via [`Config`eration]'s [`attribs`] function.
///
/// After the creation of the [`Config`eration], the [`desired_swap_interval`]
/// can be modified into any other [`SwapInterval`] as long as it is within range
/// of any one of the [`SwapIntervalRange`]s in [`swap_interval_ranges`] via
/// [`Config`eration]'s [`set_desired_swap_interval`] function.
///
/// Please refer to [`ConfigsFinder`]'s methods for details on what each parameter
/// is for.
///
/// [`Config`eration]: crate::config::ConfigWrapper
/// [`SwapInterval`]: crate::config::SwapInterval
/// [`SwapIntervalRange`]: crate::config::SwapIntervalRange
/// [`attribs`]: crate::config::ConfigWrapper::attribs()
/// [`set_desired_swap_interval`]: crate::config::ConfigWrapper::set_desired_swap_interval()
/// [`desired_swap_interval`]: crate::config::ConfigAttribs::desired_swap_interval
/// [`swap_interval_ranges`]: crate::config::ConfigAttribs::swap_interval_ranges
/// [`ConfigsFinder`]: crate::config::ConfigsFinder
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ConfigAttribs {
    pub desired_swap_interval: SwapInterval,
    pub swap_interval_ranges: Vec<SwapIntervalRange>,
    pub version: (Api, Version),
    pub hardware_accelerated: bool,
    pub color_bits: u8,
    pub alpha_bits: u8,
    pub depth_bits: u8,
    pub stencil_bits: u8,
    pub stereoscopy: bool,
    pub double_buffer: bool,
    pub multisampling: Option<u16>,
    pub srgb: bool,
    pub pbuffer_surface_support: bool,
    pub pixmap_surface_support: bool,
    pub window_surface_support: bool,
    pub surfaceless_support: bool,
}

/// A type that contains the [`ConfigAttribs`] along side with the native api's
/// config type and (depending on the native API) possibly the connection to the
/// native API..
///
/// Please refer to [`ConfigAttribs`] for more information.
///
/// **WARNING:** Glutin clients should use the [`Config`] type in their code,
/// not this type. If I had a choice, I'd hide this type, but alas, due to
/// limitations in rustdoc, I cannot.
///
/// [`ConfigAttribs`]: crate::config::ConfigAttribs
/// [`Config`]: crate::config::Config
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ConfigWrapper<T, CA> {
    pub(crate) attribs: CA,
    pub(crate) config: T,
}

/// A simple type alias for [`ConfigWrapper`]. Glutin clients should use this
/// type in their code, not [`ConfigWrapper`]. If I had a choice, I'd hide
/// [`ConfigWrapper`], but alas, due to limitations in rustdoc, I cannot.
///
/// [`ConfigWrapper`]: crate::config::ConfigWrapper
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

impl ConfigWrapper<platform_impl::Config, ConfigAttribs> {
    /// Provides immutable access to [`Config`eration]'s [`ConfigAttribs`].
    ///
    /// Please refer to [`ConfigAttribs`] for more information.
    ///
    /// [`Config`eration]: crate::config::ConfigWrapper
    /// [`ConfigAttribs`]: crate::config::ConfigAttribs
    #[inline]
    pub fn attribs(&self) -> &ConfigAttribs {
        &self.attribs
    }

    /// Changes the [`Config`eration]'s [`desired_swap_interval`].
    ///
    /// Please refer to [`ConfigAttribs`] for more information.
    ///
    /// [`desired_swap_interval`]: crate::config::ConfigAttribs::desired_swap_interval
    /// [`Config`eration]: crate::config::ConfigWrapper
    /// [`ConfigAttribs`]: crate::config::ConfigAttribs
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
}

impl Config {
    /// Turns the `config` parameter into another type.
    #[inline]
    pub(crate) fn as_ref(&self) -> ConfigWrapper<&platform_impl::Config, &ConfigAttribs> {
        ConfigWrapper {
            config: &self.config,
            attribs: &self.attribs,
        }
    }
}

/// A type for finding one or more [`Config`eration]s that meet a certain
/// criteria.
///
/// For details on what each member controls, please scroll through the
/// [methods] bellow.
///
/// For what the defaults currently are, please refer to our [defaults
/// implementation].
///
/// [`Config`eration]: crate::config::ConfigWrapper
/// [methods]: ./struct.ConfigsFinder.html#methods
/// [defaults implementation]: ./struct.ConfigsFinder.html#impl-Default
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct ConfigsFinder {
    pub version: (Api, Version),
    pub hardware_accelerated: Option<bool>,
    pub color_bits: Option<u8>,
    pub float_color_buffer: Option<bool>,
    pub alpha_bits: Option<u8>,
    pub depth_bits: Option<u8>,
    pub stencil_bits: Option<u8>,
    pub double_buffer: Option<bool>,
    pub multisampling: Option<u16>,
    pub stereoscopy: bool,
    pub srgb: Option<bool>,
    pub desired_swap_interval: Option<SwapInterval>,
    pub pbuffer_surface_support: bool,
    pub window_surface_support: bool,
    pub pixmap_surface_support: bool,
    pub surfaceless_support: bool,
    pub plat_attr: platform_impl::ConfigPlatformAttributes,
}

impl Default for ConfigsFinder {
    #[inline]
    fn default() -> Self {
        ConfigsFinder {
            hardware_accelerated: Some(true),
            color_bits: Some(24),
            // FIXME EGL_EXT_pixel_format_float
            float_color_buffer: None,
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
            plat_attr: Default::default(),
        }
    }
}

impl ConfigsFinder {
    /// Makes a `ConfigsFinder` with the default options.
    #[inline]
    fn new() -> Self {
        Default::default()
    }

    /// Sets which OpenGL API and version to use.
    #[inline]
    pub fn with_gl(mut self, version: (Api, Version)) -> Self {
        self.version = version;
        self
    }

    /// If `true`, the color buffer must use a floating point format. `false`
    /// means it must not use a floating point format. `None` means "don't
    /// care".
    ///
    /// Using floating points allows you to write values outside of the `[0.0,
    /// 1.0]` range.
    #[inline]
    pub fn with_float_color_buffer(mut self, float_color_buffer: Option<bool>) -> Self {
        self.float_color_buffer = float_color_buffer;
        self
    }

    /// Contains the minimum number of samples per pixel in the color, depth
    /// and stencil buffers. `None` means "don't care".
    /// A value of `None` indicates that multisampling must not be enabled.
    ///
    /// # Panic
    ///
    /// Will panic if `samples` is not a power of two.
    #[inline]
    pub fn with_multisampling(mut self, samples: Option<u16>) -> Self {
        assert!(samples.unwrap_or(2).is_power_of_two());
        self.multisampling = samples;
        self
    }

    /// Sets the number of bits in the depth buffer. `None` means "don't care".
    #[inline]
    pub fn with_depth_buffer(mut self, bits: Option<u8>) -> Self {
        self.depth_bits = bits;
        self
    }

    /// Sets the number of bits in the stencil buffer. `None` means "don't care".
    #[inline]
    pub fn with_stencil_buffer(mut self, bits: Option<u8>) -> Self {
        self.stencil_bits = bits;
        self
    }

    /// Sets the number of bits in the color buffer. `None` means "don't care".
    #[inline]
    pub fn with_pixel_format(mut self, color_bits: Option<u8>, alpha_bits: Option<u8>) -> Self {
        self.color_bits = color_bits;
        self.alpha_bits = alpha_bits;
        self
    }

    /// If true, only stereoscopic formats will be considered. If false, only
    /// non-stereoscopic formats.
    #[inline]
    pub fn with_stereoscopy(mut self, stereo: bool) -> Self {
        self.stereoscopy = stereo;
        self
    }

    /// If sRGB-capable formats will be considered. If `None`, don't care.
    #[inline]
    pub fn with_srgb(mut self, srgb: Option<bool>) -> Self {
        self.srgb = srgb;
        self
    }

    /// Sets the desired [`SwapInterval`]. Please refer to [`SwapInterval`] for
    /// more details.
    ///
    /// [`SwapInterval`]: crate::config::SwapInterval
    #[inline]
    pub fn with_desired_swap_interval(
        mut self,
        desired_swap_interval: Option<SwapInterval>,
    ) -> Self {
        self.desired_swap_interval = desired_swap_interval;
        self
    }

    /// Whether or not the [`Config`eration]s should support [`PBuffer`]s.
    ///
    /// [`Config`eration]: crate::config::ConfigWrapper
    /// [`PBuffer`]: crate::surface::PBuffer
    #[inline]
    pub fn with_pbuffer_surface_support(mut self, pbss: bool) -> Self {
        self.pbuffer_surface_support = pbss;
        self
    }

    /// Whether or not the [`Config`eration]s should support [`Pixmap`]s.
    ///
    /// [`Config`eration]: crate::config::ConfigWrapper
    /// [`Pixmap`]: crate::surface::Pixmap
    #[inline]
    pub fn with_pixmap_surface_support(mut self, pss: bool) -> Self {
        self.pixmap_surface_support = pss;
        self
    }

    /// Whether or not the [`Config`eration]s should support [`Window`]s.
    ///
    /// [`Config`eration]: crate::config::ConfigWrapper
    /// [`Window`]: crate::surface::Window
    #[inline]
    pub fn with_window_surface_support(mut self, wss: bool) -> Self {
        self.window_surface_support = wss;
        self
    }

    /// Whether or not the [`Config`eration]s should support surfaceless
    /// contexts.
    ///
    /// Please refer to [`Context::make_current_surfaceless`] for more details.
    ///
    /// [`Config`eration]: crate::config::ConfigWrapper
    /// [`Context::make_current_surfaceless`]: crate::context::Context::make_current_surfaceless()
    #[inline]
    pub fn with_surfaceless_support(mut self, ss: bool) -> Self {
        self.surfaceless_support = ss;
        self
    }

    /// Sets whether double buffering should be enabled.
    ///
    /// If `true`, only double-buffered formats will be considered. If false,
    /// only single-buffer formats. `None` means "don't care".
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

    /// Sets whether hardware acceleration is required. `None` means "don't care".
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

    /// Finds all the [`Config`eration]s that match the specified requirements.
    ///
    /// [`Config`eration]: crate::config::ConfigWrapper
    #[inline]
    pub fn find<ND: NativeDisplay>(self, nd: &ND) -> Result<Vec<Config>, Error> {
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
