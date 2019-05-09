use crate::platform::unix::x11::XConnection;
use glutin_glx_sys as ffi;

use std::sync::Arc;

pub fn get_visual_info_from_xid(
    xconn: &Arc<XConnection>,
    xid: ffi::VisualID,
) -> ffi::XVisualInfo {
    assert_ne!(xid, 0);
    let mut template: ffi::XVisualInfo = unsafe { std::mem::zeroed() };
    template.visualid = xid;

    let mut num_visuals = 0;
    let vi = unsafe {
        (xconn.xlib.XGetVisualInfo)(
            xconn.display,
            ffi::VisualIDMask,
            &mut template,
            &mut num_visuals,
        )
    };
    xconn
        .check_errors()
        .expect("Failed to call `XGetVisualInfo`");
    assert!(!vi.is_null());
    assert!(num_visuals == 1);

    let vi_copy = unsafe { std::ptr::read(vi as *const _) };
    unsafe {
        (xconn.xlib.XFree)(vi as *mut _);
    }
    vi_copy
}

#[derive(Clone, Copy, Debug)]
pub enum Lacks {
    Transparency,
    XID,
}

/// Should always check for lack of xid before lack of transparency.
pub fn examine_visual_info(
    xconn: &Arc<XConnection>,
    visual_infos: ffi::XVisualInfo,
    want_transparency: bool,
    want_xid: Option<ffi::VisualID>,
) -> Result<(), Lacks> {
    if let Some(want_xid) = want_xid {
        if visual_infos.visualid != want_xid {
            return Err(Lacks::XID);
        }
    }

    unsafe {
        if want_transparency {
            let pict_format = (xconn.xrender.XRenderFindVisualFormat)(
                xconn.display as *mut _,
                visual_infos.visual,
            );
            if pict_format.is_null() {
                return Err(Lacks::Transparency);
            }

            if (*pict_format).direct.alphaMask == 0 {
                return Err(Lacks::Transparency);
            }
        }
    }

    return Ok(());
}

pub use super::select_config;
pub use crate::api::egl::SurfaceType;
