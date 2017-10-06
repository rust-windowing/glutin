use ContextError;
use CreationError;
use GlAttributes;
use GlProfile;
use GlRequest;
use Api;
use Robustness;

use libc;
use libc::c_int;
use std::ffi::{CStr, CString};
use std::ptr;
use super::ffi;


pub struct PureContext {
    glx: ffi::glx::Glx,
    display: *mut ffi::Display,
    context: ffi::GLXContext,
}

// TODO: remove me
fn with_c_str<F, T>(s: &str, f: F) -> T where F: FnOnce(*const libc::c_char) -> T {
    use std::ffi::CString;
    let c_str = CString::new(s.as_bytes().to_vec()).unwrap();
    f(c_str.as_ptr())
}

impl PureContext {
    pub fn new<'a>(
        glx: ffi::glx::Glx,
        xlib: &'a ffi::Xlib,
        opengl: &'a GlAttributes<ffi::GLXContext>,
        display: *mut ffi::Display,
        screen_id: libc::c_int,
    ) -> Result<PureContextPrototype<'a>, CreationError>
    {
        // TL;DR: https://www.virtualbox.org/ticket/8293
        let (mut major, mut minor) = (0, 0);
        unsafe {
            glx.QueryVersion(display as *mut _, &mut major, &mut minor);
        }

        // loading the list of extensions
        let extensions = unsafe {
            let extensions = glx.QueryExtensionsString(display as *mut _, screen_id);
            let extensions = CStr::from_ptr(extensions).to_bytes().to_vec();
            String::from_utf8(extensions).unwrap()
        };

        // generating the dummy config
        let fb_config = unsafe {
            try!(choose_fbconfig(&glx, xlib, display, screen_id)
                .map_err(|_| CreationError::NoAvailablePixelFormat))
        };

        Ok(PureContextPrototype {
            glx,
            extensions,
            xlib,
            opengl,
            display,
            fb_config,
        })
    }

    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        // TODO: glutin needs some internal changes for proper error recovery
        let res = self.glx.MakeCurrent(self.display as *mut _, 0, self.context);
        if res == 0 {
            panic!("glx::MakeCurrent failed");
        }
        Ok(())
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        unsafe { self.glx.GetCurrentContext() == self.context }
    }

    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        unsafe {
            self.glx.GetProcAddress(addr as *const _) as *const _
        }
    }

    #[inline]
    pub fn get_api(&self) -> ::Api {
        ::Api::OpenGl
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::GLXContext {
        self.context
    }
}

unsafe impl Send for PureContext {}
unsafe impl Sync for PureContext {}

impl Drop for PureContext {
    fn drop(&mut self) {
        unsafe {
            if self.is_current() {
                self.glx.MakeCurrent(self.display as *mut _, 0, ptr::null_mut());
            }

            self.glx.DestroyContext(self.display as *mut _, self.context);
        }
    }
}

impl<'a> super::ContextPrototype<'a> {
    pub(crate) fn finish_pure(&self) -> Result<ffi::GLXContext, CreationError> {
        let opengl = self.opengl.clone().map_sharing(|c| unsafe { c.raw_handle() });
        let puree = PureContextPrototype {
            glx: self.glx.clone(),
            extensions: self.extensions.clone(),
            xlib: self.xlib,
            opengl: &opengl,
            display: self.display,
            fb_config: self.fb_config
        };
        puree.finish_impl(Some(&self.visual_infos))
    }
}

pub struct PureContextPrototype<'a> {
    glx: ffi::glx::Glx,
    extensions: String,
    xlib: &'a ffi::Xlib,
    opengl: &'a GlAttributes<ffi::GLXContext>,
    display: *mut ffi::Display,
    fb_config: ffi::glx::types::GLXFBConfig,
}

impl<'a> PureContextPrototype<'a> {
    fn finish_impl(
        &self,
        visual_infos: Option<&ffi::XVisualInfo>,
    ) -> Result<ffi::GLXContext, CreationError> {
        let share = self.opengl.sharing.unwrap_or(ptr::null());

        // loading the extra GLX functions
        let extra_functions = ffi::glx_extra::Glx::load_with(|addr| {
            with_c_str(addr, |s| {
                unsafe { self.glx.GetProcAddress(s as *const u8) as *const _ }
            })
        });

        // creating GL context
        match self.opengl.version {
            GlRequest::Latest => {
                let opengl_versions = [(4, 5), (4, 4), (4, 3), (4, 2), (4, 1), (4, 0),
                                       (3, 3), (3, 2), (3, 1)];
                loop {
                    // Try all OpenGL versions in descending order because some non-compliant
                    // drivers don't return the latest supported version but the one requested
                    for opengl_version in opengl_versions.iter()
                    {
                        match create_context(&self.glx, &extra_functions, &self.extensions, &self.xlib,
                                             *opengl_version, self.opengl.profile,
                                             self.opengl.debug, self.opengl.robustness, share,
                                             self.display, self.fb_config, visual_infos)
                        {
                            Ok(x) => return Ok(x),
                            Err(_) => continue,
                        }
                    }
                    return create_context(&self.glx, &extra_functions, &self.extensions, &self.xlib, (1, 0),
                                          self.opengl.profile, self.opengl.debug,
                                          self.opengl.robustness, share,
                                          self.display, self.fb_config, visual_infos);
                }
            }
            GlRequest::Specific(Api::OpenGl, (major, minor)) => {
                create_context(&self.glx, &extra_functions, &self.extensions, &self.xlib, (major, minor),
                               self.opengl.profile, self.opengl.debug,
                               self.opengl.robustness, share, self.display, self.fb_config,
                               visual_infos)
            }
            GlRequest::Specific(_, _) => {
                panic!("Only OpenGL is supported")
            }
            GlRequest::GlThenGles { opengl_version: (major, minor), .. } => {
                create_context(&self.glx, &extra_functions, &self.extensions, &self.xlib, (major, minor),
                               self.opengl.profile, self.opengl.debug,
                               self.opengl.robustness, share, self.display, self.fb_config,
                               visual_infos)
            }
        }
    }

    pub fn finish(self) -> Result<PureContext, CreationError> {
        let context = self.finish_impl(None)?;
        Ok(PureContext {
            glx: self.glx,
            display: self.display,
            context,
        })
    }
}

extern fn x_error_callback(_dpy: *mut ffi::Display, _err: *mut ffi::XErrorEvent) -> i32
{
    0
}


fn create_context(glx: &ffi::glx::Glx, extra_functions: &ffi::glx_extra::Glx, extensions: &str, xlib: &ffi::Xlib,
                  version: (u8, u8), profile: Option<GlProfile>, debug: bool,
                  robustness: Robustness, share: ffi::GLXContext, display: *mut ffi::Display,
                  fb_config: ffi::glx::types::GLXFBConfig,
                  visual_infos: Option<&ffi::XVisualInfo>)
                  -> Result<ffi::GLXContext, CreationError>
{
    unsafe {
        let old_callback = (xlib.XSetErrorHandler)(Some(x_error_callback));
        let context = if extensions.split(' ').any(|i| i == "GLX_ARB_create_context") {
            let mut attributes = Vec::with_capacity(9);

            attributes.push(ffi::glx_extra::CONTEXT_MAJOR_VERSION_ARB as c_int);
            attributes.push(version.0 as c_int);
            attributes.push(ffi::glx_extra::CONTEXT_MINOR_VERSION_ARB as c_int);
            attributes.push(version.1 as c_int);

            if let Some(profile) = profile {
                let flag = match profile {
                    GlProfile::Compatibility =>
                        ffi::glx_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                    GlProfile::Core =>
                        ffi::glx_extra::CONTEXT_CORE_PROFILE_BIT_ARB,
                };

                attributes.push(ffi::glx_extra::CONTEXT_PROFILE_MASK_ARB as c_int);
                attributes.push(flag as c_int);
            }

            let flags = {
                let mut flags = 0;

                // robustness
                if extensions.split(' ').any(|i| i == "GLX_ARB_create_context_robustness") {
                    match robustness {
                        Robustness::RobustNoResetNotification | Robustness::TryRobustNoResetNotification => {
                            attributes.push(ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                            attributes.push(ffi::glx_extra::NO_RESET_NOTIFICATION_ARB as c_int);
                            flags = flags | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                        },
                        Robustness::RobustLoseContextOnReset | Robustness::TryRobustLoseContextOnReset => {
                            attributes.push(ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                            attributes.push(ffi::glx_extra::LOSE_CONTEXT_ON_RESET_ARB as c_int);
                            flags = flags | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                        },
                        Robustness::NotRobust => (),
                        Robustness::NoError => (),
                    }
                } else {
                    match robustness {
                        Robustness::RobustNoResetNotification | Robustness::RobustLoseContextOnReset => {
                            return Err(CreationError::RobustnessNotSupported);
                        },
                        _ => ()
                    }
                }

                if debug {
                    flags = flags | ffi::glx_extra::CONTEXT_DEBUG_BIT_ARB as c_int;
                }

                flags
            };

            attributes.push(ffi::glx_extra::CONTEXT_FLAGS_ARB as c_int);
            attributes.push(flags);

            attributes.push(0);

            extra_functions.CreateContextAttribsARB(display as *mut _, fb_config, share, 1,
                                                    attributes.as_ptr())

        } else if let Some(vis) = visual_infos {
            glx.CreateContext(display as *mut _, vis as *const _ as *mut _, share, 1)
        } else {
            return Err(CreationError::NotSupported);
        };

        (xlib.XSetErrorHandler)(old_callback);

        if context.is_null() {
            // TODO: check for errors and return `OpenGlVersionNotSupported`
            return Err(CreationError::OsError(format!("GL context creation failed")));
        }

        Ok(context)
    }
}

unsafe fn choose_fbconfig(
    glx: &ffi::glx::Glx,
    xlib: &ffi::Xlib,
    display: *mut ffi::Display,
    screen_id: libc::c_int,
) -> Result<ffi::glx::types::GLXFBConfig, ()>
{
    let descriptor = [
        ffi::glx::X_RENDERABLE as c_int, 0,
        ffi::glx::X_VISUAL_TYPE as c_int, ffi::glx::NONE as c_int,
        ffi::glx::DRAWABLE_TYPE as c_int, 0,
        0
    ];

    // calling glXChooseFBConfig
    let mut num_configs = 1;
    let configs = glx.ChooseFBConfig(
        display as *mut _,
        screen_id,
        descriptor.as_ptr(),
        &mut num_configs,
    );
    if configs.is_null() || num_configs == 0 { return Err(()); }

    let config = *configs;
    (xlib.XFree)(configs as *mut _);

    Ok(config)
}
