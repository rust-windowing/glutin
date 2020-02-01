use crate::api::glx::ffi;

use glutin_x11_sym::Display;
use winit_types::error::Error;
use winit_types::platform::OsError;

use std::os::raw;
use std::ptr;
use std::sync::Arc;

#[inline]
pub fn get_visual_info_from_xid(
    disp: &Arc<Display>,
    xid: ffi::VisualID,
) -> Result<ffi::XVisualInfo, Error> {
    let xlib = syms!(XLIB);

    if xid == 0 {
        return Err(make_oserror!(OsError::Misc(
            "Can not get XVisualInfo of xid 0".to_string()
        )));
    }

    let mut template: ffi::XVisualInfo = unsafe { std::mem::zeroed() };
    template.visualid = xid;

    let mut num_visuals = 0;
    let vi = unsafe {
        (xlib.XGetVisualInfo)(***disp, ffi::VisualIDMask, &mut template, &mut num_visuals)
    };

    disp.check_errors().map_err(|err| unsafe {
        if !vi.is_null() {
            (xlib.XFree)(vi as *mut _);
        }
        err
    })?;

    if vi.is_null() {
        return Err(make_oserror!(OsError::Misc(format!(
            "Tried to get XVisualInfo of xid {:?} but got NULL",
            xid
        ))));
    }

    if num_visuals != 1 {
        unsafe {
            (xlib.XFree)(vi as *mut _);
            return Err(make_oserror!(OsError::Misc(format!(
                "Tried to get XVisualInfo of xid {:?} but got returned {:?} visuals",
                xid, num_visuals
            ))));
        }
    }

    let vi_copy: ffi::XVisualInfo = unsafe { ptr::read(vi as *const _) };
    unsafe {
        (xlib.XFree)(vi as *mut _);
    }
    Ok(vi_copy)
}

#[derive(Clone, Copy, Debug)]
pub enum Lacks {
    Transparency,
    XID,
}

/// Should always check for lack of xid before lack of transparency.
#[inline]
pub fn examine_visual_info(
    disp: &Arc<Display>,
    visual_infos: ffi::XVisualInfo,
    wants_transparency: Option<bool>,
    target_visual_xid: Option<raw::c_ulong>,
) -> Result<(), Lacks> {
    if let Some(target_visual_xid) = target_visual_xid {
        if visual_infos.visualid != target_visual_xid {
            return Err(Lacks::XID);
        }
    }

    if let Some(wants_transparency) = wants_transparency {
        unsafe {
            if wants_transparency {
                let pict_format =
                    (syms!(XRENDER).XRenderFindVisualFormat)(***disp, visual_infos.visual);
                if pict_format.is_null() {
                    return Err(Lacks::Transparency);
                }

                if (*pict_format).direct.alphaMask == 0 {
                    return Err(Lacks::Transparency);
                }
            }
        }
    }

    Ok(())
}
