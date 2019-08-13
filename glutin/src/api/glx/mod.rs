#![cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]

mod make_current_guard;
mod glx {
    use crate::api::dlloader::{SymTrait, SymWrapper};
    use glutin_glx_sys as ffi;
    use std::ops::{Deref, DerefMut};

    #[derive(Clone)]
    pub struct Glx(SymWrapper<ffi::glx::Glx>);

    /// Because `*const raw::c_void` doesn't implement `Sync`.
    unsafe impl Sync for Glx {}

    impl SymTrait for ffi::glx::Glx {
        fn load_with<F>(_: &libloading::Library, loadfn: F) -> Self
        where
            F: FnMut(&'static str) -> *const std::os::raw::c_void,
        {
            Self::load_with(loadfn)
        }
    }

    impl Glx {
        pub fn new() -> Result<Self, ()> {
            let paths = vec!["libGL.so.1", "libGL.so"];

            SymWrapper::new(paths).map(|i| Glx(i))
        }
    }

    impl Deref for Glx {
        type Target = ffi::glx::Glx;

        fn deref(&self) -> &ffi::glx::Glx {
            &self.0
        }
    }

    impl DerefMut for Glx {
        fn deref_mut(&mut self) -> &mut ffi::glx::Glx {
            &mut self.0
        }
    }
}

pub use self::glx::Glx;
use self::make_current_guard::MakeCurrentGuard;
use crate::{
    Api, ContextError, CreationError, GlAttributes, GlProfile, GlRequest,
    PixelFormat, PixelFormatRequirements, ReleaseBehavior, Robustness,
};

use crate::platform::unix::x11::XConnection;
use crate::platform_impl::x11_utils::SurfaceType;
use glutin_glx_sys as ffi;
use winit::dpi;

use std::ffi::{CStr, CString};
use std::os::raw;
use std::sync::Arc;

lazy_static! {
    pub static ref GLX: Option<Glx> = Glx::new().ok();
}

#[derive(Debug)]
pub struct Context {
    xconn: Arc<XConnection>,
    drawable: ffi::Window,
    context: ffi::GLXContext,
    pixel_format: PixelFormat,
}

impl Context {
    // transparent is `None` if window is raw.
    pub fn new<'a>(
        xconn: Arc<XConnection>,
        pf_reqs: &PixelFormatRequirements,
        opengl: &'a GlAttributes<&'a Context>,
        screen_id: raw::c_int,
        surface_type: SurfaceType,
        transparent: Option<bool>,
    ) -> Result<ContextPrototype<'a>, CreationError> {
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
            glx.QueryVersion(xconn.display as *mut _, &mut major, &mut minor);
        }

        // loading the list of extensions
        let extensions = load_extensions(&xconn, screen_id)?;

        // finding the pixel format we want
        let (fb_config, pixel_format, visual_infos) = unsafe {
            choose_fbconfig(
                &extensions,
                &xconn,
                screen_id,
                pf_reqs,
                surface_type,
                transparent,
            )?
        };

        Ok(ContextPrototype {
            extensions,
            xconn,
            opengl,
            fb_config,
            visual_infos: unsafe { std::mem::transmute(visual_infos) },
            pixel_format,
        })
    }

    unsafe fn check_make_current(
        &self,
        ret: Option<i32>,
    ) -> Result<(), ContextError> {
        if ret == Some(0) {
            let err = self.xconn.check_errors();
            Err(ContextError::OsError(format!(
                "`glXMakeCurrent` failed: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        let glx = GLX.as_ref().unwrap();
        let res = glx.MakeCurrent(
            self.xconn.display as *mut _,
            self.drawable,
            self.context,
        );
        self.check_make_current(Some(res))
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        let glx = GLX.as_ref().unwrap();
        if self.drawable == glx.GetCurrentDrawable()
            || self.context == glx.GetCurrentContext()
        {
            let res = glx.MakeCurrent(
                self.xconn.display as *mut _,
                0,
                std::ptr::null(),
            );
            self.check_make_current(Some(res))
        } else {
            self.check_make_current(None)
        }
    }

    #[inline]
    pub fn is_current(&self) -> bool {
        let glx = GLX.as_ref().unwrap();
        unsafe { glx.GetCurrentContext() == self.context }
    }

    #[inline]
    pub fn get_api(&self) -> crate::Api {
        crate::Api::OpenGl
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::GLXContext {
        self.context
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const () {
        let glx = GLX.as_ref().unwrap();
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        unsafe { glx.GetProcAddress(addr as *const _) as *const _ }
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            glx.SwapBuffers(self.xconn.display as *mut _, self.drawable);
        }
        if let Err(err) = self.xconn.check_errors() {
            Err(ContextError::OsError(format!(
                "`glXSwapBuffers` failed: {:?}",
                err
            )))
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.pixel_format.clone()
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Drop for Context {
    fn drop(&mut self) {
        let glx = GLX.as_ref().unwrap();
        unsafe {
            // See `drop` for `crate::api::egl::Context` for rationale.
            let mut guard =
                MakeCurrentGuard::new(&self.xconn, self.drawable, self.context)
                    .map_err(|err| ContextError::OsError(err))
                    .unwrap();

            let gl_finish_fn = self.get_proc_address("glFinish");
            assert!(gl_finish_fn != std::ptr::null());
            let gl_finish_fn =
                std::mem::transmute::<_, extern "system" fn()>(gl_finish_fn);
            gl_finish_fn();

            if guard.old_context() == Some(self.context) {
                guard.invalidate()
            }
            std::mem::drop(guard);

            glx.DestroyContext(self.xconn.display as *mut _, self.context);
        }
    }
}

#[derive(Debug)]
pub struct ContextPrototype<'a> {
    extensions: String,
    xconn: Arc<XConnection>,
    opengl: &'a GlAttributes<&'a Context>,
    fb_config: ffi::glx::types::GLXFBConfig,
    visual_infos: ffi::XVisualInfo,
    pixel_format: PixelFormat,
}

impl<'a> ContextPrototype<'a> {
    #[inline]
    pub fn get_visual_infos(&self) -> &ffi::XVisualInfo {
        &self.visual_infos
    }

    // creating GL context
    fn create_context(
        &self,
    ) -> Result<(ffi::glx_extra::Glx, ffi::GLXContext), CreationError> {
        let glx = GLX.as_ref().unwrap();
        let share = match self.opengl.sharing {
            Some(ctx) => ctx.context,
            None => std::ptr::null(),
        };

        // loading the extra GLX functions
        let extra_functions = ffi::glx_extra::Glx::load_with(|proc_name| {
            let c_str = CString::new(proc_name).unwrap();
            unsafe {
                glx.GetProcAddress(c_str.as_ptr() as *const u8) as *const _
            }
        });

        let context = match self.opengl.version {
            GlRequest::Latest => {
                let opengl_versions = [
                    (4, 6),
                    (4, 5),
                    (4, 4),
                    (4, 3),
                    (4, 2),
                    (4, 1),
                    (4, 0),
                    (3, 3),
                    (3, 2),
                    (3, 1),
                ];
                let ctx;
                'outer: loop {
                    // Try all OpenGL versions in descending order because some
                    // non-compliant drivers don't return
                    // the latest supported version but the one requested
                    for opengl_version in opengl_versions.iter() {
                        match create_context(
                            &extra_functions,
                            &self.extensions,
                            &self.xconn.xlib,
                            *opengl_version,
                            self.opengl.profile,
                            self.opengl.debug,
                            self.opengl.robustness,
                            share,
                            self.xconn.display,
                            self.fb_config,
                            &self.visual_infos,
                        ) {
                            Ok(x) => {
                                ctx = x;
                                break 'outer;
                            }
                            Err(_) => continue,
                        }
                    }
                    ctx = create_context(
                        &extra_functions,
                        &self.extensions,
                        &self.xconn.xlib,
                        (1, 0),
                        self.opengl.profile,
                        self.opengl.debug,
                        self.opengl.robustness,
                        share,
                        self.xconn.display,
                        self.fb_config,
                        &self.visual_infos,
                    )?;
                    break;
                }
                ctx
            }
            GlRequest::Specific(Api::OpenGl, (major, minor)) => create_context(
                &extra_functions,
                &self.extensions,
                &self.xconn.xlib,
                (major, minor),
                self.opengl.profile,
                self.opengl.debug,
                self.opengl.robustness,
                share,
                self.xconn.display,
                self.fb_config,
                &self.visual_infos,
            )?,
            GlRequest::Specific(_, _) => panic!("Only OpenGL is supported"),
            GlRequest::GlThenGles {
                opengl_version: (major, minor),
                ..
            } => create_context(
                &extra_functions,
                &self.extensions,
                &self.xconn.xlib,
                (major, minor),
                self.opengl.profile,
                self.opengl.debug,
                self.opengl.robustness,
                share,
                self.xconn.display,
                self.fb_config,
                &self.visual_infos,
            )?,
        };

        Ok((extra_functions, context))
    }

    pub fn finish_pbuffer(
        self,
        size: dpi::PhysicalSize,
    ) -> Result<Context, CreationError> {
        let glx = GLX.as_ref().unwrap();
        let size: (u32, u32) = size.into();
        let (_extra_functions, context) = self.create_context()?;

        let attributes: Vec<raw::c_int> = vec![
            ffi::glx::PBUFFER_WIDTH as raw::c_int,
            size.0 as raw::c_int,
            ffi::glx::PBUFFER_HEIGHT as raw::c_int,
            size.1 as raw::c_int,
            0,
        ];

        let pbuffer = unsafe {
            glx.CreatePbuffer(
                self.xconn.display as *mut _,
                self.fb_config,
                attributes.as_ptr(),
            )
        };

        Ok(Context {
            xconn: self.xconn,
            drawable: pbuffer,
            context,
            pixel_format: self.pixel_format,
        })
    }

    pub fn finish(self, window: ffi::Window) -> Result<Context, CreationError> {
        let glx = GLX.as_ref().unwrap();
        let (extra_functions, context) = self.create_context()?;

        // vsync
        if self.opengl.vsync {
            let _guard = MakeCurrentGuard::new(&self.xconn, window, context)
                .map_err(|err| CreationError::OsError(err))?;

            if check_ext(&self.extensions, "GLX_EXT_swap_control")
                && extra_functions.SwapIntervalEXT.is_loaded()
            {
                // this should be the most common extension
                unsafe {
                    extra_functions.SwapIntervalEXT(
                        self.xconn.display as *mut _,
                        window,
                        1,
                    );
                }

                let mut swap = unsafe { std::mem::uninitialized() };
                unsafe {
                    glx.QueryDrawable(
                        self.xconn.display as *mut _,
                        window,
                        ffi::glx_extra::SWAP_INTERVAL_EXT as i32,
                        &mut swap,
                    );
                }

                if swap != 1 {
                    return Err(CreationError::OsError(format!("Couldn't setup vsync: expected interval `1` but got `{}`", swap)));
                }
            } else if check_ext(&self.extensions, "GLX_MESA_swap_control")
                && extra_functions.SwapIntervalMESA.is_loaded()
            {
                unsafe {
                    extra_functions.SwapIntervalMESA(1);
                }
            } else if check_ext(&self.extensions, "GLX_SGI_swap_control")
                && extra_functions.SwapIntervalSGI.is_loaded()
            {
                unsafe {
                    extra_functions.SwapIntervalSGI(1);
                }
            } else {
                return Err(CreationError::OsError(
                    "Couldn't find any available vsync extension".to_string(),
                ));
            }
        }

        Ok(Context {
            xconn: self.xconn,
            drawable: window,
            context,
            pixel_format: self.pixel_format,
        })
    }
}

extern "C" fn x_error_callback(
    _dpy: *mut ffi::Display,
    _err: *mut ffi::XErrorEvent,
) -> i32 {
    0
}

fn create_context(
    extra_functions: &ffi::glx_extra::Glx,
    extensions: &str,
    xlib: &ffi::Xlib,
    version: (u8, u8),
    profile: Option<GlProfile>,
    debug: bool,
    robustness: Robustness,
    share: ffi::GLXContext,
    display: *mut ffi::Display,
    fb_config: ffi::glx::types::GLXFBConfig,
    visual_infos: &ffi::XVisualInfo,
) -> Result<ffi::GLXContext, CreationError> {
    let glx = GLX.as_ref().unwrap();
    unsafe {
        let old_callback = (xlib.XSetErrorHandler)(Some(x_error_callback));
        let context = if check_ext(extensions, "GLX_ARB_create_context") {
            let mut attributes = Vec::with_capacity(9);

            attributes
                .push(ffi::glx_extra::CONTEXT_MAJOR_VERSION_ARB as raw::c_int);
            attributes.push(version.0 as raw::c_int);
            attributes
                .push(ffi::glx_extra::CONTEXT_MINOR_VERSION_ARB as raw::c_int);
            attributes.push(version.1 as raw::c_int);

            if let Some(profile) = profile {
                let flag = match profile {
                    GlProfile::Compatibility => {
                        ffi::glx_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB
                    }
                    GlProfile::Core => {
                        ffi::glx_extra::CONTEXT_CORE_PROFILE_BIT_ARB
                    }
                };

                attributes.push(
                    ffi::glx_extra::CONTEXT_PROFILE_MASK_ARB as raw::c_int,
                );
                attributes.push(flag as raw::c_int);
            }

            let flags = {
                let mut flags = 0;

                // robustness
                if check_ext(extensions, "GLX_ARB_create_context_robustness") {
                    match robustness {
                        Robustness::RobustNoResetNotification
                        | Robustness::TryRobustNoResetNotification => {
                            attributes.push(
                                ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as raw::c_int,
                            );
                            attributes.push(
                                ffi::glx_extra::NO_RESET_NOTIFICATION_ARB
                                    as raw::c_int,
                            );
                            flags = flags
                                | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB
                                    as raw::c_int;
                        }
                        Robustness::RobustLoseContextOnReset
                        | Robustness::TryRobustLoseContextOnReset => {
                            attributes.push(
                                ffi::glx_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as raw::c_int,
                            );
                            attributes.push(
                                ffi::glx_extra::LOSE_CONTEXT_ON_RESET_ARB
                                    as raw::c_int,
                            );
                            flags = flags
                                | ffi::glx_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB
                                    as raw::c_int;
                        }
                        Robustness::NotRobust => (),
                        Robustness::NoError => (),
                    }
                } else {
                    match robustness {
                        Robustness::RobustNoResetNotification
                        | Robustness::RobustLoseContextOnReset => {
                            return Err(CreationError::RobustnessNotSupported);
                        }
                        _ => (),
                    }
                }

                if debug {
                    flags = flags
                        | ffi::glx_extra::CONTEXT_DEBUG_BIT_ARB as raw::c_int;
                }

                flags
            };

            attributes.push(ffi::glx_extra::CONTEXT_FLAGS_ARB as raw::c_int);
            attributes.push(flags);

            attributes.push(0);

            extra_functions.CreateContextAttribsARB(
                display as *mut _,
                fb_config,
                share,
                1,
                attributes.as_ptr(),
            )
        } else {
            let visual_infos: *const ffi::XVisualInfo = visual_infos;
            glx.CreateContext(
                display as *mut _,
                visual_infos as *mut _,
                share,
                1,
            )
        };

        (xlib.XSetErrorHandler)(old_callback);

        if context.is_null() {
            // TODO: check for errors and return `OpenGlVersionNotSupported`
            return Err(CreationError::OsError(
                "GL context creation failed".to_string(),
            ));
        }

        Ok(context)
    }
}

/// Enumerates all available FBConfigs
unsafe fn choose_fbconfig(
    extensions: &str,
    xconn: &Arc<XConnection>,
    screen_id: raw::c_int,
    pf_reqs: &PixelFormatRequirements,
    surface_type: SurfaceType,
    transparent: Option<bool>,
) -> Result<
    (ffi::glx::types::GLXFBConfig, PixelFormat, ffi::XVisualInfo),
    CreationError,
> {
    let glx = GLX.as_ref().unwrap();

    let descriptor = {
        let mut out: Vec<raw::c_int> = Vec::with_capacity(37);

        out.push(ffi::glx::X_RENDERABLE as raw::c_int);
        out.push(1);

        if let Some(xid) = pf_reqs.x11_visual_xid {
            // getting the visual infos
            let fvi = crate::platform_impl::x11_utils::get_visual_info_from_xid(
                &xconn, xid,
            );

            out.push(ffi::glx::X_VISUAL_TYPE as raw::c_int);
            out.push(fvi.class as raw::c_int);

            out.push(ffi::glx::VISUAL_ID as raw::c_int);
            out.push(xid as raw::c_int);
        } else {
            out.push(ffi::glx::X_VISUAL_TYPE as raw::c_int);
            out.push(ffi::glx::TRUE_COLOR as raw::c_int);
        }

        out.push(ffi::glx::DRAWABLE_TYPE as raw::c_int);
        let surface_type = match surface_type {
            SurfaceType::Window => ffi::glx::WINDOW_BIT,
            SurfaceType::PBuffer => ffi::glx::PBUFFER_BIT,
            SurfaceType::Surfaceless => ffi::glx::DONT_CARE, /* TODO: Properly support */
        };
        out.push(surface_type as raw::c_int);

        // TODO: Use RGB/RGB_FLOAT_BIT_ARB if they don't want alpha bits,
        // fallback to it if they don't care
        out.push(ffi::glx::RENDER_TYPE as raw::c_int);
        if pf_reqs.float_color_buffer {
            if check_ext(extensions, "GLX_ARB_fbconfig_float") {
                out.push(ffi::glx_extra::RGBA_FLOAT_BIT_ARB as raw::c_int);
            } else {
                return Err(CreationError::NoAvailablePixelFormat);
            }
        } else {
            out.push(ffi::glx::RGBA_BIT as raw::c_int);
        }

        if let Some(color) = pf_reqs.color_bits {
            out.push(ffi::glx::RED_SIZE as raw::c_int);
            out.push((color / 3) as raw::c_int);
            out.push(ffi::glx::GREEN_SIZE as raw::c_int);
            out.push(
                (color / 3 + if color % 3 != 0 { 1 } else { 0 }) as raw::c_int,
            );
            out.push(ffi::glx::BLUE_SIZE as raw::c_int);
            out.push(
                (color / 3 + if color % 3 == 2 { 1 } else { 0 }) as raw::c_int,
            );
        }

        if let Some(alpha) = pf_reqs.alpha_bits {
            out.push(ffi::glx::ALPHA_SIZE as raw::c_int);
            out.push(alpha as raw::c_int);
        }

        if let Some(depth) = pf_reqs.depth_bits {
            out.push(ffi::glx::DEPTH_SIZE as raw::c_int);
            out.push(depth as raw::c_int);
        }

        if let Some(stencil) = pf_reqs.stencil_bits {
            out.push(ffi::glx::STENCIL_SIZE as raw::c_int);
            out.push(stencil as raw::c_int);
        }

        let double_buffer = pf_reqs.double_buffer.unwrap_or(true);
        out.push(ffi::glx::DOUBLEBUFFER as raw::c_int);
        out.push(if double_buffer { 1 } else { 0 });

        if let Some(multisampling) = pf_reqs.multisampling {
            if check_ext(extensions, "GLX_ARB_multisample") {
                out.push(ffi::glx_extra::SAMPLE_BUFFERS_ARB as raw::c_int);
                out.push(if multisampling == 0 { 0 } else { 1 });
                out.push(ffi::glx_extra::SAMPLES_ARB as raw::c_int);
                out.push(multisampling as raw::c_int);
            } else {
                return Err(CreationError::NoAvailablePixelFormat);
            }
        }

        out.push(ffi::glx::STEREO as raw::c_int);
        out.push(if pf_reqs.stereoscopy { 1 } else { 0 });

        if pf_reqs.srgb {
            if check_ext(extensions, "GLX_ARB_framebuffer_sRGB") {
                out.push(
                    ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as raw::c_int,
                );
                out.push(1);
            } else if check_ext(extensions, "GLX_EXT_framebuffer_sRGB") {
                out.push(
                    ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as raw::c_int,
                );
                out.push(1);
            } else {
                return Err(CreationError::NoAvailablePixelFormat);
            }
        }

        match pf_reqs.release_behavior {
            ReleaseBehavior::Flush => (),
            ReleaseBehavior::None => {
                if check_ext(extensions, "GLX_ARB_context_flush_control") {
                    out.push(
                        ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_ARB
                            as raw::c_int,
                    );
                    out.push(
                        ffi::glx_extra::CONTEXT_RELEASE_BEHAVIOR_NONE_ARB
                            as raw::c_int,
                    );
                }
            }
        }

        out.push(ffi::glx::CONFIG_CAVEAT as raw::c_int);
        out.push(ffi::glx::DONT_CARE as raw::c_int);

        out.push(0);
        out
    };

    // calling glXChooseFBConfig
    let (fb_config, visual_infos): (
        ffi::glx::types::GLXFBConfig,
        ffi::XVisualInfo,
    ) = {
        let mut num_configs = 0;
        let configs = glx.ChooseFBConfig(
            xconn.display as *mut _,
            screen_id,
            descriptor.as_ptr(),
            &mut num_configs,
        );
        if configs.is_null() {
            return Err(CreationError::NoAvailablePixelFormat);
        }
        if num_configs == 0 {
            return Err(CreationError::NoAvailablePixelFormat);
        }

        match crate::platform_impl::x11_utils::select_config(
            xconn,
            transparent,
            pf_reqs,
            (0..num_configs).collect(),
            |config_id| {
                let visual_infos_raw = glx.GetVisualFromFBConfig(
                    xconn.display as *mut _,
                    *configs.offset(*config_id as isize),
                );

                if visual_infos_raw.is_null() {
                    return None;
                }

                let visual_infos: ffi::XVisualInfo =
                    std::ptr::read(visual_infos_raw as *const _);
                (xconn.xlib.XFree)(visual_infos_raw as *mut _);
                Some(visual_infos)
            },
        ) {
            Ok((config_id, visual_infos)) => {
                let config = *configs.offset(config_id as isize);
                let config = config.clone();

                (xconn.xlib.XFree)(configs as *mut _);
                (config, visual_infos)
            }
            Err(()) => {
                (xconn.xlib.XFree)(configs as *mut _);
                return Err(CreationError::NoAvailablePixelFormat);
            }
        }
    };

    let get_attrib = |attrib: raw::c_int| -> i32 {
        let mut value = 0;
        glx.GetFBConfigAttrib(
            xconn.display as *mut _,
            fb_config,
            attrib,
            &mut value,
        );
        // TODO: check return value
        value
    };

    let pf_desc = PixelFormat {
        hardware_accelerated: get_attrib(ffi::glx::CONFIG_CAVEAT as raw::c_int)
            != ffi::glx::SLOW_CONFIG as raw::c_int,
        color_bits: get_attrib(ffi::glx::RED_SIZE as raw::c_int) as u8
            + get_attrib(ffi::glx::GREEN_SIZE as raw::c_int) as u8
            + get_attrib(ffi::glx::BLUE_SIZE as raw::c_int) as u8,
        alpha_bits: get_attrib(ffi::glx::ALPHA_SIZE as raw::c_int) as u8,
        depth_bits: get_attrib(ffi::glx::DEPTH_SIZE as raw::c_int) as u8,
        stencil_bits: get_attrib(ffi::glx::STENCIL_SIZE as raw::c_int) as u8,
        stereoscopy: get_attrib(ffi::glx::STEREO as raw::c_int) != 0,
        double_buffer: get_attrib(ffi::glx::DOUBLEBUFFER as raw::c_int) != 0,
        multisampling: if get_attrib(ffi::glx::SAMPLE_BUFFERS as raw::c_int)
            != 0
        {
            Some(get_attrib(ffi::glx::SAMPLES as raw::c_int) as u16)
        } else {
            None
        },
        srgb: get_attrib(
            ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB as raw::c_int,
        ) != 0
            || get_attrib(
                ffi::glx_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT as raw::c_int,
            ) != 0,
    };

    Ok((fb_config, pf_desc, visual_infos))
}

/// Checks if `ext` is available.
fn check_ext(extensions: &str, ext: &str) -> bool {
    extensions.split(' ').find(|&s| s == ext).is_some()
}

fn load_extensions(
    xconn: &Arc<XConnection>,
    screen_id: raw::c_int,
) -> Result<String, CreationError> {
    unsafe {
        let glx = GLX.as_ref().unwrap();
        let extensions =
            glx.QueryExtensionsString(xconn.display as *mut _, screen_id);
        if extensions.is_null() {
            return Err(CreationError::OsError(
                "`glXQueryExtensionsString` found no glX extensions"
                    .to_string(),
            ));
        }
        let extensions = CStr::from_ptr(extensions).to_bytes().to_vec();
        Ok(String::from_utf8(extensions).unwrap())
    }
}
