#![cfg(any(target_os = "windows"))]

use ContextError;
use CreationError;
use GlAttributes;
use GlContext;
use GlRequest;
use GlProfile;
use PixelFormat;
use PixelFormatRequirements;
use Robustness;
use Api;

use self::make_current_guard::CurrentContextGuard;

use std::ffi::{CStr, CString, OsStr};
use std::os::raw::{c_void, c_int};
use std::os::windows::ffi::OsStrExt;
use std::{mem, ptr};
use std::io;

use winapi;
use kernel32;
use user32;
use gdi32;

mod make_current_guard;
mod gl;

/// A WGL context.
///
/// Note: should be destroyed before its window.
pub struct Context {
    context: ContextWrapper,

    hdc: winapi::HDC,

    /// Binded to `opengl32.dll`.
    ///
    /// `wglGetProcAddress` returns null for GL 1.1 functions because they are
    ///  already defined by the system. This module contains them.
    gl_library: winapi::HMODULE,

    /// The pixel format that has been used to create this context.
    pixel_format: PixelFormat,
}

/// A simple wrapper that destroys the window when it is destroyed.
struct WindowWrapper(winapi::HWND, winapi::HDC);

impl Drop for WindowWrapper {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            user32::DestroyWindow(self.0);
        }
    }
}

/// Wraps around a context so that it is destroyed when necessary.
struct ContextWrapper(winapi::HGLRC);

impl Drop for ContextWrapper {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            gl::wgl::DeleteContext(self.0 as *const _);
        }
    }
}

impl Context {
    /// Attempt to build a new WGL context on a window.
    ///
    /// The window must **not** have had `SetPixelFormat` called on it.
    ///
    /// # Unsafety
    ///
    /// The `window` must continue to exist as long as the resulting `Context` exists.
    pub unsafe fn new(pf_reqs: &PixelFormatRequirements, opengl: &GlAttributes<winapi::HGLRC>,
                      window: winapi::HWND) -> Result<Context, CreationError>
    {
        let hdc = user32::GetDC(window);
        if hdc.is_null() {
            let err = Err(CreationError::OsError(format!("GetDC function failed: {}",
                                                format!("{}", io::Error::last_os_error()))));
            return err;
        }

        // loading the functions that are not guaranteed to be supported
        let extra_functions = try!(load_extra_functions(window));

        // getting the list of the supported extensions
        let extensions = if extra_functions.GetExtensionsStringARB.is_loaded() {
            let data = extra_functions.GetExtensionsStringARB(hdc as *const _);
            let data = CStr::from_ptr(data).to_bytes().to_vec();
            String::from_utf8(data).unwrap()

        } else if extra_functions.GetExtensionsStringEXT.is_loaded() {
            let data = extra_functions.GetExtensionsStringEXT();
            let data = CStr::from_ptr(data).to_bytes().to_vec();
            String::from_utf8(data).unwrap()

        } else {
            format!("")
        };

        // calling SetPixelFormat
        let pixel_format = {
            let formats = if extensions.split(' ').find(|&i| i == "WGL_ARB_pixel_format")
                                                  .is_some()
            {
                let f = enumerate_arb_pixel_formats(&extra_functions, &extensions, hdc);
                if f.is_empty() {
                    enumerate_native_pixel_formats(hdc)
                } else {
                    f
                }
            } else {
                enumerate_native_pixel_formats(hdc)
            };

            let (id, f) = try!(pf_reqs.choose_pixel_format(formats));
            try!(set_pixel_format(hdc, id));
            f
        };

        // creating the OpenGL context
        let context = try!(create_context(Some((&extra_functions, pf_reqs, opengl, &extensions)),
                                          window, hdc));

        // loading the opengl32 module
        let gl_library = try!(load_opengl32_dll());

        // handling vsync
        if extensions.split(' ').find(|&i| i == "WGL_EXT_swap_control").is_some() {
            let _guard = try!(CurrentContextGuard::make_current(hdc, context.0));

            if extra_functions.SwapIntervalEXT(if opengl.vsync { 1 } else { 0 }) == 0 {
                return Err(CreationError::OsError(format!("wglSwapIntervalEXT failed")));
            }
        }

        Ok(Context {
            context: context,
            hdc: hdc,
            gl_library: gl_library,
            pixel_format: pixel_format,
        })
    }

    /// Returns the raw HGLRC.
    #[inline]
    pub fn get_hglrc(&self) -> winapi::HGLRC {
        self.context.0
    }
}

impl GlContext for Context {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        if gl::wgl::MakeCurrent(self.hdc as *const _, self.context.0 as *const _) != 0 {
            Ok(())
        } else {
            Err(ContextError::IoError(io::Error::last_os_error()))
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        unsafe { gl::wgl::GetCurrentContext() == self.context.0 as *const c_void }
    }

    fn get_proc_address(&self, addr: &str) -> *const () {
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();

        unsafe {
            let p = gl::wgl::GetProcAddress(addr) as *const _;
            if !p.is_null() { return p; }
            kernel32::GetProcAddress(self.gl_library, addr) as *const _
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        // TODO: decide how to handle the error
        /*if unsafe { gdi32::SwapBuffers(self.hdc) } != 0 {
            Ok(())
        } else {
            Err(ContextError::IoError(io::Error::last_os_error()))
        }*/
        unsafe { gdi32::SwapBuffers(self.hdc) };
        Ok(())
    }

    #[inline]
    fn get_api(&self) -> Api {
        // FIXME: can be opengl es
        Api::OpenGl
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        self.pixel_format.clone()
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

/// Creates an OpenGL context.
///
/// If `extra` is `Some`, this function will attempt to use the latest WGL functions to create the
/// context.
///
/// Otherwise, only the basic API will be used and the chances of `CreationError::NotSupported`
/// being returned increase.
unsafe fn create_context(extra: Option<(&gl::wgl_extra::Wgl, &PixelFormatRequirements,
                                        &GlAttributes<winapi::HGLRC>, &str)>,
                         _: winapi::HWND, hdc: winapi::HDC)
                         -> Result<ContextWrapper, CreationError>
{
    let share;

    if let Some((extra_functions, pf_reqs, opengl, extensions)) = extra {
        share = opengl.sharing.unwrap_or(ptr::null_mut());

        if extensions.split(' ').find(|&i| i == "WGL_ARB_create_context").is_some() {
            let mut attributes = Vec::new();

            match opengl.version {
                GlRequest::Latest => {},
                GlRequest::Specific(Api::OpenGl, (major, minor)) => {
                    attributes.push(gl::wgl_extra::CONTEXT_MAJOR_VERSION_ARB as c_int);
                    attributes.push(major as c_int);
                    attributes.push(gl::wgl_extra::CONTEXT_MINOR_VERSION_ARB as c_int);
                    attributes.push(minor as c_int);
                },
                GlRequest::Specific(Api::OpenGlEs, (major, minor)) => {
                    if extensions.split(' ').find(|&i| i == "WGL_EXT_create_context_es2_profile")
                                            .is_some()
                    {
                        attributes.push(gl::wgl_extra::CONTEXT_PROFILE_MASK_ARB as c_int);
                        attributes.push(gl::wgl_extra::CONTEXT_ES2_PROFILE_BIT_EXT as c_int);
                    } else {
                        return Err(CreationError::OpenGlVersionNotSupported);
                    }

                    attributes.push(gl::wgl_extra::CONTEXT_MAJOR_VERSION_ARB as c_int);
                    attributes.push(major as c_int);
                    attributes.push(gl::wgl_extra::CONTEXT_MINOR_VERSION_ARB as c_int);
                    attributes.push(minor as c_int);
                },
                GlRequest::Specific(_, _) => return Err(CreationError::OpenGlVersionNotSupported),
                GlRequest::GlThenGles { opengl_version: (major, minor), .. } => {
                    attributes.push(gl::wgl_extra::CONTEXT_MAJOR_VERSION_ARB as c_int);
                    attributes.push(major as c_int);
                    attributes.push(gl::wgl_extra::CONTEXT_MINOR_VERSION_ARB as c_int);
                    attributes.push(minor as c_int);
                },
            }

            if let Some(profile) = opengl.profile {
                if extensions.split(' ').find(|&i| i == "WGL_ARB_create_context_profile").is_some()
                {
                    let flag = match profile {
                        GlProfile::Compatibility =>
                            gl::wgl_extra::CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB,
                        GlProfile::Core =>
                            gl::wgl_extra::CONTEXT_CORE_PROFILE_BIT_ARB,
                    };
                    attributes.push(gl::wgl_extra::CONTEXT_PROFILE_MASK_ARB as c_int);
                    attributes.push(flag as c_int);
                } else {
                    return Err(CreationError::NotSupported);
                }
            }

            let flags = {
                let mut flags = 0;

                // robustness
                if extensions.split(' ').find(|&i| i == "WGL_ARB_create_context_robustness").is_some() {
                    match opengl.robustness {
                        Robustness::RobustNoResetNotification | Robustness::TryRobustNoResetNotification => {
                            attributes.push(gl::wgl_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                            attributes.push(gl::wgl_extra::NO_RESET_NOTIFICATION_ARB as c_int);
                            flags = flags | gl::wgl_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                        },
                        Robustness::RobustLoseContextOnReset | Robustness::TryRobustLoseContextOnReset => {
                            attributes.push(gl::wgl_extra::CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as c_int);
                            attributes.push(gl::wgl_extra::LOSE_CONTEXT_ON_RESET_ARB as c_int);
                            flags = flags | gl::wgl_extra::CONTEXT_ROBUST_ACCESS_BIT_ARB as c_int;
                        },
                        Robustness::NotRobust => (),
                        Robustness::NoError => (),
                    }
                } else {
                    match opengl.robustness {
                        Robustness::RobustNoResetNotification | Robustness::RobustLoseContextOnReset => {
                            return Err(CreationError::RobustnessNotSupported);
                        },
                        _ => ()
                    }
                }

                if opengl.debug {
                    flags = flags | gl::wgl_extra::CONTEXT_DEBUG_BIT_ARB as c_int;
                }

                flags
            };

            attributes.push(gl::wgl_extra::CONTEXT_FLAGS_ARB as c_int);
            attributes.push(flags);

            attributes.push(0);

            let ctxt = extra_functions.CreateContextAttribsARB(hdc as *const c_void,
                                                               share as *const c_void,
                                                               attributes.as_ptr());

            if ctxt.is_null() {
                return Err(CreationError::OsError(format!("wglCreateContextAttribsARB failed: {}",
                                                      format!("{}", io::Error::last_os_error()))));
            } else {
                return Ok(ContextWrapper(ctxt as winapi::HGLRC));
            }
        }

    } else {
        share = ptr::null_mut();
    }

    let ctxt = gl::wgl::CreateContext(hdc as *const c_void);
    if ctxt.is_null() {
        return Err(CreationError::OsError(format!("wglCreateContext failed: {}",
                                                  format!("{}", io::Error::last_os_error()))));
    }

    if !share.is_null() {
        if gl::wgl::ShareLists(share as *const c_void, ctxt) == 0 {
            return Err(CreationError::OsError(format!("wglShareLists failed: {}",
                                                      format!("{}", io::Error::last_os_error()))));
        }
    };

    Ok(ContextWrapper(ctxt as winapi::HGLRC))
}

/// Enumerates the list of pixel formats without using WGL.
///
/// Gives less precise results than `enumerate_arb_pixel_formats`.
unsafe fn enumerate_native_pixel_formats(hdc: winapi::HDC) -> Vec<(c_int, PixelFormat)> {
    let size_of_pxfmtdescr = mem::size_of::<winapi::PIXELFORMATDESCRIPTOR>() as u32;
    let num = gdi32::DescribePixelFormat(hdc, 1, size_of_pxfmtdescr, ptr::null_mut());

    let mut result = Vec::new();

    for index in (0 .. num) {
        let mut output: winapi::PIXELFORMATDESCRIPTOR = mem::zeroed();
        
        if gdi32::DescribePixelFormat(hdc, index, size_of_pxfmtdescr, &mut output) == 0 {
            continue;
        }

        if (output.dwFlags & winapi::PFD_DRAW_TO_WINDOW) == 0 {
            continue;
        }

        if (output.dwFlags & winapi::PFD_SUPPORT_OPENGL) == 0 {
            continue;
        }

        if output.iPixelType != winapi::PFD_TYPE_RGBA {
            continue;
        }

        result.push((index, PixelFormat {
            hardware_accelerated: (output.dwFlags & winapi::PFD_GENERIC_FORMAT) == 0,
            color_bits: output.cRedBits + output.cGreenBits + output.cBlueBits,
            alpha_bits: output.cAlphaBits,
            depth_bits: output.cDepthBits,
            stencil_bits: output.cStencilBits,
            stereoscopy: (output.dwFlags & winapi::PFD_STEREO) != 0,
            double_buffer: (output.dwFlags & winapi::PFD_DOUBLEBUFFER) != 0,
            multisampling: None,
            srgb: false,
        }));
    }

    result
}

/// Enumerates the list of pixel formats by using extra WGL functions.
///
/// Gives more precise results than `enumerate_native_pixel_formats`.
unsafe fn enumerate_arb_pixel_formats(extra: &gl::wgl_extra::Wgl, extensions: &str,
                                      hdc: winapi::HDC) -> Vec<(c_int, PixelFormat)>
{
    let get_info = |index: u32, attrib: u32| {
        let mut value = mem::uninitialized();
        extra.GetPixelFormatAttribivARB(hdc as *const _, index as c_int,
                                        0, 1, [attrib as c_int].as_ptr(),
                                        &mut value);
        value as u32
    };

    // getting the number of formats
    // the `1` is ignored
    let num = get_info(1, gl::wgl_extra::NUMBER_PIXEL_FORMATS_ARB);

    let mut result = Vec::new();

    for index in (0 .. num) {
        if get_info(index, gl::wgl_extra::DRAW_TO_WINDOW_ARB) == 0 {
            continue;
        }
        if get_info(index, gl::wgl_extra::SUPPORT_OPENGL_ARB) == 0 {
            continue;
        }

        if get_info(index, gl::wgl_extra::ACCELERATION_ARB) == gl::wgl_extra::NO_ACCELERATION_ARB {
            continue;
        }

        if get_info(index, gl::wgl_extra::PIXEL_TYPE_ARB) != gl::wgl_extra::TYPE_RGBA_ARB {
            continue;
        }

        result.push((index as c_int, PixelFormat {
            hardware_accelerated: true,
            color_bits: get_info(index, gl::wgl_extra::RED_BITS_ARB) as u8 + 
                        get_info(index, gl::wgl_extra::GREEN_BITS_ARB) as u8 +
                        get_info(index, gl::wgl_extra::BLUE_BITS_ARB) as u8,
            alpha_bits: get_info(index, gl::wgl_extra::ALPHA_BITS_ARB) as u8,
            depth_bits: get_info(index, gl::wgl_extra::DEPTH_BITS_ARB) as u8,
            stencil_bits: get_info(index, gl::wgl_extra::STENCIL_BITS_ARB) as u8,
            stereoscopy: get_info(index, gl::wgl_extra::STEREO_ARB) != 0,
            double_buffer: get_info(index, gl::wgl_extra::DOUBLE_BUFFER_ARB) != 0,
            multisampling: {
                if extensions.split(' ').find(|&i| i == "WGL_ARB_multisample").is_some() {
                    match get_info(index, gl::wgl_extra::SAMPLES_ARB) {
                        0 => None,
                        a => Some(a as u16),
                    }
                } else {
                    None
                }
            },
            srgb: if extensions.split(' ').find(|&i| i == "WGL_ARB_framebuffer_sRGB").is_some() {
                get_info(index, gl::wgl_extra::FRAMEBUFFER_SRGB_CAPABLE_ARB) != 0
            } else if extensions.split(' ').find(|&i| i == "WGL_EXT_framebuffer_sRGB").is_some() {
                get_info(index, gl::wgl_extra::FRAMEBUFFER_SRGB_CAPABLE_EXT) != 0
            } else {
                false
            },
        }));
    }

    result
}

/// Calls `SetPixelFormat` on a window.
unsafe fn set_pixel_format(hdc: winapi::HDC, id: c_int) -> Result<(), CreationError> {
    let mut output: winapi::PIXELFORMATDESCRIPTOR = mem::zeroed();

    if gdi32::DescribePixelFormat(hdc, id, mem::size_of::<winapi::PIXELFORMATDESCRIPTOR>()
                                  as winapi::UINT, &mut output) == 0
    {
        return Err(CreationError::OsError(format!("DescribePixelFormat function failed: {}",
                                                  format!("{}", io::Error::last_os_error()))));
    }

    if gdi32::SetPixelFormat(hdc, id, &output) == 0 {
        return Err(CreationError::OsError(format!("SetPixelFormat function failed: {}",
                                                  format!("{}", io::Error::last_os_error()))));
    }

    Ok(())
}

/// Loads the `opengl32.dll` library.
unsafe fn load_opengl32_dll() -> Result<winapi::HMODULE, CreationError> {
    let name = OsStr::new("opengl32.dll").encode_wide().chain(Some(0).into_iter())
                                         .collect::<Vec<_>>();

    let lib = kernel32::LoadLibraryW(name.as_ptr());

    if lib.is_null() {
        return Err(CreationError::OsError(format!("LoadLibrary function failed: {}",
                                                  format!("{}", io::Error::last_os_error()))));
    }

    Ok(lib)
}

/// Loads the WGL functions that are not guaranteed to be supported.
///
/// The `window` must be passed because the driver can vary depending on the window's
/// characteristics.
unsafe fn load_extra_functions(window: winapi::HWND) -> Result<gl::wgl_extra::Wgl, CreationError> {
    let (ex_style, style) = (winapi::WS_EX_APPWINDOW, winapi::WS_POPUP |
                             winapi::WS_CLIPSIBLINGS | winapi::WS_CLIPCHILDREN);

    // creating a dummy invisible window
    let dummy_window = {
        // getting the rect of the real window
        let rect = {
            let mut placement: winapi::WINDOWPLACEMENT = mem::zeroed();
            placement.length = mem::size_of::<winapi::WINDOWPLACEMENT>() as winapi::UINT;
            if user32::GetWindowPlacement(window, &mut placement) == 0 {
                panic!();
            }
            placement.rcNormalPosition
        };

        // getting the class name of the real window
        let mut class_name = [0u16; 128];
        if user32::GetClassNameW(window, class_name.as_mut_ptr(), 128) == 0 {
            return Err(CreationError::OsError(format!("GetClassNameW function failed: {}",
                                              format!("{}", io::Error::last_os_error()))));
        }

        // this dummy window should match the real one enough to get the same OpenGL driver
        let win = user32::CreateWindowExW(ex_style, class_name.as_ptr(),
                                          b"dummy window\0".as_ptr() as *const _, style,
                                          winapi::CW_USEDEFAULT, winapi::CW_USEDEFAULT,
                                          rect.right - rect.left,
                                          rect.bottom - rect.top,
                                          ptr::null_mut(), ptr::null_mut(),
                                          kernel32::GetModuleHandleW(ptr::null()),
                                          ptr::null_mut());

        if win.is_null() {
            return Err(CreationError::OsError(format!("CreateWindowEx function failed: {}",
                                              format!("{}", io::Error::last_os_error()))));
        }

        let hdc = user32::GetDC(win);
        if hdc.is_null() {
            let err = Err(CreationError::OsError(format!("GetDC function failed: {}",
                                               format!("{}", io::Error::last_os_error()))));
            return err;
        }

        WindowWrapper(win, hdc)
    };

    // getting the pixel format that we will use and setting it
    {
        let formats = enumerate_native_pixel_formats(dummy_window.1);
        let id = try!(choose_dummy_pixel_format(formats.into_iter()));
        try!(set_pixel_format(dummy_window.1, id));
    }

    // creating the dummy OpenGL context and making it current
    let dummy_context = try!(create_context(None, dummy_window.0, dummy_window.1));
    let _current_context = try!(CurrentContextGuard::make_current(dummy_window.1,
                                                                  dummy_context.0));

    // loading the extra WGL functions
    Ok(gl::wgl_extra::Wgl::load_with(|addr| {
        let addr = CString::new(addr.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        gl::wgl::GetProcAddress(addr) as *const c_void
    }))
}

/// Given a list of pixel formats, this function chooses one that is likely to be provided by
/// the main video driver of the system.
fn choose_dummy_pixel_format<I>(iter: I) -> Result<c_int, CreationError>
                                where I: Iterator<Item=(c_int, PixelFormat)>
{
    let mut backup_id = None;

    for (id, format) in iter {
        if backup_id.is_none() {
            backup_id = Some(id);
        }

        if format.hardware_accelerated {
            return Ok(id);
        }
    }

    backup_id.ok_or(CreationError::OsError("No available pixel format".to_string()))
}
