use super::ffi;
use winit_types::error::Error;
use winit_types::platform::OsError;

/// A guard for when you want to make the context current. Destroying the guard
/// restores the previously-current context.
#[derive(Debug)]
pub struct MakeCurrentGuard {
    display: ffi::egl::types::EGLDisplay,
    old_display: ffi::egl::types::EGLDisplay,
    possibly_invalid: Option<MakeCurrentGuardInner>,
    keep: bool,
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
    ) -> Result<Self, Error> {
        unsafe {
            let egl = super::EGL.as_ref().unwrap();

            let mut ret = MakeCurrentGuard {
                display,
                old_display: egl.GetCurrentDisplay(),
                possibly_invalid: Some(MakeCurrentGuardInner {
                    old_draw_surface: egl.GetCurrentSurface(ffi::egl::DRAW as i32),
                    old_read_surface: egl.GetCurrentSurface(ffi::egl::READ as i32),
                    old_context: egl.GetCurrentContext(),
                }),
                keep: false,
            };

            if ret.old_display == ffi::egl::NO_DISPLAY {
                ret.invalidate();
            }

            let res = egl.MakeCurrent(display, draw_surface, read_surface, context);

            if res == 0 {
                Err(make_oserror!(OsError::Misc(format!(
                    "eglMakeCurrent failed with 0x{:x}",
                    egl.GetError()
                ))))
            } else {
                Ok(ret)
            }
        }
    }

    pub fn new_keep(display: ffi::egl::types::EGLDisplay) -> Self {
        unsafe {
            let egl = super::EGL.as_ref().unwrap();

            let mut ret = MakeCurrentGuard {
                display,
                old_display: egl.GetCurrentDisplay(),
                possibly_invalid: Some(MakeCurrentGuardInner {
                    old_draw_surface: egl.GetCurrentSurface(ffi::egl::DRAW as i32),
                    old_read_surface: egl.GetCurrentSurface(ffi::egl::READ as i32),
                    old_context: egl.GetCurrentContext(),
                }),
                keep: true,
            };

            if ret.old_display == ffi::egl::NO_DISPLAY {
                ret.invalidate();
            }

            ret
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
            if pi.old_draw_surface == draw_surface && draw_surface != ffi::egl::NO_SURFACE
                || pi.old_read_surface == read_surface && read_surface != ffi::egl::NO_SURFACE
                || pi.old_context == context && pi.old_context != ffi::egl::NO_CONTEXT
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
        let (draw_surface, read_surface, context) = match self.possibly_invalid.take() {
            Some(inner) => {
                if self.keep {
                    return;
                } else {
                    (
                        inner.old_draw_surface,
                        inner.old_read_surface,
                        inner.old_context,
                    )
                }
            }
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
            let res = egl.MakeCurrent(display, draw_surface, read_surface, context);

            if res == 0 {
                let err = egl.GetError();
                panic!("[glutin] `eglMakeCurrent` failed: 0x{:x}", err)
            }
        }
    }
}
