//! WGL Api.

use std::collections::HashSet;
use std::ffi::{CString, OsStr};
use std::io::Error as IoError;
use std::mem;
use std::ops::Deref;
use std::os::windows::ffi::OsStrExt;

use glutin_wgl_sys::{wgl, wgl_extra};
use once_cell::sync::OnceCell;
use windows_sys::Win32::Foundation::{HMODULE, HWND};
use windows_sys::Win32::Graphics::{Gdi as gdi, OpenGL as gl};
use windows_sys::Win32::UI::WindowsAndMessaging::{self as wm, WINDOWPLACEMENT, WNDCLASSEXW};

use crate::error::{Error, ErrorKind, Result};

pub mod config;
pub mod context;
pub mod display;
pub mod surface;

pub(crate) static WGL_EXTRA: OnceCell<WglExtra> = OnceCell::new();

pub(crate) struct WglExtra(wgl_extra::Wgl);

unsafe impl Send for WglExtra {}
unsafe impl Sync for WglExtra {}

impl WglExtra {
    fn new() -> Self {
        Self(wgl_extra::Wgl::load_with(|addr| unsafe {
            let addr = CString::new(addr.as_bytes()).unwrap();
            let addr = addr.as_ptr();
            wgl::GetProcAddress(addr).cast()
        }))
    }
}

impl Deref for WglExtra {
    type Target = wgl_extra::Wgl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe fn load_extra_functions(
    instance: HMODULE,
    win: HWND,
) -> Result<(&'static WglExtra, HashSet<&'static str>)> {
    let rect = unsafe {
        let mut placement: WINDOWPLACEMENT = std::mem::zeroed();
        placement.length = mem::size_of::<WINDOWPLACEMENT>() as _;
        if wm::GetWindowPlacement(win, &mut placement) == 0 {
            return Err(IoError::last_os_error().into());
        }
        placement.rcNormalPosition
    };

    let mut class_name = [0u16; 128];
    unsafe {
        if wm::GetClassNameW(win, class_name.as_mut_ptr(), 128) == 0 {
            return Err(IoError::last_os_error().into());
        }
    }

    let mut class = unsafe {
        let mut class: WNDCLASSEXW = std::mem::zeroed();
        if wm::GetClassInfoExW(instance, class_name.as_ptr(), &mut class) == 0 {
            return Err(IoError::last_os_error().into());
        }

        class
    };

    let class_name = OsStr::new("WglDummy Window")
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();

    class.cbSize = mem::size_of::<WNDCLASSEXW>() as _;
    class.lpszClassName = class_name.as_ptr();
    class.lpfnWndProc = Some(wm::DefWindowProcW);

    // This shouldn't fail if the registration of the real window class
    // worked. Multiple registrations of the window class trigger an
    // error which we want to ignore silently (e.g for multi-window
    // setups).
    unsafe { wm::RegisterClassExW(&class) };

    // This dummy window should match the real one enough to get the same OpenGL
    // driver.
    let title = OsStr::new("dummy window")
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();

    let ex_style = wm::WS_EX_APPWINDOW;
    let style = wm::WS_POPUP | wm::WS_CLIPSIBLINGS | wm::WS_CLIPCHILDREN;
    let win = unsafe {
        wm::CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            title.as_ptr() as _,
            style,
            wm::CW_USEDEFAULT,
            wm::CW_USEDEFAULT,
            rect.right - rect.left,
            rect.bottom - rect.top,
            0,
            0,
            instance,
            std::ptr::null_mut(),
        )
    };

    if win == 0 {
        return Err(IoError::last_os_error().into());
    }

    // Recovery may enter with a live context. Restore its binding after loading WGL.
    let previous_context = unsafe { gl::wglGetCurrentContext() };
    let previous_dc = unsafe { gl::wglGetCurrentDC() };
    let mut context = 0;
    let mut binding_attempted = false;
    let hdc = unsafe { gdi::GetDC(win) };
    // Keep initialization errors before any cleanup call changes the OS error.
    let result = (|| -> Result<_> {
        if hdc == 0 {
            return Err(IoError::last_os_error().into());
        }
        unsafe {
            let (pixel_format_index, descriptor) = config::choose_dummy_pixel_format(hdc)?;
            if gl::SetPixelFormat(hdc, pixel_format_index, &descriptor) == 0 {
                return Err(IoError::last_os_error().into());
            }
            context = gl::wglCreateContext(hdc);
            if context == 0 {
                return Err(IoError::last_os_error().into());
            }
            // A failed MakeCurrent can also clear the previous binding.
            binding_attempted = true;
            if gl::wglMakeCurrent(hdc, context) == 0 {
                return Err(IoError::last_os_error().into());
            }
        }

        // Load WGL.
        let wgl_extra = WGL_EXTRA.get_or_init(WglExtra::new);
        let client_extensions = display::load_extensions(hdc, wgl_extra);
        Ok((wgl_extra, client_extensions))
    })();

    let mut cleanup_error: Option<Error> = None;
    unsafe {
        if binding_attempted && gl::wglMakeCurrent(previous_dc, previous_context) == 0 {
            cleanup_error = Some(IoError::last_os_error().into());
        }

        // If restoration failed, detach the dummy before deleting its resources.
        if context != 0 && gl::wglGetCurrentContext() == context {
            if gl::wglMakeCurrent(0, 0) == 0 {
                let error = IoError::last_os_error();
                cleanup_error.get_or_insert_with(|| error.into());
            }
        }

        let mut context_deleted = context == 0;
        if context != 0 && gl::wglGetCurrentContext() != context {
            if gl::wglDeleteContext(context) == 0 {
                let error = IoError::last_os_error();
                cleanup_error.get_or_insert_with(|| error.into());
            } else {
                context_deleted = true;
            }
        }

        // Do not invalidate a drawable still needed by an undeleted dummy context.
        if context_deleted {
            if hdc != 0 && gdi::ReleaseDC(win, hdc) == 0 {
                // ReleaseDC does not document GetLastError support.
                cleanup_error.get_or_insert_with(|| {
                    Error::new(None, Some("dummy ReleaseDC failed".into()), ErrorKind::Misc)
                });
            }
            if wm::DestroyWindow(win) == 0 {
                let error = IoError::last_os_error();
                cleanup_error.get_or_insert_with(|| error.into());
            }
        }
    }

    // Prefer the original initialization error, but never report cleanup failure as success.
    result.and_then(|loaded| match cleanup_error {
        Some(error) => Err(error),
        None => Ok(loaded),
    })
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        let raw = error.raw_os_error().map(|code| code as i64);
        Error::new(raw, Some(error.to_string()), ErrorKind::Misc)
    }
}
