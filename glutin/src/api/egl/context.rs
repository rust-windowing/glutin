//! Everything related to `EGLContext` management.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use glutin_egl_sys::egl::types::{EGLenum, EGLint};
use glutin_egl_sys::{egl, EGLContext};

use crate::config::{Api, GetGlConfig};
use crate::context::{
    self, AsRawContext, ContextApi, ContextAttributes, GlProfile, RawContext, Robustness, Version,
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
        let mut attrs = Vec::<EGLint>::new();

        let supports_opengl = self.inner.version > Version::new(1, 3);
        let config_api = config.api();

        let (api, mut version) = match context_attributes.api {
            api @ Some(ContextApi::OpenGl(_)) | api @ None
                if supports_opengl && config_api.contains(Api::OPENGL) =>
            {
                (egl::OPENGL_API, api.and_then(|api| api.version()))
            },
            api @ Some(ContextApi::Gles(_)) | api @ None => {
                let version = match api.and_then(|api| api.version()) {
                    Some(version) => version,
                    None if config_api.contains(Api::GLES3) => Version::new(3, 0),
                    None if config_api.contains(Api::GLES2) => Version::new(2, 0),
                    _ => Version::new(1, 0),
                };
                (egl::OPENGL_ES_API, Some(version))
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("the requested context Api isn't supported.").into()
                )
            },
        };

        let is_one_five = self.inner.version >= Version::new(1, 5);
        if is_one_five || self.inner.client_extensions.contains("EGL_KHR_create_context") {
            let mut flags = 0;

            // Add profile for the OpenGL Api.
            if api == egl::OPENGL_API {
                let (profile, new_version) =
                    context::pick_profile(context_attributes.profile, version);
                version = Some(new_version);
                let profile = match profile {
                    GlProfile::Core => egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                    GlProfile::Compatibility => egl::CONTEXT_OPENGL_COMPATIBILITY_PROFILE_BIT,
                };

                attrs.push(egl::CONTEXT_OPENGL_PROFILE_MASK as EGLint);
                attrs.push(profile as EGLint);
            }

            if let Some(version) = version {
                attrs.push(egl::CONTEXT_MAJOR_VERSION as EGLint);
                attrs.push(version.major as EGLint);
                attrs.push(egl::CONTEXT_MINOR_VERSION as EGLint);
                attrs.push(version.minor as EGLint);
            }

            let has_robustsess = self.inner.features.contains(DisplayFeatures::CONTEXT_ROBUSTNESS);
            let has_no_error = self.inner.features.contains(DisplayFeatures::CONTEXT_NO_ERROR);

            match context_attributes.robustness {
                Robustness::NotRobust => (),
                Robustness::NoError if has_no_error => {
                    attrs.push(egl::CONTEXT_OPENGL_NO_ERROR_KHR as EGLint);
                    attrs.push(egl::TRUE as EGLint);
                },
                Robustness::RobustLoseContextOnReset if has_robustsess => {
                    attrs.push(egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY as EGLint);
                    attrs.push(egl::LOSE_CONTEXT_ON_RESET as EGLint);
                    flags |= egl::CONTEXT_OPENGL_ROBUST_ACCESS_BIT_KHR;
                },
                Robustness::RobustNoResetNotification if has_robustsess => {
                    attrs.push(egl::CONTEXT_OPENGL_RESET_NOTIFICATION_STRATEGY as EGLint);
                    attrs.push(egl::NO_RESET_NOTIFICATION as EGLint);
                    flags |= egl::CONTEXT_OPENGL_ROBUST_ACCESS_BIT_KHR;
                },
                _ => {
                    return Err(
                        ErrorKind::NotSupported("context robustness is not supported").into()
                    )
                },
            }

            if context_attributes.debug && is_one_five && !has_no_error {
                attrs.push(egl::CONTEXT_OPENGL_DEBUG as EGLint);
                attrs.push(egl::TRUE as EGLint);
            }

            if flags != 0 {
                attrs.push(egl::CONTEXT_FLAGS_KHR as EGLint);
                attrs.push(flags as EGLint);
            }
        }

        attrs.push(egl::NONE as EGLint);

        let shared_context = if let Some(shared_context) =
            context_attributes.shared_context.as_ref()
        {
            match shared_context {
                RawContext::Egl(shared_context) => *shared_context,
                #[allow(unreachable_patterns)]
                _ => return Err(ErrorKind::NotSupported("passed incompatible raw context").into()),
            }
        } else {
            egl::NO_CONTEXT
        };

        // Bind the api.
        unsafe {
            if self.inner.egl.BindAPI(api) == egl::FALSE {
                return Err(super::check_error().err().unwrap());
            }

            let config = config.clone();
            let context = self.inner.egl.CreateContext(
                *self.inner.raw,
                *config.inner.raw,
                shared_context,
                attrs.as_ptr(),
            );

            if context == egl::NO_CONTEXT {
                return Err(super::check_error().err().unwrap());
            }

            let inner =
                ContextInner { display: self.clone(), config, raw: EglContext(context), api };
            Ok(NotCurrentContext::new(inner))
        }
    }
}

/// A wrapper around `EGLContext` that is known to be not current.
#[derive(Debug)]
pub struct NotCurrentContext {
    inner: ContextInner,
}

impl NotCurrentContext {
    /// Make a [`Self::PossiblyCurrentContext`] indicating that the context
    /// could be current on the thread.
    pub fn make_current_surfaceless(self) -> Result<PossiblyCurrentContext> {
        self.inner.make_current_surfaceless()?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }

    fn new(inner: ContextInner) -> Self {
        Self { inner }
    }
}

impl NotCurrentGlContext for NotCurrentContext {
    type PossiblyCurrentContext = PossiblyCurrentContext;

    fn treat_as_possibly_current(self) -> Self::PossiblyCurrentContext {
        PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData }
    }
}

impl<T: SurfaceTypeTrait> NotCurrentGlContextSurfaceAccessor<T> for NotCurrentContext {
    type PossiblyCurrentContext = PossiblyCurrentContext;
    type Surface = Surface<T>;

    fn make_current(self, surface: &Surface<T>) -> Result<PossiblyCurrentContext> {
        self.inner.make_current_draw_read(surface, surface)?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }

    fn make_current_draw_read(
        self,
        surface_draw: &Surface<T>,
        surface_read: &Surface<T>,
    ) -> Result<PossiblyCurrentContext> {
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
        RawContext::Egl(*self.inner.raw)
    }
}

impl Sealed for NotCurrentContext {}

/// A wrapper around `EGLContext` that could be current for the current thread.
#[derive(Debug)]
pub struct PossiblyCurrentContext {
    pub(crate) inner: ContextInner,
    _nosendsync: PhantomData<EGLContext>,
}

impl PossiblyCurrentContext {
    /// Make this context current on the calling thread.
    pub fn make_current_surfaceless(&self) -> Result<()> {
        self.inner.make_current_surfaceless()
    }
}

impl PossiblyCurrentGlContext for PossiblyCurrentContext {
    type NotCurrentContext = NotCurrentContext;

    fn make_not_current(self) -> Result<Self::NotCurrentContext> {
        self.inner.make_not_current()?;
        Ok(NotCurrentContext::new(self.inner))
    }

    fn is_current(&self) -> bool {
        unsafe {
            self.inner.bind_api();
            self.inner.display.inner.egl.GetCurrentContext() == *self.inner.raw
        }
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
        RawContext::Egl(*self.inner.raw)
    }
}

impl Sealed for PossiblyCurrentContext {}

pub(crate) struct ContextInner {
    display: Display,
    config: Config,
    raw: EglContext,
    api: egl::types::EGLenum,
}

impl ContextInner {
    fn make_current_surfaceless(&self) -> Result<()> {
        unsafe {
            if self.display.inner.egl.MakeCurrent(
                *self.display.inner.raw,
                egl::NO_SURFACE,
                egl::NO_SURFACE,
                *self.raw,
            ) == egl::FALSE
            {
                super::check_error()
            } else {
                Ok(())
            }
        }
    }

    fn make_current_draw_read<T: SurfaceTypeTrait>(
        &self,
        surface_draw: &Surface<T>,
        surface_read: &Surface<T>,
    ) -> Result<()> {
        unsafe {
            let draw = surface_draw.raw;
            let read = surface_read.raw;
            if self.display.inner.egl.MakeCurrent(*self.display.inner.raw, draw, read, *self.raw)
                == egl::FALSE
            {
                super::check_error()
            } else {
                Ok(())
            }
        }
    }

    fn make_not_current(&self) -> Result<()> {
        unsafe {
            self.bind_api();

            if self.display.inner.egl.MakeCurrent(
                *self.display.inner.raw,
                egl::NO_SURFACE,
                egl::NO_SURFACE,
                egl::NO_CONTEXT,
            ) == egl::FALSE
            {
                super::check_error()
            } else {
                Ok(())
            }
        }
    }

    fn context_api(&self) -> ContextApi {
        match self.query_attribute(egl::CONTEXT_CLIENT_TYPE as EGLint).map(|a| a as EGLenum) {
            Some(egl::OPENGL_API) => ContextApi::OpenGl(None),
            // Map the rest to the GLES.
            _ => ContextApi::Gles(None),
        }
    }

    /// Query the context attribute.
    fn query_attribute(&self, attribute: EGLint) -> Option<EGLint> {
        unsafe {
            let mut attribute_value = 0;
            if self.display.inner.egl.QueryContext(
                self.display.inner.raw.cast(),
                self.raw.cast(),
                attribute,
                &mut attribute_value,
            ) == egl::FALSE
            {
                None
            } else {
                Some(attribute_value)
            }
        }
    }

    /// This function could panic, but it does that for sanity reasons.
    ///
    /// When we create context we bind api and then store it and rebind
    /// on functions requiring it, so if it fails it means that it worked
    /// before, but for some reason stopped working, which should not
    /// happen according to the specification.
    pub(crate) fn bind_api(&self) {
        unsafe {
            if self.display.inner.egl.QueryAPI() == self.api {
                return;
            }

            if self.display.inner.egl.BindAPI(self.api) == egl::FALSE {
                panic!("EGL Api couldn't be bound anymore.");
            }
        }
    }
}

impl Drop for ContextInner {
    fn drop(&mut self) {
        unsafe {
            self.display.inner.egl.DestroyContext(*self.display.inner.raw, *self.raw);
        }
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
struct EglContext(EGLContext);

// Impl only `Send` for EglContext.
unsafe impl Send for EglContext {}

impl Deref for EglContext {
    type Target = EGLContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
