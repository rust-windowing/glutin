#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

pub mod ffi;
mod glx;
mod make_current_guard;

pub use self::glx::{Glx, GlxExtra};
use self::make_current_guard::MakeCurrentGuard;

use crate::config::{Api, ConfigAttribs, ConfigWrapper, ConfigsFinder, SwapIntervalRange, Version};
use crate::context::{ContextBuilderWrapper, GlProfile, ReleaseBehaviour, Robustness};
use crate::surface::{PBuffer, Pixmap, SurfaceType, SurfaceTypeTrait, Window};
use crate::utils::NoCmp;

use glutin_interface::{NativeDisplay, NativeWindow, NativeWindowSource, RawDisplay, RawWindow};
use glutin_x11_sym::Display as X11Display;
use winit_types::dpi;
use winit_types::error::{Error, ErrorType};
use winit_types::platform::OsError;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw;
use std::slice;
use std::sync::Arc;

lazy_static! {
    pub static ref GLX: Result<Glx, Error> = Glx::new();
    pub static ref GLX_EXTRA: Result<GlxExtra, Error> = GLX
        .as_ref()
        .map(|glx| GlxExtra::new(glx))
        .map_err(|err| err.clone());
}

#[derive(Debug, PartialEq, Eq)]
pub struct Display {
    display: Arc<X11Display>,
    screen: raw::c_int,
    extensions: Vec<String>,
    version: (u8, u8),
}

impl Display {
    #[inline]
    pub fn new(screen: raw::c_int, display: &Arc<X11Display>) -> Result<Arc<Self>, Error> {
        let glx = GLX.as_ref().unwrap();
        // This is completely ridiculous, but VirtualBox's OpenGL driver needs
        // some call handled by *it* (i.e. not Mesa) to occur before
        // anything else can happen. That is because VirtualBox's OpenGL
        // driver is going to apply binary patches to Mesa in the DLL
        // constructor and until it's loaded it won't have a chance to do that.
        //
        // The easiest way to do this is to just call `glXQueryVersion()` before
        // doing anything else. See: https://www.virtualbox.org/ticket/8293
        let (mut major, mut minor) = (0, 0);
        unsafe {
            glx.QueryVersion(***display as *mut _, &mut major, &mut minor);
        }

        // loading the list of extensions
        let extensions = Self::load_extensions(display, screen)?
            .split(' ')
            .map(|e| e.to_string())
            .collect::<Vec<_>>();

        Ok(Arc::new(Display {
            display: Arc::clone(display),
            screen,
            extensions,
            version: (major as _, minor as _),
        }))
    }

    #[inline]
    fn load_extensions(disp: &Arc<X11Display>, screen: raw::c_int) -> Result<String, Error> {
        unsafe {
            let glx = GLX.as_ref().unwrap();
            let extensions = glx.QueryExtensionsString(***disp as *mut _, screen);
            if extensions.is_null() {
                return Err(make_oserror!(OsError::Misc(
                    "`glXQueryExtensionsString` found no glX extensions".to_string(),
                )));
            }
            let extensions = CStr::from_ptr(extensions).to_bytes().to_vec();
            Ok(String::from_utf8(extensions).unwrap())
        }
    }

    #[inline]
    fn has_extension(&self, e: &str) -> bool {
        self.extensions.iter().find(|s| s == &e).is_some()
    }
}

impl Deref for Display {
    type Target = Arc<X11Display>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

// FIXME why is this a macro again?
macro_rules! attrib {
    ($glx:expr, $disp:expr, $conf:expr, $attr:expr $(,)?) => {{
        let mut value = 0;
        let res = unsafe {
            $glx.GetFBConfigAttrib(****$disp as *mut _, $conf, $attr as raw::c_int, &mut value)
        };
        match res {
            0 => Ok(value),
            err => Err(make_oserror!(OsError::Misc(format!(
                "glxGetFBConfigAttrib failed for {:?} with 0x{:x}",
                $conf, err,
            )))),
        }
    }};
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    display: Arc<Display>,
    config: ffi::glx::types::GLXFBConfig,
    visual_info: NoCmp<ffi::XVisualInfo>,
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

impl Config {
    #[inline]
    pub fn new<F>(
        cf: &ConfigsFinder,
        screen: raw::c_int,
        disp: &Arc<X11Display>,
        mut conf_selector: F,
    ) -> Result<Vec<(ConfigAttribs, Config)>, Error>
    where
        F: FnMut(
            Vec<ffi::glx::types::GLXFBConfig>,
        ) -> Vec<Result<(ffi::glx::types::GLXFBConfig, ffi::XVisualInfo), Error>>,
    {
        let xlib = syms!(XLIB);
        let glx = GLX.as_ref().map_err(|e| e.clone())?;
        let disp = Display::new(screen, disp)?;
        let mut errors = make_error!(ErrorType::NoAvailableConfig);

        if cf.must_support_surfaceless {
            return Err(make_error!(ErrorType::NotSupported(
                "EGL surfaceless not supported with GLX".to_string(),
            )));
        }

        let descriptor = {
            let mut out: Vec<raw::c_int> = Vec::with_capacity(37);

            out.push(ffi::glx::X_RENDERABLE as raw::c_int);
            out.push(1);

            if let Some(xid) = cf.plat_attr.x11_visual_xid {
                // getting the visual infos
                let fvi = crate::platform_impl::x11::utils::get_visual_info_from_xid(&**disp, xid)?;

                out.push(ffi::glx::X_VISUAL_TYPE as raw::c_int);
                out.push(fvi.class as raw::c_int);

                out.push(ffi::glx::VISUAL_ID as raw::c_int);
                out.push(xid as raw::c_int);
            } else {
                out.push(ffi::glx::X_VISUAL_TYPE as raw::c_int);
                out.push(ffi::glx::TRUE_COLOR as raw::c_int);
            }

            out.push(ffi::glx::DRAWABLE_TYPE as raw::c_int);
            let mut surface_type = 0;
            if cf.must_support_windows {
                surface_type = surface_type | ffi::glx::WINDOW_BIT;
            }
            if cf.must_support_pbuffers {
                surface_type = surface_type | ffi::glx::PBUFFER_BIT;
            }
            if cf.must_support_pixmaps {
                surface_type = surface_type | ffi::glx::PIXMAP_BIT;
            }
            out.push(surface_type as raw::c_int);

            if let Some(color) = cf.color_bits {
                out.push(ffi::glx::RED_SIZE as raw::c_int);
                out.push((color / 3) as raw::c_int);
                out.push(ffi::glx::GREEN_SIZE as raw::c_int);
                out.push((color / 3 + if color % 3 != 0 { 1 } else { 0 }) as raw::c_int);
                out.push(ffi::glx::BLUE_SIZE as raw::c_int);
                out.push((color / 3 + if color % 3 == 2 { 1 } else { 0 }) as raw::c_int);
            }

            if let Some(alpha) = cf.alpha_bits {
                out.push(ffi::glx::ALPHA_SIZE as raw::c_int);
                out.push(alpha as raw::c_int);
            }

            out.push(ffi::glx::RENDER_TYPE as raw::c_int);
            match cf.float_color_buffer {
                Some(true) => {
                    if !disp.has_extension("GLX_ARB_fbconfig_float") {
                        return Err(errors);
                    }
                    out.push(ffi::glx_extra::RGBA_FLOAT_BIT_ARB as raw::c_int);
                }
                _ => {
                    out.push(ffi::glx::RGBA_BIT as raw::c_int);
                }
            }

            if let Some(depth) = cf.depth_bits {
                out.push(ffi::glx::DEPTH_SIZE as raw::c_int);
                out.push(depth as raw::c_int);
            }

            if let Some(stencil) = cf.stencil_bits {
                out.push(ffi::glx::STENCIL_SIZE as raw::c_int);
                out.push(stencil as raw::c_int);
            }

            let double_buffer = cf.double_buffer.unwrap_or(true);
            out.push(ffi::glx::DOUBLEBUFFER as raw::c_int);
            out.push(if double_buffer { 1 } else { 0 });

            if let Some(multisampling) = cf.multisampling {
                if !disp.has_extension("GLX_ARB_multisample") {
                    return Err(make_error!(ErrorType::NoAvailableConfig));
                }
                out.push(ffi::glx_extra::SAMPLE_BUFFERS_ARB as raw::c_int);
                out.push(if multisampling == 0 { 0 } else { 1 });
                out.push(ffi::glx_extra::SAMPLES_ARB as raw::c_int);
                out.push(multisampling as raw::c_int);
            }

            out.push(ffi::glx::STEREO as raw::c_int);
            out.push(if cf.stereoscopy { 1 } else { 0 });

            // The ARB ext says that if we don't pass GLX_FRAMEBUFFER_SRGB_CAPABLE_ARB
            // it is treated as don't care, which is what we want.
            //
            // The ARB ext was ammended to say so in
            // https://github.com/KhronosGroup/OpenGL-Registry/issues/199.
            //
            // The EXT ext doesn't specify, but given that they should both behave
            // (nearly) the same, it is safe to assume that this is also the case
            // for the EXT ext.
            if let Some(srgb) = cf.srgb {
                let srgb = if srgb { 1 } else { 0 };
                if disp.has_extension("GLX_ARB_framebuffer_sRGB") {
                    out.push(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as raw::c_int);
                    out.push(srgb);
                } else if disp.has_extension("GLX_EXT_framebuffer_sRGB") {
                    out.push(ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as raw::c_int);
                    out.push(srgb);
                } else {
                    return Err(make_error!(ErrorType::NoAvailableConfig));
                }
            }

            // FIXME mv context
            //match cf.release_behavior {
            //    ReleaseBehaviour::Flush => {
            //        if disp.has_extension("GLX_ARB_context_flush_control") {
            //            // With how shitty drivers are, never hurts to be explicit
            //            out.push(
            //                ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_ARB
            //                    as raw::c_int,
            //            );
            //            out.push(
            //                ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_FLUSH_ARB
            //                    as raw::c_int,
            //            );
            //        }
            //    },
            //    ReleaseBehaviour::None => {
            //        if !disp.has_extension("GLX_ARB_context_flush_control") {
            //            return Err(make_error!(ErrorType::FlushControlNotSupported));
            //        }
            //        out.push(
            //            ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_ARB
            //                as raw::c_int,
            //        );
            //        out.push(
            //            ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_NONE_ARB
            //                as raw::c_int,
            //        );
            //    }
            //}

            out.push(ffi::glx::CONFIG_CAVEAT as raw::c_int);
            out.push(ffi::glx::DONT_CARE as raw::c_int);

            out.push(0);
            out
        };

        // calling glXChooseFBConfig
        let mut num_confs = 0;
        let confs_ptr = unsafe {
            glx.ChooseFBConfig(
                ****disp as *mut _,
                screen,
                descriptor.as_ptr(),
                &mut num_confs,
            )
        };

        if let Err(err) = disp.check_errors() {
            errors.append(err);
            return Err(errors);
        }

        if confs_ptr.is_null() {
            return Err(errors);
        }

        if num_confs == 0 {
            return Err(errors);
        }

        let confs: Vec<ffi::glx::types::GLXFBConfig> = unsafe {
            let confs = slice::from_raw_parts(confs_ptr, num_confs as usize)
                .iter()
                .cloned()
                .collect();
            (xlib.XFree)(confs_ptr as *mut _);
            confs
        };

        let confs: Vec<_> = conf_selector(confs)
            .into_iter()
            .filter_map(|conf| match conf {
                Err(err) => {
                    errors.append(err);
                    return None;
                }
                Ok(conf) => Some(conf),
            })
            .map(|(conf, visual_info)| {
                // There is no way for us to know the SwapIntervalRange's max until the
                // drawable is made.
                //
                // 1000 is reasonable.
                let mut swap_interval_ranges = vec![
                    SwapIntervalRange::DontWait,
                    SwapIntervalRange::Wait(1..1000),
                ];
                if disp.has_extension("GLX_EXT_swap_control_tear") {
                    swap_interval_ranges.push(SwapIntervalRange::AdaptiveWait(1..1000));
                }

                let surf_type = attrib!(glx, disp, conf, ffi::glx::DRAWABLE_TYPE)? as u32;
                let attribs = ConfigAttribs {
                    version: cf.version,
                    supports_windows: (surf_type & ffi::glx::WINDOW_BIT) != 0,
                    supports_pixmaps: (surf_type & ffi::glx::PIXMAP_BIT) != 0,
                    supports_pbuffers: (surf_type & ffi::glx::PBUFFER_BIT) != 0,
                    supports_surfaceless: false,
                    hardware_accelerated: attrib!(glx, disp, conf, ffi::glx::CONFIG_CAVEAT)?
                        != ffi::glx::SLOW_CONFIG as raw::c_int,
                    color_bits: attrib!(glx, disp, conf, ffi::glx::RED_SIZE)? as u8
                        + attrib!(glx, disp, conf, ffi::glx::BLUE_SIZE)? as u8
                        + attrib!(glx, disp, conf, ffi::glx::GREEN_SIZE)? as u8,
                    alpha_bits: attrib!(glx, disp, conf, ffi::glx::ALPHA_SIZE)? as u8,
                    depth_bits: attrib!(glx, disp, conf, ffi::glx::DEPTH_SIZE)? as u8,
                    stencil_bits: attrib!(glx, disp, conf, ffi::glx::STENCIL_SIZE)? as u8,
                    stereoscopy: attrib!(glx, disp, conf, ffi::glx::STEREO)? != 0,
                    double_buffer: attrib!(glx, disp, conf, ffi::glx::DOUBLEBUFFER)? != 0,

                    multisampling: match disp.has_extension("GLX_ARB_multisample") {
                        false => None,
                        true => match attrib!(glx, disp, conf, ffi::glx::SAMPLE_BUFFERS)? {
                            0 => None,
                            _ => Some(attrib!(glx, disp, conf, ffi::glx::SAMPLES)? as u16),
                        },
                    },
                    srgb: match (
                        disp.has_extension("GLX_ARB_framebuffer_sRGB"),
                        disp.has_extension("GLX_EXT_framebuffer_sRGB"),
                    ) {
                        (true, _) => {
                            attrib!(
                                glx,
                                disp,
                                conf,
                                ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB
                            )? != 0
                        }
                        (_, true) => {
                            attrib!(
                                glx,
                                disp,
                                conf,
                                ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT
                            )? != 0
                        }
                        // Mesa prior to 2017 did not support sRGB contexts. It is sane to assume
                        // that if neither ext is implmented the config is most likely not sRGB.
                        //
                        // Of course, this might not be the case, but without either ext
                        // implemented there is no way for us to tell.
                        (_, _) => false,
                    },
                    swap_interval_ranges,
                };

                Ok((attribs, conf, visual_info))
            })
            // FIXME: Pleasing borrowck. Lokathor demands unrolling this loop.
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|conf| match conf {
                Err(err) => {
                    errors.append(err);
                    return None;
                }
                Ok(conf) => Some(conf),
            })
            .collect();

        if confs.is_empty() {
            return Err(errors);
        }

        if let Err(err) = disp.check_errors() {
            errors.append(err);
            return Err(errors);
        }

        Ok(confs
            .into_iter()
            .map(|(attribs, config, visual)| {
                (
                    attribs,
                    Config {
                        display: Arc::clone(&disp),
                        config,
                        visual_info: NoCmp(visual),
                    },
                )
            })
            .collect())
    }

    #[inline]
    pub fn display(&self) -> &Arc<X11Display> {
        &*self.display
    }

    #[inline]
    pub fn screen(&self) -> raw::c_int {
        self.display.screen
    }

    #[inline]
    pub fn get_visual_info(&self) -> ffi::XVisualInfo {
        *self.visual_info
    }
}

#[inline]
pub fn get_native_visual_id(
    disp: &Arc<X11Display>,
    conf: ffi::glx::types::GLXFBConfig,
) -> Result<ffi::VisualID, Error> {
    let glx = GLX.as_ref().unwrap();
    Ok(attrib!(glx, &disp, conf, ffi::glx::VISUAL_ID)? as ffi::VisualID)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Context {
    display: Arc<Display>,
    context: ffi::glx::types::GLXContext,
    config: ConfigWrapper<Config, ConfigAttribs>,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Drop for Context {
    fn drop(&mut self) {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            glx.DestroyContext(****self.display as *mut _, self.context);
        }

        self.display.check_errors().unwrap();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Surface<T: SurfaceTypeTrait> {
    display: Arc<Display>,
    surface: ffi::Drawable,
    config: ConfigWrapper<Config, ConfigAttribs>,
    phantom: PhantomData<T>,
}

unsafe impl<T: SurfaceTypeTrait> Send for Surface<T> {}
unsafe impl<T: SurfaceTypeTrait> Sync for Surface<T> {}

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    fn drop(&mut self) {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            match T::surface_type() {
                SurfaceType::Window => glx.DestroyWindow(****self.display as *mut _, self.surface),
                SurfaceType::PBuffer => {
                    glx.DestroyPbuffer(****self.display as *mut _, self.surface)
                }
                SurfaceType::Pixmap => {
                    if self.display.version >= (1, 3) {
                        glx.DestroyPixmap(****self.display as *mut _, self.surface)
                    } else {
                        glx.DestroyGLXPixmap(****self.display as *mut _, self.surface)
                    }
                }
            }
        }
        self.display.check_errors().unwrap();
    }
}

//#[derive(Debug)]
//pub struct Context {
//    xconn: Arc<XConnection>,
//    drawable: ffi::Window,
//    context: ffi::glx::types::GLXContext,
//    pixel_format: PixelFormat,
//}
//
//impl Context {
//    // transparent is `None` if window is raw.
//    pub fn new<'a>(
//        xconn: Arc<XConnection>,
//        cb: &'a ContextBuilderWrapper<&'a Context>,
//        screen_id: raw::c_int,
//        surface_type: SurfaceType,
//        transparent: Option<bool>,
//    ) -> Result<ContextPrototype<'a>, Error> {
//        // finding the pixel format we want
//        let (fb_config, pixel_format, visual_infos) = unsafe {
//            choose_fbconfig(
//                &extensions,
//                &xconn,
//                screen_id,
//                cb,
//                surface_type,
//                transparent,
//            )?
//        };
//
//        Ok(ContextPrototype {
//            extensions,
//            xconn,
//            gl_attr: &cb.gl_attr,
//            fb_config,
//            visual_infos: unsafe { std::mem::transmute(visual_infos) },
//            pixel_format,
//        })
//    }
//
//    unsafe fn check_make_current(
//        &self,
//        ret: Option<i32>,
//    ) -> Result<(), Error> {
//        if ret == Some(0) {
//            let err = self.xconn.check_errors();
//            Err(make_oserror!(OsError::Misc(format!(
//                "`glXMakeCurrent` failed: {:?}",
//                err
//            ))))
//        } else {
//            Ok(())
//        }
//    }
//
//    #[inline]
//    pub unsafe fn make_current(&self) -> Result<(), Error> {
//        let glx = GLX.as_ref().unwrap();
//        let res = glx.MakeCurrent(
//            self.xconn.display as *mut _,
//            self.drawable,
//            self.context,
//        );
//        self.check_make_current(Some(res))
//    }
//
//    #[inline]
//    pub unsafe fn make_not_current(&self) -> Result<(), Error> {
//        let glx = GLX.as_ref().unwrap();
//        if self.drawable == glx.GetCurrentDrawable()
//            || self.context == glx.GetCurrentContext()
//        {
//            let res = glx.MakeCurrent(
//                self.xconn.display as *mut _,
//                0,
//                std::ptr::null(),
//            );
//            self.check_make_current(Some(res))
//        } else {
//            self.check_make_current(None)
//        }
//    }
//
//    #[inline]
//    pub fn is_current(&self) -> bool {
//        let glx = GLX.as_ref().unwrap();
//        unsafe { glx.GetCurrentContext() == self.context }
//    }
//
//    #[inline]
//    pub fn get_api(&self) -> Api {
//        Api::OpenGl
//    }
//
//    #[inline]
//    pub unsafe fn raw_handle(&self) -> ffi::glx::types::GLXContext {
//        self.context
//    }
//
//    #[inline]
//    pub fn get_proc_address(&self, addr: &str) -> *const () {
//        let glx = GLX.as_ref().unwrap();
//        let addr = CString::new(addr.as_bytes()).unwrap();
//        let addr = addr.as_ptr();
//        unsafe { glx.GetProcAddress(addr as *const _) as *const _ }
//    }
//
//    #[inline]
//    pub fn swap_buffers(&self) -> Result<(), Error> {
//        let glx = GLX.as_ref().unwrap();
//        unsafe {
//            glx.SwapBuffers(self.xconn.display as *mut _, self.drawable);
//        }
//        if let Err(err) = self.xconn.check_errors() {
//            Err(make_oserror!(OsError::Misc(format!(
//                "`glXSwapBuffers` failed: {:?}",
//                err
//            ))))
//        } else {
//            Ok(())
//        }
//    }
//
//    #[inline]
//    pub fn get_pixel_format(&self) -> PixelFormat {
//        self.pixel_format.clone()
//    }
//}
//
//unsafe impl Send for Context {}
//unsafe impl Sync for Context {}
//
//impl Drop for Context {
//    fn drop(&mut self) {
//        let glx = GLX.as_ref().unwrap();
//        unsafe {
//            // See `drop` for `crate::api::egl::Context` for rationale.
//            let mut guard =
//                MakeCurrentGuard::new(&self.xconn, self.drawable, self.context)
//                    .unwrap();
//
//            let gl_finish_fn = self.get_proc_address("glFinish");
//            assert!(gl_finish_fn != std::ptr::null());
//            let gl_finish_fn =
//                std::mem::transmute::<_, extern "system" fn()>(gl_finish_fn);
//            gl_finish_fn();
//
//            if guard.old_context() == Some(self.context) {
//                guard.invalidate()
//            }
//            std::mem::drop(guard);
//
//            glx.DestroyContext(self.xconn.display as *mut _, self.context);
//        }
//    }
//}
//
//#[derive(Debug)]
//pub struct ContextPrototype<'a> {
//    extensions: String,
//    xconn: Arc<XConnection>,
//    gl_attr: &'a GlAttributes<&'a Context>,
//    fb_config: ffi::glx::types::GLXFBConfig,
//    visual_infos: ffi::XVisualInfo,
//    pixel_format: PixelFormat,
//}
//
//impl<'a> ContextPrototype<'a> {
//    #[inline]
//    pub fn get_visual_infos(&self) -> &ffi::XVisualInfo {
//        &self.visual_infos
//    }
//
//    // creating GL context
//    fn create_context(
//        &self,
//    ) -> Result<(ffi::glx_extra::Glx, ffi::glx::types::GLXContext), Error> {
//        let glx = GLX.as_ref().unwrap();
//        let share = match self.gl_attr.sharing {
//            Some(ctx) => ctx.context,
//            None => std::ptr::null(),
//        };
//
//        let context = match self.gl_attr.version {
//            GlRequest::Latest => {
//                let opengl_versions = [
//                    GlVersion(4, 6),
//                    GlVersion(4, 5),
//                    GlVersion(4, 4),
//                    GlVersion(4, 3),
//                    GlVersion(4, 2),
//                    GlVersion(4, 1),
//                    GlVersion(4, 0),
//                    GlVersion(3, 3),
//                    GlVersion(3, 2),
//                    GlVersion(3, 1),
//                ];
//                let ctx;
//                'outer: loop {
//                    // Try all OpenGL versions in descending order because some
//                    // non-compliant drivers don't return
//                    // the latest supported version but the one requested
//                    for opengl_version in opengl_versions.iter() {
//                        match create_context(
//                            &extra_functions,
//                            &self.extensions,
//                            &self.xconn.xlib,
//                            *opengl_version,
//                            self.gl_attr.profile,
//                            self.gl_attr.debug,
//                            self.gl_attr.robustness,
//                            share,
//                            self.xconn.display,
//                            self.fb_config,
//                            &self.visual_infos,
//                        ) {
//                            Ok(x) => {
//                                ctx = x;
//                                break 'outer;
//                            }
//                            Err(_) => continue,
//                        }
//                    }
//                    ctx = create_context(
//                        &extra_functions,
//                        &self.extensions,
//                        &self.xconn.xlib,
//                        GlVersion(1, 0),
//                        self.gl_attr.profile,
//                        self.gl_attr.debug,
//                        self.gl_attr.robustness,
//                        share,
//                        self.xconn.display,
//                        self.fb_config,
//                        &self.visual_infos,
//                    )?;
//                    break;
//                }
//                ctx
//            }
//            GlRequest::Specific(Api::OpenGl, opengl_version) => create_context(
//                &extra_functions,
//                &self.extensions,
//                &self.xconn.xlib,
//                opengl_version,
//                self.gl_attr.profile,
//                self.gl_attr.debug,
//                self.gl_attr.robustness,
//                share,
//                self.xconn.display,
//                self.fb_config,
//                &self.visual_infos,
//            )?,
//            GlRequest::Specific(_, _) => panic!("Only OpenGL is supported"),
//            GlRequest::GlThenGles {
//                opengl_version,
//                ..
//            } => create_context(
//                &extra_functions,
//                &self.extensions,
//                &self.xconn.xlib,
//                opengl_version,
//                self.gl_attr.profile,
//                self.gl_attr.debug,
//                self.gl_attr.robustness,
//                share,
//                self.xconn.display,
//                self.fb_config,
//                &self.visual_infos,
//            )?,
//        };
//
//        Ok((extra_functions, context))
//    }
//
//    pub fn finish_pbuffer(
//        self,
//        size: dpi::PhysicalSize<u32>,
//    ) -> Result<Context, Error> {
//        let glx = GLX.as_ref().unwrap();
//        let size: (u32, u32) = size.into();
//        let (_extra_functions, context) = self.create_context()?;
//
//        let attributes: Vec<raw::c_int> = vec![
//            ffi::glx::PBUFFER_WIDTH as raw::c_int,
//            size.0 as raw::c_int,
//            ffi::glx::PBUFFER_HEIGHT as raw::c_int,
//            size.1 as raw::c_int,
//            0,
//        ];
//
//        let pbuffer = unsafe {
//            glx.CreatePbuffer(
//                self.xconn.display as *mut _,
//                self.fb_config,
//                attributes.as_ptr(),
//            )
//        };
//
//        Ok(Context {
//            xconn: self.xconn,
//            drawable: pbuffer,
//            context,
//            pixel_format: self.pixel_format,
//        })
//    }
//
//    pub fn finish(self, window: ffi::Window) -> Result<Context, Error> {
//        let glx = GLX.as_ref().unwrap();
//        let (extra_functions, context) = self.create_context()?;
//
//        // vsync
//        if self.gl_attr.vsync {
//            let _guard = MakeCurrentGuard::new(&self.xconn, window, context)?;
//
//            if check_ext(&self.extensions, "GLX_EXT_swap_control")
//                && extra_functions.SwapIntervalEXT.is_loaded()
//            {
//                // this should be the most common extension
//                unsafe {
//                    extra_functions.SwapIntervalEXT(
//                        self.xconn.display as *mut _,
//                        window,
//                        1,
//                    );
//                }
//
//                let mut swap = 0;
//                unsafe {
//                    glx.QueryDrawable(
//                        self.xconn.display as *mut _,
//                        window,
//                        ffi::glx_extra::SWAP_INTERVAL_EXT as i32,
//                        &mut swap,
//                    );
//                }
//
//                if swap != 1 {
//                    return Err(make_oserror!(OsError::Misc(format!(
//                        "Couldn't setup vsync: expected interval `1` but got `{}`",
//                        swap
//                    ))));
//                }
//            } else if check_ext(&self.extensions, "GLX_MESA_swap_control")
//                && extra_functions.SwapIntervalMESA.is_loaded()
//            {
//                unsafe {
//                    extra_functions.SwapIntervalMESA(1);
//                }
//            } else if check_ext(&self.extensions, "GLX_SGI_swap_control")
//                && extra_functions.SwapIntervalSGI.is_loaded()
//            {
//                unsafe {
//                    extra_functions.SwapIntervalSGI(1);
//                }
//            } else {
//                return Err(make_oserror!(OsError::Misc(
//                    "Couldn't find any available vsync extension".to_string(),
//                )));
//            }
//        }
//
//        Ok(Context {
//            xconn: self.xconn,
//            drawable: window,
//            context,
//            pixel_format: self.pixel_format,
//        })
//    }
//}
//
//extern "C" fn x_error_callback(
//    _dpy: *mut ffi::Display,
//    _err: *mut ffi::XErrorEvent,
//) -> i32 {
//    0
//}
//
//fn create_context(
//    extra_functions: &ffi::glx_extra::Glx,
//    extensions: &str,
//    xlib: &ffi::Xlib,
//    version: GlVersion,
//    profile: Option<GlProfile>,
//    debug: bool,
//    robustness: Robustness,
//    share: ffi::glx::types::GLXContext,
//    display: *mut ffi::Display,
//    fb_config: ffi::glx::types::GLXFBConfig,
//    visual_infos: &ffi::XVisualInfo,
//) -> Result<ffi::glx::types::GLXContext, Error> {
//    let glx = GLX.as_ref().unwrap();
//    unsafe {
//        let old_callback = (xlib.XSetErrorHandler)(Some(x_error_callback));
//        let context = if check_ext(extensions, "GLX_ARB_create_context") {
//            let mut attributes = Vec::with_capacity(9);
//
//            attributes
//                .push(ffi::glx_extra::CONTEXT_MAJOR_VERSION_ARB as raw::c_int);
//            attributes.push(version.0 as raw::c_int);
//            attributes
//                .push(ffi::glx_extra::CONTEXT_MINOR_VERSION_ARB as raw::c_int);
//            attributes.push(version.1 as raw::c_int);
//
//            if let Some(profile) = profile {
//                let flag = match profile {
//                    GlProfile::Compatibility => {
//                        ffi::glx_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB
//                    }
//                    GlProfile::Core => {
//                        ffi::glx_extra::CONTEXT_CORE_PROFILE_BIT_ARB
//                    }
//                };
//
//                attributes.push(
//                    ffi::glx_extra::CONTEXT_PROFILE_MASK_ARB as raw::c_int,
//                );
//                attributes.push(flag as raw::c_int);
//            }
//
//            let flags = {
//                let mut flags = 0;
//
//                // robustness
//                if check_ext(extensions, "GLX_ARB_create_context_robustness") {
//                    match robustness {
//                        Robustness::RobustNoResetNotification
//                        | Robustness::TryRobustNoResetNotification => {
//                            attributes.push(
//                                ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as raw::c_int,
//                            );
//                            attributes.push(
//                                ffi::glx_extra::NO_RESET_NOTIFICATION_ARB
//                                    as raw::c_int,
//                            );
//                            flags = flags
//                                | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB
//                                    as raw::c_int;
//                        }
//                        Robustness::RobustLoseContextOnReset
//                        | Robustness::TryRobustLoseContextOnReset => {
//                            attributes.push(
//                                ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as raw::c_int,
//                            );
//                            attributes.push(
//                                ffi::glx_extra::LOSE_CONTEXT_ON_RESET_ARB
//                                    as raw::c_int,
//                            );
//                            flags = flags
//                                | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB
//                                    as raw::c_int;
//                        }
//                        Robustness::NotRobust => (),
//                        Robustness::NoError => (),
//                    }
//                } else {
//                    match robustness {
//                        Robustness::RobustNoResetNotification
//                        | Robustness::RobustLoseContextOnReset => {
//                            return Err(make_error!(ErrorType::RobustnessNotSupported));
//                        }
//                        _ => (),
//                    }
//                }
//
//                if debug {
//                    flags = flags
//                        | ffi::glx_extra::CONTEXT_DEBUG_BIT_ARB as raw::c_int;
//                }
//
//                flags
//            };
//
//            attributes.push(ffi::glx_extra::CONTEXT_FLAGS_ARB as raw::c_int);
//            attributes.push(flags);
//
//            attributes.push(0);
//
//            extra_functions.CreateContextAttribsARB(
//                display as *mut _,
//                fb_config,
//                share,
//                1,
//                attributes.as_ptr(),
//            )
//        } else {
//            let visual_infos: *const ffi::XVisualInfo = visual_infos;
//            glx.CreateContext(
//                display as *mut _,
//                visual_infos as *mut _,
//                share,
//                1,
//            )
//        };
//
//        (xlib.XSetErrorHandler)(old_callback);
//
//        if context.is_null() {
//            // TODO: check for errors and return `OpenGlVersionNotSupported`
//            return Err(make_oserror!(OsError::Misc(
//                "GL context creation failed".to_string(),
//            )));
//        }
//
//        Ok(context)
//    }
//}
//
