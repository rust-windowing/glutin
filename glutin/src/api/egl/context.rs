//! Everything related to `EGLContext` management.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use glutin_egl_sys::egl::types::{EGLenum, EGLint};
use glutin_egl_sys::{egl, EGLContext};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

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

impl<D: HasDisplayHandle> Display<D> {
    pub(crate) fn create_context<W: HasWindowHandle>(
        &self,
        config: &Config<D>,
        context_attributes: &ContextAttributes<W>,
    ) -> Result<NotCurrentContext<D>> {
        let mut attrs = Vec::<EGLint>::new();

        let supports_opengl = self.inner.version > Version::new(1, 3);
        let config_api = config.api();

        let (api, mut version) = match context_attributes.inner.api {
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
        if is_one_five || self.inner.display_extensions.contains("EGL_KHR_create_context") {
            let mut flags = 0;

            // Add profile for the OpenGL Api.
            if api == egl::OPENGL_API {
                let (profile, new_version) =
                    context::pick_profile(context_attributes.inner.profile, version);
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

            let mut requested_no_error = false;
            match context_attributes.inner.robustness {
                Robustness::NotRobust => (),
                Robustness::NoError
                    if self.inner.features.contains(DisplayFeatures::CONTEXT_NO_ERROR) =>
                {
                    attrs.push(egl::CONTEXT_OPENGL_NO_ERROR_KHR as EGLint);
                    attrs.push(egl::TRUE as EGLint);
                    requested_no_error = true;
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

            if context_attributes.inner.debug && is_one_five && !requested_no_error {
                attrs.push(egl::CONTEXT_OPENGL_DEBUG as EGLint);
                attrs.push(egl::TRUE as EGLint);
            }

            if flags != 0 {
                attrs.push(egl::CONTEXT_FLAGS_KHR as EGLint);
                attrs.push(flags as EGLint);
            }
        } else if self.inner.version >= Version::new(1, 3) {
            // EGL 1.3 uses that to indicate client version instead of major/minor. The
            // constant is the same as `CONTEXT_MAJOR_VERSION`.
            if let Some(version) = version {
                attrs.push(egl::CONTEXT_CLIENT_VERSION as EGLint);
                attrs.push(version.major as EGLint);
            }
        }

        attrs.push(egl::NONE as EGLint);

        let shared_context = if let Some(shared_context) =
            context_attributes.inner.shared_context.as_ref()
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
pub struct NotCurrentContext<D> {
    inner: ContextInner<D>,
}

impl<D: HasDisplayHandle> NotCurrentContext<D> {
    /// Make a [`Self::PossiblyCurrentContext`] indicating that the context
    /// could be current on the thread.
    pub fn make_current_surfaceless(self) -> Result<PossiblyCurrentContext<D>> {
        self.inner.make_current_surfaceless()?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }

    fn new(inner: ContextInner<D>) -> Self {
        Self { inner }
    }
}

impl<D: HasDisplayHandle> NotCurrentGlContext for NotCurrentContext<D> {
    type PossiblyCurrentContext = PossiblyCurrentContext<D>;
    type Surface<T: SurfaceTypeTrait> = Surface<D, T>;

    fn treat_as_possibly_current(self) -> Self::PossiblyCurrentContext {
        PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData }
    }

    fn make_current<T: SurfaceTypeTrait>(
        self,
        surface: &Surface<D, T>,
    ) -> Result<PossiblyCurrentContext<D>> {
        self.inner.make_current_draw_read(surface, surface)?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }

    fn make_current_draw_read<T: SurfaceTypeTrait>(
        self,
        surface_draw: &Surface<D, T>,
        surface_read: &Surface<D, T>,
    ) -> Result<PossiblyCurrentContext<D>> {
        self.inner.make_current_draw_read(surface_draw, surface_read)?;
        Ok(PossiblyCurrentContext { inner: self.inner, _nosendsync: PhantomData })
    }
}

impl<D: HasDisplayHandle> GlContext for NotCurrentContext<D> {
    fn context_api(&self) -> ContextApi {
        self.inner.context_api()
    }
}

impl<D: HasDisplayHandle> GetGlConfig for NotCurrentContext<D> {
    type Target = Config<D>;

    fn config(&self) -> Self::Target {
        self.inner.config.clone()
    }
}

impl<D: HasDisplayHandle> GetGlDisplay for NotCurrentContext<D> {
    type Target = Display<D>;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl<D: HasDisplayHandle> AsRawContext for NotCurrentContext<D> {
    fn raw_context(&self) -> RawContext {
        RawContext::Egl(*self.inner.raw)
    }
}

impl<D: HasDisplayHandle> Sealed for NotCurrentContext<D> {}

/// A wrapper around `EGLContext` that could be current for the current thread.
#[derive(Debug)]
pub struct PossiblyCurrentContext<D> {
    pub(crate) inner: ContextInner<D>,
    _nosendsync: PhantomData<EGLContext>,
}

impl<D: HasDisplayHandle> PossiblyCurrentContext<D> {
    /// Make this context current on the calling thread.
    pub fn make_current_surfaceless(&self) -> Result<()> {
        self.inner.make_current_surfaceless()
    }
}

impl<D: HasDisplayHandle> PossiblyCurrentGlContext for PossiblyCurrentContext<D> {
    type NotCurrentContext = NotCurrentContext<D>;
    type Surface<T: SurfaceTypeTrait> = Surface<D, T>;

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

    fn make_current<T: SurfaceTypeTrait>(&self, surface: &Self::Surface<T>) -> Result<()> {
        self.inner.make_current_draw_read(surface, surface)
    }

    fn make_current_draw_read<T: SurfaceTypeTrait>(
        &self,
        surface_draw: &Self::Surface<T>,
        surface_read: &Self::Surface<T>,
    ) -> Result<()> {
        self.inner.make_current_draw_read(surface_draw, surface_read)
    }
}

impl<D: HasDisplayHandle> GlContext for PossiblyCurrentContext<D> {
    fn context_api(&self) -> ContextApi {
        self.inner.context_api()
    }
}

impl<D: HasDisplayHandle> GetGlConfig for PossiblyCurrentContext<D> {
    type Target = Config<D>;

    fn config(&self) -> Self::Target {
        self.inner.config.clone()
    }
}

impl<D: HasDisplayHandle> GetGlDisplay for PossiblyCurrentContext<D> {
    type Target = Display<D>;

    fn display(&self) -> Self::Target {
        self.inner.display.clone()
    }
}

impl<D: HasDisplayHandle> AsRawContext for PossiblyCurrentContext<D> {
    fn raw_context(&self) -> RawContext {
        RawContext::Egl(*self.inner.raw)
    }
}

impl<D: HasDisplayHandle> Sealed for PossiblyCurrentContext<D> {}

pub(crate) struct ContextInner<D> {
    display: Display<D>,
    config: Config<D>,
    raw: EglContext,
    api: egl::types::EGLenum,
}

impl<D: HasDisplayHandle> ContextInner<D> {
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
        surface_draw: &Surface<D, T>,
        surface_read: &Surface<D, T>,
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

impl<D> Drop for ContextInner<D> {
    fn drop(&mut self) {
        unsafe {
            self.display.inner.egl.DestroyContext(*self.display.inner.raw, *self.raw);
        }
    }
}

impl<D> fmt::Debug for ContextInner<D> {
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
