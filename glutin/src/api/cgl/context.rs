//! Everything related to `NSOpenGLContext`.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use cgl::CGLSetParameter;
use cocoa::appkit::{NSOpenGLContext, NSOpenGLContextParameter};
use cocoa::base::{id, nil};

use objc::rc::autoreleasepool;
use objc::runtime::{BOOL, NO};

use crate::config::GetGlConfig;
use crate::context::{AsRawContext, ContextApi, ContextAttributes, RawContext, Robustness};
use crate::display::GetGlDisplay;
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{SurfaceTypeTrait, SwapInterval};

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
            Some(RawContext::Cgl(share_context)) => share_context.cast(),
            _ => nil,
        };

        if matches!(context_attributes.api, Some(ContextApi::Gles(_))) {
            return Err(ErrorKind::NotSupported("gles is not supported with CGL").into());
        }

        if context_attributes.robustness != Robustness::NotRobust {
            return Err(ErrorKind::NotSupported("robustness is not supported with CGL").into());
        }

        unsafe {
            let config = config.clone();
            let raw = NSOpenGLContext::alloc(nil)
                .initWithFormat_shareContext_(*config.inner.raw, share_context as *mut _);

            if config.inner.transparency {
                let opacity = 0;
                super::check_error(CGLSetParameter(
                    raw.CGLContextObj().cast(),
                    cgl::kCGLCPSurfaceOpacity,
                    &opacity,
                ))?;
            }

            let inner = ContextInner { display: self.clone(), config, raw: NSOpenGLContextId(raw) };
            let context = NotCurrentContext { inner };

            Ok(context)
        }
    }
}

/// A wrapper arounh `NSOpenGLContext` that is known to be not current on the
/// current thread.
#[derive(Debug)]
pub struct NotCurrentContext {
    pub(crate) inner: ContextInner,
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
        RawContext::Cgl(self.inner.raw.cast())
    }
}

impl Sealed for NotCurrentContext {}

/// A wrapper around `NSOpenGLContext` that could be curront on the current
/// thread.
#[derive(Debug)]
pub struct PossiblyCurrentContext {
    pub(crate) inner: ContextInner,
    // The context could be current only on the one thread.
    _nosendsync: PhantomData<id>,
}

impl PossiblyCurrentGlContext for PossiblyCurrentContext {
    type NotCurrentContext = NotCurrentContext;

    fn make_not_current(self) -> Result<Self::NotCurrentContext> {
        self.inner.make_not_current()?;
        Ok(NotCurrentContext::new(self.inner))
    }

    fn is_current(&self) -> bool {
        autoreleasepool(|| unsafe {
            let current = NSOpenGLContext::currentContext(nil);
            if current != nil {
                let is_equal: BOOL = msg_send![current, isEqual: *self.inner.raw];
                is_equal != NO
            } else {
                false
            }
        })
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
        RawContext::Cgl(self.inner.raw.cast())
    }
}

impl Sealed for PossiblyCurrentContext {}

pub(crate) struct ContextInner {
    display: Display,
    config: Config,
    pub(crate) raw: NSOpenGLContextId,
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
        autoreleasepool(|| unsafe {
            self.raw.update();
            self.raw.makeCurrentContext();
            self.raw.setView_(surface.ns_view);
            Ok(())
        })
    }

    pub(crate) fn set_swap_interval(&self, interval: SwapInterval) {
        let interval = match interval {
            SwapInterval::DontWait => 0,
            SwapInterval::Wait(_) => 1,
        };

        autoreleasepool(|| unsafe {
            self.raw.setValues_forParameter_(
                &interval,
                NSOpenGLContextParameter::NSOpenGLCPSwapInterval,
            );
        })
    }

    pub(crate) fn update(&self) {
        unsafe { self.raw.update() }
    }

    pub(crate) fn flush_buffer(&self) -> Result<()> {
        autoreleasepool(|| unsafe {
            self.raw.flushBuffer();
            Ok(())
        })
    }

    pub(crate) fn current_view(&self) -> id {
        unsafe { self.raw.view() }
    }

    fn make_not_current(&self) -> Result<()> {
        unsafe {
            self.raw.update();
            NSOpenGLContext::clearCurrentContext(nil);
            Ok(())
        }
    }
}

impl Drop for ContextInner {
    fn drop(&mut self) {
        unsafe {
            if *self.raw != nil {
                let _: () = msg_send![*self.raw, release];
            }
        }
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

#[derive(Debug)]
pub(crate) struct NSOpenGLContextId(id);

unsafe impl Send for NSOpenGLContextId {}

impl Deref for NSOpenGLContextId {
    type Target = id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
