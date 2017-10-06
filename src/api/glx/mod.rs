#![cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]

use ContextError;
use CreationError;
use GlAttributes;
use PixelFormat;
use PixelFormatRequirements;
use ReleaseBehavior;

use libc;
use libc::c_int;
use std::ffi::{CStr, CString};
use std::{mem, ptr, slice};

mod puree;

pub use self::puree::{PureContext, PureContextPrototype};

pub mod ffi {
    pub use x11_dl::xlib::*;
    pub use self::glx::types::GLXContext;

    /// GLX bindings
    pub mod glx {
        include!(concat!(env!("OUT_DIR"), "/glx_bindings.rs"));
    }

    /// Functions that are not necessarly always available
    pub mod glx_extra {
        include!(concat!(env!("OUT_DIR"), "/glx_extra_bindings.rs"));
    }
}

pub struct Context {
    glx: ffi::glx::Glx,
    display: *mut ffi::Display,
    window: ffi::Window,
    context: ffi::GLXContext,
    pixel_format: PixelFormat,
}

// TODO: remove me
fn with_c_str<F, T>(s: &str, f: F) -> T where F: FnOnce(*const libc::c_char) -> T {
    use std::ffi::CString;
    let c_str = CString::new(s.as_bytes().to_vec()).unwrap();
    f(c_str.as_ptr())
}

impl Context {
    pub fn new<'a>(
        glx: ffi::glx::Glx,
        xlib: &'a ffi::Xlib,
        pf_reqs: &PixelFormatRequirements,
        opengl: &'a GlAttributes<&'a Context>,
        display: *mut ffi::Display,
        screen_id: libc::c_int,
        transparent: bool,
    ) -> Result<ContextPrototype<'a>, CreationError>
    {
        // This is completely ridiculous, but VirtualBox's OpenGL driver needs some call handled by
        // *it* (i.e. not Mesa) to occur before anything else can happen. That is because
        // VirtualBox's OpenGL driver is going to apply binary patches to Mesa in the DLL
        // constructor and until it's loaded it won't have a chance to do that.
        //
        // The easiest way to do this is to just call `glXQueryVersion()` before doing anything
        // else. See: https://www.virtualbox.org/ticket/8293
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

        // finding the pixel format we want
        let (fb_config, pixel_format) = unsafe {
            try!(choose_fbconfig(&glx, &extensions, xlib, display, screen_id, pf_reqs, transparent)
                .map_err(|_| CreationError::NoAvailablePixelFormat))
        };

        // getting the visual infos
        let visual_infos: ffi::glx::types::XVisualInfo = unsafe {
            let vi = glx.GetVisualFromFBConfig(display as *mut _, fb_config);
            if vi.is_null() {
                return Err(CreationError::OsError(format!("glxGetVisualFromFBConfig failed")));
            }
            let vi_copy = ptr::read(vi as *const _);
            (xlib.XFree)(vi as *mut _);
            vi_copy
        };

        Ok(ContextPrototype {
            glx,
            extensions,
            xlib,
            opengl,
            display,
            fb_config,
            visual_infos: unsafe { mem::transmute(visual_infos) },
            pixel_format,
        })
    }

    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        // TODO: glutin needs some internal changes for proper error recovery
        let res = self.glx.MakeCurrent(self.display as *mut _, self.window, self.context);
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
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        // TODO: glutin needs some internal changes for proper error recovery
        unsafe { self.glx.SwapBuffers(self.display as *mut _, self.window); }
        Ok(())
    }

    #[inline]
    pub fn get_api(&self) -> ::Api {
        ::Api::OpenGl
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pixel_format.clone()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::GLXContext {
        self.context
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            if self.is_current() {
                self.glx.MakeCurrent(self.display as *mut _, 0, ptr::null_mut());
            }

            self.glx.DestroyContext(self.display as *mut _, self.context);
        }
    }
}

pub struct ContextPrototype<'a> {
    glx: ffi::glx::Glx,
    extensions: String,
    xlib: &'a ffi::Xlib,
    opengl: &'a GlAttributes<&'a Context>,
    display: *mut ffi::Display,
    fb_config: ffi::glx::types::GLXFBConfig,
    visual_infos: ffi::XVisualInfo,
    pixel_format: PixelFormat,
}

impl<'a> ContextPrototype<'a> {
    #[inline]
    pub fn get_visual_infos(&self) -> &ffi::XVisualInfo {
        &self.visual_infos
    }

    pub fn finish(self, window: ffi::Window) -> Result<Context, CreationError> {
        // loading the extra GLX functions
        let extra_functions = ffi::glx_extra::Glx::load_with(|addr| {
            with_c_str(addr, |s| {
                unsafe { self.glx.GetProcAddress(s as *const u8) as *const _ }
            })
        });

        // creating GL context
        let context = self.finish_pure()?;

        // vsync
        if self.opengl.vsync {
            unsafe { self.glx.MakeCurrent(self.display as *mut _, window, context) };

            if extra_functions.SwapIntervalEXT.is_loaded() {
                // this should be the most common extension
                unsafe {
                    extra_functions.SwapIntervalEXT(self.display as *mut _, window, 1);
                }

                // checking that it worked
                // TODO: handle this
                /*if self.builder.strict {
                    let mut swap = unsafe { mem::uninitialized() };
                    unsafe {
                        self.glx.QueryDrawable(self.display as *mut _, window,
                                               ffi::glx_extra::SWAP_INTERVAL_EXT as i32,
                                               &mut swap);
                    }

                    if swap != 1 {
                        return Err(CreationError::OsError(format!("Couldn't setup vsync: expected \
                                                    interval `1` but got `{}`", swap)));
                    }
                }*/

            // GLX_MESA_swap_control is not official
            /*} else if extra_functions.SwapIntervalMESA.is_loaded() {
                unsafe {
                    extra_functions.SwapIntervalMESA(1);
                }*/

            } else if extra_functions.SwapIntervalSGI.is_loaded() {
                unsafe {
                    extra_functions.SwapIntervalSGI(1);
                }

            }/* else if self.builder.strict {
                // TODO: handle this
                return Err(CreationError::OsError(format!("Couldn't find any available vsync extension")));
            }*/

            unsafe { self.glx.MakeCurrent(self.display as *mut _, 0, ptr::null()) };
        }

        Ok(Context {
            glx: self.glx,
            display: self.display,
            window: window,
            context: context,
            pixel_format: self.pixel_format,
        })
    }
}

/// Enumerates all available FBConfigs
unsafe fn choose_fbconfig(glx: &ffi::glx::Glx, extensions: &str, xlib: &ffi::Xlib,
                          display: *mut ffi::Display, screen_id: libc::c_int,
                          reqs: &PixelFormatRequirements, transparent: bool)
                          -> Result<(ffi::glx::types::GLXFBConfig, PixelFormat), ()>
{
    let descriptor = {
        let mut out: Vec<c_int> = Vec::with_capacity(37);

        out.push(ffi::glx::X_RENDERABLE as c_int);
        out.push(1);

        out.push(ffi::glx::X_VISUAL_TYPE as c_int);
        out.push(ffi::glx::TRUE_COLOR as c_int);

        out.push(ffi::glx::DRAWABLE_TYPE as c_int);
        out.push(ffi::glx::WINDOW_BIT as c_int);

        out.push(ffi::glx::RENDER_TYPE as c_int);
        if reqs.float_color_buffer {
            if extensions.split(' ').find(|&i| i == "GLX_ARB_fbconfig_float").is_some() {
                out.push(ffi::glx_extra::RGBA_FLOAT_BIT_ARB as c_int);
            } else {
                return Err(());
            }
        } else {
            out.push(ffi::glx::RGBA_BIT as c_int);
        }

        if let Some(color) = reqs.color_bits {
            out.push(ffi::glx::RED_SIZE as c_int);
            out.push((color / 3) as c_int);
            out.push(ffi::glx::GREEN_SIZE as c_int);
            out.push((color / 3 + if color % 3 != 0 { 1 } else { 0 }) as c_int);
            out.push(ffi::glx::BLUE_SIZE as c_int);
            out.push((color / 3 + if color % 3 == 2 { 1 } else { 0 }) as c_int);
        }

        if let Some(alpha) = reqs.alpha_bits {
            out.push(ffi::glx::ALPHA_SIZE as c_int);
            out.push(alpha as c_int);
        }

        if let Some(depth) = reqs.depth_bits {
            out.push(ffi::glx::DEPTH_SIZE as c_int);
            out.push(depth as c_int);
        }

        if let Some(stencil) = reqs.stencil_bits {
            out.push(ffi::glx::STENCIL_SIZE as c_int);
            out.push(stencil as c_int);
        }

        let double_buffer = reqs.double_buffer.unwrap_or(true);
        out.push(ffi::glx::DOUBLEBUFFER as c_int);
        out.push(if double_buffer { 1 } else { 0 });

        if let Some(multisampling) = reqs.multisampling {
            if extensions.split(' ').find(|&i| i == "GLX_ARB_multisample").is_some() {
                out.push(ffi::glx_extra::SAMPLE_BUFFERS_ARB as c_int);
                out.push(if multisampling == 0 { 0 } else { 1 });
                out.push(ffi::glx_extra::SAMPLES_ARB as c_int);
                out.push(multisampling as c_int);
            } else {
                return Err(());
            }
        }

        out.push(ffi::glx::STEREO as c_int);
        out.push(if reqs.stereoscopy { 1 } else { 0 });

        if reqs.srgb {
            if extensions.split(' ').find(|&i| i == "GLX_ARB_framebuffer_sRGB").is_some() {
                out.push(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as c_int);
                out.push(1);
            } else if extensions.split(' ').find(|&i| i == "GLX_EXT_framebuffer_sRGB").is_some() {
                out.push(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as c_int);
                out.push(1);
            } else {
                return Err(());
            }
        }

        match reqs.release_behavior {
            ReleaseBehavior::Flush => (),
            ReleaseBehavior::None => {
                if extensions.split(' ').find(|&i| i == "GLX_ARB_context_flush_control").is_some() {
                    out.push(ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_ARB as c_int);
                    out.push(ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_NONE_ARB as c_int);
                }
            },
        }

        out.push(ffi::glx::CONFIG_CAVEAT as c_int);
        out.push(ffi::glx::DONT_CARE as c_int);

        out.push(0);
        out
    };

    // calling glXChooseFBConfig
    let fb_config = {
        let mut num_configs = 1;
        let configs = glx.ChooseFBConfig(display as *mut _, screen_id, descriptor.as_ptr(),
                                        &mut num_configs);
        if configs.is_null() { return Err(()); }
        if num_configs == 0 { return Err(()); }

        let config = if transparent {
            let configs = slice::from_raw_parts(configs, num_configs as usize);
            configs.iter().find(|&config| {
                let vi = glx.GetVisualFromFBConfig(display as *mut _, *config);
                // Transparency was requested, so only choose configs with 32 bits for RGBA.
                let found = !vi.is_null() && (*vi).depth == 32;
                (xlib.XFree)(vi as *mut _);

                found
            })
        } else {
            Some(&*configs)
        };

        let res = if let Some(&conf) = config {
            Ok(conf)
        } else {
            Err(())
        };

        (xlib.XFree)(configs as *mut _);
        res?
    };

    let get_attrib = |attrib: c_int| -> i32 {
        let mut value = 0;
        glx.GetFBConfigAttrib(display as *mut _, fb_config, attrib, &mut value);
        // TODO: check return value
        value
    };

    let pf_desc = PixelFormat {
        hardware_accelerated: get_attrib(ffi::glx::CONFIG_CAVEAT as c_int) !=
                                                            ffi::glx::SLOW_CONFIG as c_int,
        color_bits: get_attrib(ffi::glx::RED_SIZE as c_int) as u8 +
                    get_attrib(ffi::glx::GREEN_SIZE as c_int) as u8 +
                    get_attrib(ffi::glx::BLUE_SIZE as c_int) as u8,
        alpha_bits: get_attrib(ffi::glx::ALPHA_SIZE as c_int) as u8,
        depth_bits: get_attrib(ffi::glx::DEPTH_SIZE as c_int) as u8,
        stencil_bits: get_attrib(ffi::glx::STENCIL_SIZE as c_int) as u8,
        stereoscopy: get_attrib(ffi::glx::STEREO as c_int) != 0,
        double_buffer: get_attrib(ffi::glx::DOUBLEBUFFER as c_int) != 0,
        multisampling: if get_attrib(ffi::glx::SAMPLE_BUFFERS as c_int) != 0 {
            Some(get_attrib(ffi::glx::SAMPLES as c_int) as u16)
        } else {
            None
        },
        srgb: get_attrib(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as c_int) != 0 ||
              get_attrib(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as c_int) != 0,
    };

    Ok((fb_config, pf_desc))
}
