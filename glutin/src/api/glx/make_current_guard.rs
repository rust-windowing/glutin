use crate::platform::unix::x11::XConnection;
use glutin_glx_sys as ffi;

use std::sync::Arc;

/// A guard for when you want to make the context current. Destroying the guard
/// restores the previously-current context.
#[derive(Debug)]
pub struct MakeCurrentGuard {
    old_display: *mut ffi::Display,
    display: *mut ffi::Display,
    xconn: Arc<XConnection>,
    possibly_invalid: Option<MakeCurrentGuardInner>,
}

#[derive(Debug)]
struct MakeCurrentGuardInner {
    old_drawable: ffi::glx::types::GLXDrawable,
    old_context: ffi::GLXContext,
}

impl MakeCurrentGuard {
    pub fn new(
        xconn: &Arc<XConnection>,
        drawable: ffi::glx::types::GLXDrawable,
        context: ffi::GLXContext,
    ) -> Result<Self, String> {
        unsafe {
            let glx = super::GLX.as_ref().unwrap();

            let ret = MakeCurrentGuard {
                old_display: glx.GetCurrentDisplay() as *mut _,
                display: xconn.display as *mut _,
                xconn: Arc::clone(xconn),
                possibly_invalid: Some(MakeCurrentGuardInner {
                    old_drawable: glx.GetCurrentDrawable(),
                    old_context: glx.GetCurrentContext(),
                }),
            };

            let res =
                glx.MakeCurrent(xconn.display as *mut _, drawable, context);

            if res == 0 {
                let err = xconn.check_errors();
                Err(format!("`glXMakeCurrent` failed: {:?}", err))
            } else {
                Ok(ret)
            }
        }
    }

    pub fn old_context(&mut self) -> Option<ffi::GLXContext> {
        self.possibly_invalid.as_ref().map(|pi| pi.old_context)
    }

    pub fn invalidate(&mut self) {
        self.possibly_invalid.take();
    }
}

impl Drop for MakeCurrentGuard {
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

        let res =
            unsafe { glx.MakeCurrent(display as *mut _, drawable, context) };

        if res == 0 {
            let err = self.xconn.check_errors();
            panic!("`glXMakeCurrent` failed: {:?}", err);
        }
    }
}
