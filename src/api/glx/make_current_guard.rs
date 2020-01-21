use super::ffi;

use glutin_x11_sym::Display;

use std::sync::Arc;

/// A guard for when you want to make the context current. Destroying the guard
/// restores the previously-current context.
#[derive(Debug)]
pub struct MakeCurrentGuard {
    old_display: *mut ffi::Display,
    display: *mut ffi::Display,
    x11_display: Arc<Display>,
    possibly_invalid: Option<MakeCurrentGuardInner>,
}

#[derive(Debug)]
struct MakeCurrentGuardInner {
    old_drawable: ffi::glx::types::GLXDrawable,
    old_context: ffi::glx::types::GLXContext,
}

impl MakeCurrentGuard {
    #[inline]
    pub fn new(
        x11_display: &Arc<Display>,
        drawable: ffi::glx::types::GLXDrawable,
        context: ffi::glx::types::GLXContext,
    ) -> Result<Self, String> {
        unsafe {
            let glx = super::GLX.as_ref().unwrap();

            let ret = MakeCurrentGuard {
                old_display: glx.GetCurrentDisplay() as *mut _,
                display: x11_display.raw() as *mut _,
                x11_display: Arc::clone(x11_display),
                possibly_invalid: Some(MakeCurrentGuardInner {
                    old_drawable: glx.GetCurrentDrawable(),
                    old_context: glx.GetCurrentContext(),
                }),
            };

            let res = glx.MakeCurrent(x11_display.raw() as *mut _, drawable, context);

            if res == 0 {
                let err = x11_display.check_errors();
                Err(format!("`glXMakeCurrent` failed: {:?}", err))
            } else {
                Ok(ret)
            }
        }
    }

    #[inline]
    pub fn old_context(&mut self) -> Option<ffi::glx::types::GLXContext> {
        self.possibly_invalid.as_ref().map(|pi| pi.old_context)
    }

    #[inline]
    pub fn invalidate(&mut self) {
        self.possibly_invalid.take();
    }
}

impl Drop for MakeCurrentGuard {
    #[inline]
    fn drop(&mut self) {
        let glx = super::GLX.as_ref().unwrap();
        let (drawable, context) = match self.possibly_invalid.take() {
            Some(inner) => (inner.old_drawable, inner.old_context),
            None => (0, std::ptr::null()),
        };

        let display = match self.old_display {
            old_display if old_display == std::ptr::null_mut() => self.display,
            old_display => old_display,
        };

        let res = unsafe { glx.MakeCurrent(display as *mut _, drawable, context) };

        if res == 0 {
            let err = self.x11_display.check_errors();
            panic!("`glXMakeCurrent` failed: {:?}", err);
        }
    }
}
