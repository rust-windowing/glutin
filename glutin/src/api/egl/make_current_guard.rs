use glutin_egl_sys as ffi;

/// A guard for when you want to make the context current. Destroying the guard
/// restores the previously-current context.
#[derive(Debug)]
pub struct MakeCurrentGuard {
    display: ffi::egl::types::EGLDisplay,
    old_display: ffi::egl::types::EGLDisplay,
    possibly_invalid: Option<MakeCurrentGuardInner>,
}

#[derive(Debug, PartialEq)]
struct MakeCurrentGuardInner {
    old_draw_surface: ffi::egl::types::EGLSurface,
    old_read_surface: ffi::egl::types::EGLSurface,
    old_context: ffi::egl::types::EGLContext,
}

impl MakeCurrentGuard {
    pub fn new(
        display: ffi::egl::types::EGLDisplay,
        draw_surface: ffi::egl::types::EGLSurface,
        read_surface: ffi::egl::types::EGLSurface,
        context: ffi::egl::types::EGLContext,
    ) -> Result<Self, String> {
        unsafe {
            let egl = super::EGL.as_ref().unwrap();

            let mut ret = MakeCurrentGuard {
                display,
                old_display: egl.GetCurrentDisplay(),
                possibly_invalid: Some(MakeCurrentGuardInner {
                    old_draw_surface: egl
                        .GetCurrentSurface(ffi::egl::DRAW as i32),
                    old_read_surface: egl
                        .GetCurrentSurface(ffi::egl::READ as i32),
                    old_context: egl.GetCurrentContext(),
                }),
            };

            if ret.old_display == ffi::egl::NO_DISPLAY {
                ret.invalidate();
            }

            let res =
                egl.MakeCurrent(display, draw_surface, read_surface, context);

            if res == 0 {
                let err = egl.GetError();
                Err(format!("`eglMakeCurrent` failed: 0x{:x}", err))
            } else {
                Ok(ret)
            }
        }
    }

    pub fn if_any_same_then_invalidate(
        &mut self,
        draw_surface: ffi::egl::types::EGLSurface,
        read_surface: ffi::egl::types::EGLSurface,
        context: ffi::egl::types::EGLContext,
    ) {
        if self.possibly_invalid.is_some() {
            let pi = self.possibly_invalid.as_ref().unwrap();
            if pi.old_draw_surface == draw_surface
                && draw_surface != ffi::egl::NO_SURFACE
                || pi.old_read_surface == read_surface
                    && read_surface != ffi::egl::NO_SURFACE
                || pi.old_context == context
            {
                self.invalidate();
            }
        }
    }

    pub fn invalidate(&mut self) {
        self.possibly_invalid.take();
    }
}

impl Drop for MakeCurrentGuard {
    fn drop(&mut self) {
        let egl = super::EGL.as_ref().unwrap();
        let (draw_surface, read_surface, context) =
            match self.possibly_invalid.take() {
                Some(inner) => (
                    inner.old_draw_surface,
                    inner.old_read_surface,
                    inner.old_context,
                ),
                None => (
                    ffi::egl::NO_SURFACE,
                    ffi::egl::NO_SURFACE,
                    ffi::egl::NO_CONTEXT,
                ),
            };

        let display = match self.old_display {
            ffi::egl::NO_DISPLAY => self.display,
            old_display => old_display,
        };

        unsafe {
            let res =
                egl.MakeCurrent(display, draw_surface, read_surface, context);

            if res == 0 {
                let err = egl.GetError();
                panic!("`eglMakeCurrent` failed: 0x{:x}", err)
            }
        }
    }
}
