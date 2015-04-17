use BuilderAttribs;
use CreationError;
use GlRequest;
use Api;

use libc;
use std::ffi::CString;
use std::{mem, ptr};

use super::ffi;

pub struct Context {
    display: *mut ffi::Display,
    window: ffi::Window,
    context: ffi::GLXContext,
}

impl Context {
    pub fn new(builder: BuilderAttribs, display: *mut ffi::Display, window: ffi::Window,
               fb_config: ffi::glx::types::GLXFBConfig, mut visual_infos: ffi::glx::types::XVisualInfo)
               -> Result<Context, CreationError>
    {
        // creating GL context
        let (context, extra_functions) = unsafe {
            let mut attributes = Vec::new();

            match builder.gl_version {
                GlRequest::Latest => {},
                GlRequest::Specific(Api::OpenGl, (major, minor)) => {
                    attributes.push(ffi::GLX_CONTEXT_MAJOR_VERSION);
                    attributes.push(major as libc::c_int);
                    attributes.push(ffi::GLX_CONTEXT_MINOR_VERSION);
                    attributes.push(minor as libc::c_int);
                },
                GlRequest::Specific(_, _) => return Err(CreationError::NotSupported),
                GlRequest::GlThenGles { opengl_version: (major, minor), .. } => {
                    attributes.push(ffi::GLX_CONTEXT_MAJOR_VERSION);
                    attributes.push(major as libc::c_int);
                    attributes.push(ffi::GLX_CONTEXT_MINOR_VERSION);
                    attributes.push(minor as libc::c_int);
                },
            }

            if builder.gl_debug {
                attributes.push(ffi::glx_extra::CONTEXT_FLAGS_ARB as libc::c_int);
                attributes.push(ffi::glx_extra::CONTEXT_DEBUG_BIT_ARB as libc::c_int);
            }

            attributes.push(0);

            // loading the extra GLX functions
            let extra_functions = ffi::glx_extra::Glx::load_with(|addr| {
                let addr = CString::new(addr.as_bytes()).unwrap();
                let addr = addr.as_ptr();
                unsafe {
                    ffi::glx::GetProcAddress(addr as *const _) as *const libc::c_void
                }
            });

            let share = if let Some(win) = builder.sharing {
                //win.x.context.context
                unimplemented!()
            } else {
                ptr::null()
            };

            let mut context = if extra_functions.CreateContextAttribsARB.is_loaded() {
                extra_functions.CreateContextAttribsARB(display as *mut ffi::glx_extra::types::Display,
                    fb_config, share, 1, attributes.as_ptr())
            } else {
                ptr::null()
            };

            if context.is_null() {
                context = ffi::glx::CreateContext(display, &mut visual_infos, share, 1)
            }

            if context.is_null() {
                return Err(CreationError::OsError(format!("GL context creation failed")));
            }

            (context, extra_functions)
        };

        // vsync
        if builder.vsync {
            unsafe { ffi::glx::MakeCurrent(display, window, context) };

            if extra_functions.SwapIntervalEXT.is_loaded() {
                // this should be the most common extension
                unsafe {
                    extra_functions.SwapIntervalEXT(display as *mut _, window, 1);
                }

                // checking that it worked
                if builder.strict {
                    let mut swap = unsafe { mem::uninitialized() };
                    unsafe {
                        ffi::glx::QueryDrawable(display, window,
                                                ffi::glx_extra::SWAP_INTERVAL_EXT as i32,
                                                &mut swap);
                    }

                    if swap != 1 {
                        return Err(CreationError::OsError(format!("Couldn't setup vsync: expected \
                                                    interval `1` but got `{}`", swap)));
                    }
                }

            // GLX_MESA_swap_control is not official
            /*} else if extra_functions.SwapIntervalMESA.is_loaded() {
                unsafe {
                    extra_functions.SwapIntervalMESA(1);
                }*/

            } else if extra_functions.SwapIntervalSGI.is_loaded() {
                unsafe {
                    extra_functions.SwapIntervalSGI(1);
                }

            } else if builder.strict {
                return Err(CreationError::OsError(format!("Couldn't find any available vsync extension")));
            }

            unsafe { ffi::glx::MakeCurrent(display, 0, ptr::null()) };
        }

        Ok(Context {
            display: display,
            window: window,
            context: context,
        })
    }

    pub fn make_current(&self) {
        let res = unsafe { ffi::glx::MakeCurrent(self.display, self.window, self.context) };
        if res == 0 {
            panic!("glx::MakeCurrent failed");
        }
    }

    pub fn is_current(&self) -> bool {
        unsafe { ffi::glx::GetCurrentContext() == self.context }
    }

    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        unsafe {
            ffi::glx::GetProcAddress(addr as *const _) as *const ()
        }
    }

    pub fn swap_buffers(&self) {
        unsafe {
            ffi::glx::SwapBuffers(self.display, self.window)
        }
    }

    pub fn get_api(&self) -> ::Api {
        ::Api::OpenGl
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Drop for Context {
    fn drop(&mut self) {
        use std::ptr;

        unsafe {
            // we don't call MakeCurrent(0, 0) because we are not sure that the context
            // is still the current one
            ffi::glx::DestroyContext(self.display, self.context);
        }
    }
}
