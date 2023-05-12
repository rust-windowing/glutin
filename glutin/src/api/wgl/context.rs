//! WGL context handling.

use std::fmt;
use std::io::Error as IoError;
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw::c_int;

use glutin_wgl_sys::wgl::types::HGLRC;
use glutin_wgl_sys::{wgl, wgl_extra};
use raw_window_handle::RawWindowHandle;
use windows_sys::Win32::Graphics::Gdi::{self as gdi, HDC};

use crate::config::GetGlConfig;
use crate::context::{
    self, AsRawContext, ContextApi, ContextAttributes, GlProfile, RawContext, ReleaseBehavior,
    Robustness, Version,
};
use crate::display::{DisplayFeatures, GetGlDisplay};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::SurfaceTypeTrait;

use super::config::Config;
use super::display::Display;
use super::surface::Surface;

impl Display {
    pub(crate) unsafe fn create_context(
        &self,
        config: &Config,
        context_attributes: &ContextAttributes,
    ) -> Result<NotCurrentContext> {
        let hdc = match context_attributes.raw_window_handle.as_ref() {
            handle @ Some(RawWindowHandle::Win32(window)) => unsafe {
                let _ = config.apply_on_native_window(handle.unwrap());
                gdi::GetDC(window.hwnd as _)
            },
            _ => config.inner.hdc,
        };

        let share_ctx = match context_attributes.shared_context {
            Some(RawContext::Wgl(share)) => share,
            _ => std::ptr::null(),
        };

        let context = if self.inner.client_extensions.contains("WGL_ARB_create_context") {
            self.create_context_arb(hdc, share_ctx, context_attributes)?
        } else {
            unsafe {
                let raw = wgl::CreateContext(hdc as *const _);
                if raw.is_null() {
                    return Err(IoError::last_os_error().into());
                }

                // Context sharing.
                if !share_ctx.is_null() && wgl::ShareLists(share_ctx, raw) == 0 {
                    return Err(IoError::last_os_error().into());
                }

                WglContext(raw)
            }
        };

        let config = config.clone();
        let is_gles = matches!(context_attributes.api, Some(ContextApi::Gles(_)));
        let inner = ContextInner { display: self.clone(), config, raw: context, is_gles };
        Ok(NotCurrentContext { inner })
    }

    fn create_context_arb(
        &self,
        hdc: HDC,
        share_context: HGLRC,
        context_attributes: &ContextAttributes,
    ) -> Result<WglContext> {
        let extra = self.inner.wgl_extra.as_ref().unwrap();
        let mut attrs = Vec::<c_int>::with_capacity(16);

        // Check whether the ES context creation is supported.
        let supports_es = self.inner.features.contains(DisplayFeatures::CREATE_ES_CONTEXT);

        let (profile, version) = match context_attributes.api {
            api @ Some(ContextApi::OpenGl(_)) | api @ None => {
                let version = api.and_then(|api| api.version());
                let (profile, version) = context::pick_profile(context_attributes.profile, version);
                let profile = match profile {
                    GlProfile::Core => wgl_extra::CONTEXT_CORE_PROFILE_BIT_ARB,
                    GlProfile::Compatibility => wgl_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                };

                (Some(profile), Some(version))
            },
            Some(ContextApi::Gles(version)) if supports_es => (
                Some(wgl_extra::CONTEXT_ES2_PROFILE_BIT_EXT),
                Some(version.unwrap_or(Version::new(2, 0))),
            ),
            _ => {
                return Err(ErrorKind::NotSupported(
                    "extension to create ES context with wgl is not present",
                )
                .into())
            },
        };

        // Set the profile.
        if let Some(profile) = profile {
            attrs.push(wgl_extra::CONTEXT_PROFILE_MASK_ARB as c_int);
            attrs.push(profile as c_int);
        }

        // Add version.
        if let Some(version) = version {
            attrs.push(wgl_extra::CONTEXT_MAJOR_VERSION_ARB as c_int);
            attrs.push(version.major as c_int);
            attrs.push(wgl_extra::CONTEXT_MINOR_VERSION_ARB as c_int);
            attrs.push(version.minor as c_int);
        }

        if let Some(profile) = context_attributes.profile {
            let profile = match profile {
                GlProfile::Core => wgl_extra::CONTEXT_CORE_PROFILE_BIT_ARB,
                GlProfile::Compatibility => wgl_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
            };

            attrs.push(wgl_extra::CONTEXT_PROFILE_MASK_ARB as c_int);
            attrs.push(profile as c_int);
        }

        let mut flags: c_int = 0;
        if self.inner.features.contains(DisplayFeatures::CONTEXT_ROBUSTNESS) {
            match context_attributes.robustness {
                Robustness::NotRobust => (),
                Robustness::RobustNoResetNotification => {
                    attrs.push(wgl_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                    attrs.push(wgl_extra::NO_RESET_NOTIFICATION_ARB as c_int);
                    flags |= wgl_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                },
                Robustness::RobustLoseContextOnReset => {
                    attrs.push(wgl_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                    attrs.push(wgl_extra::LOSE_CONTEXT_ON_RESET_ARB as c_int);
                    flags |= wgl_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                },
                Robustness::NoError => {
                    if !self.inner.features.contains(DisplayFeatures::CONTEXT_NO_ERROR) {
                        return Err(ErrorKind::NotSupported(
                            "WGL_ARB_create_context_no_error not supported",
                        )
                        .into());
                    }

                    attrs.push(wgl_extra::CONTEXT_OPENGL_NO_ERROR_ARB as c_int);
                },
            }
        } else if context_attributes.robustness != Robustness::NotRobust {
            return Err(ErrorKind::NotSupported(
                "WGL_ARB_create_context_robustness is not supported",
            )
            .into());
        }

        // Debug flag.
        if context_attributes.debug {
            flags |= wgl_extra::CONTEXT_DEBUG_BIT_ARB as c_int;
        }

        if flags != 0 {
            attrs.push(wgl_extra::CONTEXT_FLAGS_ARB as c_int);
            attrs.push(flags as c_int);
        }

        // Flush control.
        if self.inner.features.contains(DisplayFeatures::CONTEXT_RELEASE_BEHAVIOR) {
            match context_attributes.release_behavior {
                // This is the default behavior in specification.
                //
                // XXX even though we check for extensions don't pass it because it could cause
                // issues.
                ReleaseBehavior::Flush => (),
                ReleaseBehavior::None => {
                    attrs.push(wgl_extra::CONTEXT_RELEASE_BEHAVIOR_ARB as c_int);
                    attrs.push(wgl_extra::CONTEXT_RELEASE_BEHAVIOR_NONE_ARB as c_int);
                },
            }
        } else if context_attributes.release_behavior != ReleaseBehavior::Flush {
            return Err(ErrorKind::NotSupported(
                "flush control behavior WGL_ARB_context_flush_control",
            )
            .into());
        }

        // Terminate list with zero.
        attrs.push(0);

        unsafe {
            let raw = extra.CreateContextAttribsARB(hdc as _, share_context, attrs.as_ptr());
            if raw.is_null() {
                Err(IoError::last_os_error().into())
            } else {
                Ok(WglContext(raw))
            }
        }
    }
}

/// A wrapper around WGL context that could be current to the calling thread.
#[derive(Debug)]
pub struct PossiblyCurrentContext {
    inner: ContextInner,
    // The context could be current only on the one thread.
    _nosendsync: PhantomData<HGLRC>,
}

/// A wrapper around the WGL context that is known to be not current to the
/// calling thread.
#[derive(Debug)]
pub struct NotCurrentContext {
    inner: ContextInner,
}

impl Sealed for PossiblyCurrentContext {}
impl Sealed for NotCurrentContext {}

impl NotCurrentContext {
    fn new(inner: ContextInner) -> Self {
        Self { inner }
    }
}

impl GetGlDisplay for NotCurrentContext {
    type Target = Display;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl GetGlDisplay for PossiblyCurrentContext {
    type Target = Display;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl GetGlConfig for NotCurrentContext {
    type Target = Config;

    fn config(&self) -> Self::Target {
        self.inner.config.clone()
    }
}

impl GetGlConfig for PossiblyCurrentContext {
    type Target = Config;

    fn config(&self) -> Self::Target {
        self.inner.config.clone()
    }
}

impl<T: SurfaceTypeTrait> PossiblyCurrentContextGlSurfaceAccessor<T> for PossiblyCurrentContext {
    type Surface = Surface<T>;

    fn make_current(&self, surface: &Self::Surface) -> Result<()> {
        self.inner.make_current(surface)
    }

    fn make_current_draw_read(
        &self,
        surface_draw: &Self::Surface,
        surface_read: &Self::Surface,
    ) -> Result<()> {
        Err(self.inner.make_current_draw_read(surface_draw, surface_read).into())
    }
}

impl PossiblyCurrentGlContext for PossiblyCurrentContext {
    type NotCurrentContext = NotCurrentContext;

    fn make_not_current(self) -> Result<Self::NotCurrentContext> {
        unsafe {
            if self.is_current() {
                let hdc = wgl::GetCurrentDC();
                if wgl::MakeCurrent(hdc, std::ptr::null()) == 0 {
                    return Err(IoError::last_os_error().into());
                }
            }

            Ok(NotCurrentContext::new(self.inner))
        }
    }

    fn is_current(&self) -> bool {
        unsafe { wgl::GetCurrentContext() == *self.inner.raw }
    }
}

impl NotCurrentGlContext for NotCurrentContext {
    type PossiblyCurrentContext = PossiblyCurrentContext;

    fn treat_as_possibly_current(self) -> PossiblyCurrentContext {
        PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData }
    }
}

impl<T: SurfaceTypeTrait> NotCurrentGlContextSurfaceAccessor<T> for NotCurrentContext {
    type PossiblyCurrentContext = PossiblyCurrentContext;
    type Surface = Surface<T>;

    fn make_current(self, surface: &Self::Surface) -> Result<Self::PossiblyCurrentContext> {
        self.inner.make_current(surface)?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }

    fn make_current_draw_read(
        self,
        surface_draw: &Self::Surface,
        surface_read: &Self::Surface,
    ) -> Result<Self::PossiblyCurrentContext> {
        Err(self.inner.make_current_draw_read(surface_draw, surface_read).into())
    }
}

impl GlContext for PossiblyCurrentContext {
    fn context_api(&self) -> ContextApi {
        self.inner.context_api()
    }
}

impl GlContext for NotCurrentContext {
    fn context_api(&self) -> ContextApi {
        self.inner.context_api()
    }
}

impl AsRawContext for PossiblyCurrentContext {
    fn raw_context(&self) -> RawContext {
        RawContext::Wgl(*self.inner.raw)
    }
}

impl AsRawContext for NotCurrentContext {
    fn raw_context(&self) -> RawContext {
        RawContext::Wgl(*self.inner.raw)
    }
}

struct ContextInner {
    display: Display,
    config: Config,
    raw: WglContext,
    is_gles: bool,
}

impl fmt::Debug for ContextInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("config", &self.config.inner.pixel_format_index)
            .field("raw", &self.raw)
            .finish()
    }
}

#[derive(Debug)]
struct WglContext(HGLRC);

impl Deref for WglContext {
    type Target = HGLRC;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl Send for WglContext {}

impl ContextInner {
    fn make_current_draw_read<T: SurfaceTypeTrait>(
        &self,
        _surface_draw: &Surface<T>,
        _surface_read: &Surface<T>,
    ) -> ErrorKind {
        ErrorKind::NotSupported("make_current_draw_read is not supported by WGL")
    }

    fn make_current<T: SurfaceTypeTrait>(&self, surface: &Surface<T>) -> Result<()> {
        unsafe {
            if wgl::MakeCurrent(surface.hdc as _, self.raw.cast()) == 0 {
                Err(IoError::last_os_error().into())
            } else {
                Ok(())
            }
        }
    }

    fn context_api(&self) -> ContextApi {
        if self.is_gles {
            ContextApi::Gles(None)
        } else {
            ContextApi::OpenGl(None)
        }
    }
}

impl Drop for ContextInner {
    fn drop(&mut self) {
        unsafe {
            wgl::DeleteContext(*self.raw);
        }
    }
}
