//! Everything related to `NSOpenGLContext`.

use std::fmt;
use std::marker::PhantomData;

use cgl::CGLSetParameter;
use objc2::foundation::NSObject;
use objc2::rc::{autoreleasepool, Id, Shared};

use crate::config::GetGlConfig;
use crate::context::{AsRawContext, ContextApi, ContextAttributes, RawContext, Robustness};
use crate::display::GetGlDisplay;
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{SurfaceTypeTrait, SwapInterval};

use super::appkit::{run_on_main, MainThreadSafe, NSOpenGLCPSwapInterval, NSOpenGLContext};
use super::config::Config;
use super::display::Display;
use super::surface::Surface;

impl Display {
    pub(crate) unsafe fn create_context(
        &self,
        config: &Config,
        context_attributes: &ContextAttributes,
    ) -> Result<NotCurrentContext> {
        let share_context = match context_attributes.shared_context.as_ref() {
            Some(RawContext::Cgl(share_context)) => unsafe {
                share_context.cast::<NSOpenGLContext>().as_ref()
            },
            _ => None,
        };

        if matches!(context_attributes.api, Some(ContextApi::Gles(_))) {
            return Err(ErrorKind::NotSupported("gles is not supported with CGL").into());
        }

        if context_attributes.robustness != Robustness::NotRobust {
            return Err(ErrorKind::NotSupported("robustness is not supported with CGL").into());
        }

        let config = config.clone();
        let raw = NSOpenGLContext::newWithFormat_shareContext(&config.inner.raw, share_context)
            .ok_or(ErrorKind::BadConfig)?;

        if config.inner.transparency {
            let opacity = 0;
            super::check_error(unsafe {
                CGLSetParameter(raw.CGLContextObj().cast(), cgl::kCGLCPSurfaceOpacity, &opacity)
            })?;
        }

        let inner = ContextInner { display: self.clone(), config, raw };
        let context = NotCurrentContext::new(inner);

        Ok(context)
    }
}

/// A wrapper arounh `NSOpenGLContext` that is known to be not current on the
/// current thread.
#[derive(Debug)]
pub struct NotCurrentContext {
    pub(crate) inner: ContextInner,
    _nosync: PhantomData<std::cell::UnsafeCell<()>>,
}

impl NotCurrentContext {
    fn new(inner: ContextInner) -> Self {
        Self { inner, _nosync: PhantomData }
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
        RawContext::Cgl(Id::as_ptr(&self.inner.raw).cast())
    }
}

impl Sealed for NotCurrentContext {}

/// A wrapper around `NSOpenGLContext` that could be curront on the current
/// thread.
#[derive(Debug)]
pub struct PossiblyCurrentContext {
    pub(crate) inner: ContextInner,
    // The context could be current only on the one thread.
    _nosendsync: PhantomData<*mut ()>,
}

impl PossiblyCurrentGlContext for PossiblyCurrentContext {
    type NotCurrentContext = NotCurrentContext;

    fn make_not_current(self) -> Result<Self::NotCurrentContext> {
        self.inner.make_not_current()?;
        Ok(NotCurrentContext::new(self.inner))
    }

    fn is_current(&self) -> bool {
        if let Some(current) = NSOpenGLContext::currentContext() {
            current == self.inner.raw
        } else {
            false
        }
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
        RawContext::Cgl(Id::as_ptr(&self.inner.raw).cast())
    }
}

impl Sealed for PossiblyCurrentContext {}

pub(crate) struct ContextInner {
    display: Display,
    config: Config,
    pub(crate) raw: Id<NSOpenGLContext, Shared>,
}

impl ContextInner {
    fn make_current_draw_read<T: SurfaceTypeTrait>(
        &self,
        _surface_draw: &Surface<T>,
        _surface_read: &Surface<T>,
    ) -> ErrorKind {
        ErrorKind::NotSupported("make current draw read isn't supported with CGL")
    }

    fn make_current<T: SurfaceTypeTrait>(&self, surface: &Surface<T>) -> Result<()> {
        autoreleasepool(|_| {
            self.update();
            self.raw.makeCurrentContext();
            let raw = MainThreadSafe(&self.raw);
            let ns_view = MainThreadSafe(&surface.ns_view);

            run_on_main(move || unsafe {
                raw.setView(Some(*ns_view));
            });

            Ok(())
        })
    }

    fn context_api(&self) -> ContextApi {
        ContextApi::OpenGl(None)
    }

    pub(crate) fn set_swap_interval(&self, interval: SwapInterval) {
        let interval = match interval {
            SwapInterval::DontWait => 0,
            SwapInterval::Wait(_) => 1,
        };

        autoreleasepool(|_| unsafe {
            self.raw.setValues_forParameter(&interval, NSOpenGLCPSwapInterval);
        })
    }

    pub(crate) fn update(&self) {
        let raw = MainThreadSafe(&self.raw);
        run_on_main(move || {
            raw.update();
        });
    }

    pub(crate) fn flush_buffer(&self) -> Result<()> {
        autoreleasepool(|_| {
            self.raw.flushBuffer();
            Ok(())
        })
    }

    pub(crate) fn current_view(&self) -> Id<NSObject, Shared> {
        self.raw.view().expect("context to have a current view")
    }

    fn make_not_current(&self) -> Result<()> {
        self.update();
        NSOpenGLContext::clearCurrentContext();
        Ok(())
    }
}

impl fmt::Debug for ContextInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("config", &self.config.inner.raw)
            .field("raw", &self.raw)
            .finish()
    }
}
