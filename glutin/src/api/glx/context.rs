//! Everything related to `GLXContext`.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw::c_int;

use glutin_glx_sys::glx::types::GLXContext;
use glutin_glx_sys::{glx, glx_extra};

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
        let shared_context = if let Some(shared_context) =
            context_attributes.shared_context.as_ref()
        {
            match shared_context {
                RawContext::Glx(shared_context) => *shared_context,
                #[allow(unreachable_patterns)]
                _ => return Err(ErrorKind::NotSupported("incompatible context was passed").into()),
            }
        } else {
            std::ptr::null()
        };

        let context = if self.inner.client_extensions.contains("GLX_ARB_create_context")
            && self.inner.glx_extra.is_some()
        {
            self.create_context_arb(config, context_attributes, shared_context)?
        } else {
            self.create_context_legacy(config, shared_context)?
        };

        // Failed to create the context.
        if context.is_null() {
            return Err(ErrorKind::BadContext.into());
        }

        let config = config.clone();
        let is_gles = matches!(context_attributes.api, Some(ContextApi::Gles(_)));
        let inner =
            ContextInner { display: self.clone(), config, raw: GlxContext(context), is_gles };

        Ok(NotCurrentContext::new(inner))
    }

    fn create_context_arb(
        &self,
        config: &Config,
        context_attributes: &ContextAttributes,
        shared_context: GLXContext,
    ) -> Result<GLXContext> {
        let extra = self.inner.glx_extra.as_ref().unwrap();
        let mut attrs = Vec::<c_int>::with_capacity(16);

        // Check whether the ES context creation is supported.
        let supports_es = self.inner.features.contains(DisplayFeatures::CREATE_ES_CONTEXT);

        let (profile, version) = match context_attributes.api {
            api @ Some(ContextApi::OpenGl(_)) | api @ None => {
                let version = api.and_then(|api| api.version());
                let (profile, version) = context::pick_profile(context_attributes.profile, version);
                let profile = match profile {
                    GlProfile::Core => glx_extra::CONTEXT_CORE_PROFILE_BIT_ARB,
                    GlProfile::Compatibility => glx_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                };

                (Some(profile), Some(version))
            },
            Some(ContextApi::Gles(version)) if supports_es => (
                Some(glx_extra::CONTEXT_ES2_PROFILE_BIT_EXT),
                Some(version.unwrap_or(Version::new(2, 0))),
            ),
            _ => {
                return Err(ErrorKind::NotSupported(
                    "extension to create ES context with glx is not present.",
                )
                .into())
            },
        };

        // Set the profile.
        if let Some(profile) = profile {
            attrs.push(glx_extra::CONTEXT_PROFILE_MASK_ARB as c_int);
            attrs.push(profile as c_int);
        }

        // Add version.
        if let Some(version) = version {
            attrs.push(glx_extra::CONTEXT_MAJOR_VERSION_ARB as c_int);
            attrs.push(version.major as c_int);
            attrs.push(glx_extra::CONTEXT_MINOR_VERSION_ARB as c_int);
            attrs.push(version.minor as c_int);
        }

        if let Some(profile) = context_attributes.profile {
            let profile = match profile {
                GlProfile::Core => glx_extra::CONTEXT_CORE_PROFILE_BIT_ARB,
                GlProfile::Compatibility => glx_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
            };

            attrs.push(glx_extra::CONTEXT_PROFILE_MASK_ARB as c_int);
            attrs.push(profile as c_int);
        }

        let mut flags: c_int = 0;
        if self.inner.features.contains(DisplayFeatures::CONTEXT_ROBUSTNESS) {
            match context_attributes.robustness {
                Robustness::NotRobust => (),
                Robustness::RobustNoResetNotification => {
                    attrs.push(glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                    attrs.push(glx_extra::NO_RESET_NOTIFICATION_ARB as c_int);
                    flags |= glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                },
                Robustness::RobustLoseContextOnReset => {
                    attrs.push(glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                    attrs.push(glx_extra::LOSE_CONTEXT_ON_RESET_ARB as c_int);
                    flags |= glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                },
                Robustness::NoError => {
                    if !self.inner.features.contains(DisplayFeatures::CONTEXT_NO_ERROR) {
                        return Err(ErrorKind::NotSupported(
                            "GLX_ARB_create_context_no_error not supported",
                        )
                        .into());
                    }

                    attrs.push(glx_extra::CONTEXT_OPENGL_NO_ERROR_ARB as c_int);
                },
            }
        } else if context_attributes.robustness != Robustness::NotRobust {
            return Err(ErrorKind::NotSupported(
                "GLX_ARB_create_context_robustness is not supported",
            )
            .into());
        }

        // Debug flag.
        if context_attributes.debug {
            flags |= glx_extra::CONTEXT_DEBUG_BIT_ARB as c_int;
        }

        if flags != 0 {
            attrs.push(glx_extra::CONTEXT_FLAGS_ARB as c_int);
            attrs.push(flags as c_int);
        }

        // Flush control.
        if self.inner.features.contains(DisplayFeatures::CONTEXT_RELEASE_BEHAVIOR) {
            match context_attributes.release_behavior {
                // This is the default behavior in specification.
                //
                // XXX passing it explicitly causing issues with older mesa versions.
                ReleaseBehavior::Flush => (),
                ReleaseBehavior::None => {
                    attrs.push(glx_extra::CONTEXT_RELEASE_BEHAVIOR_ARB as c_int);
                    attrs.push(glx_extra::CONTEXT_RELEASE_BEHAVIOR_NONE_ARB as c_int);
                },
            }
        } else if context_attributes.release_behavior != ReleaseBehavior::Flush {
            return Err(ErrorKind::NotSupported(
                "flush control behavior GLX_ARB_context_flush_control",
            )
            .into());
        }

        // Terminate list with zero.
        attrs.push(0);

        super::last_glx_error(|| unsafe {
            extra.CreateContextAttribsARB(
                self.inner.raw.cast(),
                *config.inner.raw,
                shared_context,
                // Direct context
                1,
                attrs.as_ptr(),
            )
        })
    }

    fn create_context_legacy(
        &self,
        config: &Config,
        shared_context: GLXContext,
    ) -> Result<GLXContext> {
        let render_type =
            if config.float_pixels() { glx_extra::RGBA_FLOAT_TYPE_ARB } else { glx::RGBA_TYPE };

        super::last_glx_error(|| unsafe {
            self.inner.glx.CreateNewContext(
                self.inner.raw.cast(),
                *config.inner.raw,
                render_type as c_int,
                shared_context,
                // Direct context.
                1,
            )
        })
    }
}

/// A wrapper around `GLXContext` that is known to be not current.
#[derive(Debug)]
pub struct NotCurrentContext {
    inner: ContextInner,
}

impl NotCurrentContext {
    fn new(inner: ContextInner) -> Self {
        Self { inner }
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
        self.inner.make_current_draw_read(surface, surface)?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }

    fn make_current_draw_read(
        self,
        surface_draw: &Self::Surface,
        surface_read: &Self::Surface,
    ) -> Result<Self::PossiblyCurrentContext> {
        self.inner.make_current_draw_read(surface_draw, surface_read)?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }
}

impl GlContext for NotCurrentContext {
    fn context_api(&self) -> ContextApi {
        self.inner.context_api()
    }
}

impl GetGlConfig for NotCurrentContext {
    type Target = Config;

    fn config(&self) -> Self::Target {
        self.inner.config.clone()
    }
}

impl GetGlDisplay for NotCurrentContext {
    type Target = Display;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl AsRawContext for NotCurrentContext {
    fn raw_context(&self) -> RawContext {
        RawContext::Glx(*self.inner.raw)
    }
}

impl Sealed for NotCurrentContext {}

/// A wrapper around `GLXContext` that could be current for the current thread.
#[derive(Debug)]
pub struct PossiblyCurrentContext {
    inner: ContextInner,
    // The context could be current only on the one thread.
    _nosendsync: PhantomData<GLXContext>,
}

impl PossiblyCurrentGlContext for PossiblyCurrentContext {
    type NotCurrentContext = NotCurrentContext;

    fn make_not_current(self) -> Result<Self::NotCurrentContext> {
        self.inner.make_not_current()?;
        Ok(NotCurrentContext::new(self.inner))
    }

    fn is_current(&self) -> bool {
        unsafe { self.inner.display.inner.glx.GetCurrentContext() == *self.inner.raw }
    }
}

impl<T: SurfaceTypeTrait> PossiblyCurrentContextGlSurfaceAccessor<T> for PossiblyCurrentContext {
    type Surface = Surface<T>;

    fn make_current(&self, surface: &Self::Surface) -> Result<()> {
        self.inner.make_current_draw_read(surface, surface)
    }

    fn make_current_draw_read(
        &self,
        surface_draw: &Self::Surface,
        surface_read: &Self::Surface,
    ) -> Result<()> {
        self.inner.make_current_draw_read(surface_draw, surface_read)
    }
}

impl GlContext for PossiblyCurrentContext {
    fn context_api(&self) -> ContextApi {
        self.inner.context_api()
    }
}

impl GetGlConfig for PossiblyCurrentContext {
    type Target = Config;

    fn config(&self) -> Self::Target {
        self.inner.config.clone()
    }
}

impl GetGlDisplay for PossiblyCurrentContext {
    type Target = Display;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl AsRawContext for PossiblyCurrentContext {
    fn raw_context(&self) -> RawContext {
        RawContext::Glx(*self.inner.raw)
    }
}

impl Sealed for PossiblyCurrentContext {}

struct ContextInner {
    display: Display,
    config: Config,
    raw: GlxContext,
    is_gles: bool,
}

impl ContextInner {
    fn make_current_draw_read<T: SurfaceTypeTrait>(
        &self,
        surface_draw: &Surface<T>,
        surface_read: &Surface<T>,
    ) -> Result<()> {
        super::last_glx_error(|| unsafe {
            self.display.inner.glx.MakeContextCurrent(
                self.display.inner.raw.cast(),
                surface_draw.raw,
                surface_read.raw,
                *self.raw,
            );
        })
    }

    fn make_not_current(&self) -> Result<()> {
        super::last_glx_error(|| unsafe {
            self.display.inner.glx.MakeContextCurrent(
                self.display.inner.raw.cast(),
                0,
                0,
                std::ptr::null(),
            );
        })
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
        let _ = super::last_glx_error(|| unsafe {
            self.display.inner.glx.DestroyContext(self.display.inner.raw.cast(), *self.raw);
        });
    }
}

impl fmt::Debug for ContextInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("display", &self.display.inner.raw)
            .field("config", &self.config.inner.raw)
            .field("raw", &self.raw)
            .finish()
    }
}

#[derive(Debug)]
struct GlxContext(GLXContext);

unsafe impl Send for GlxContext {}

impl Deref for GlxContext {
    type Target = GLXContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
