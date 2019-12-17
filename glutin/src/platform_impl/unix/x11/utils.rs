use super::Display;

use x11_dl::xlib::{VisualID, VisualIDMask, XVisualInfo};

use std::sync::Arc;

pub fn get_visual_info_from_xid(
    disp: &Display,
    xid: VisualID,
) -> XVisualInfo {
    let xlib = syms!(XLIB);

    assert_ne!(xid, 0);
    let mut template: XVisualInfo = unsafe { std::mem::zeroed() };
    template.visualid = xid;

    let mut num_visuals = 0;
    let vi = unsafe {
        (xlib.XGetVisualInfo)(
            **disp.native_display,
            VisualIDMask,
            &mut template,
            &mut num_visuals,
        )
    };
    disp
        .native_display
        .check_errors()
        .expect("[glutin] Failed to call `XGetVisualInfo`");
    assert!(!vi.is_null());
    assert!(num_visuals == 1);

    let vi_copy = unsafe { std::ptr::read(vi as *const _) };
    unsafe { (xlib.XFree)(vi as *mut _);
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
    disp: &Display,
    visual_infos: XVisualInfo,
    want_transparency: bool,
    want_xid: Option<VisualID>,
) -> Result<(), Lacks> {
    if let Some(want_xid) = want_xid {
        if visual_infos.visualid != want_xid {
            return Err(Lacks::XID);
        }
    }

    unsafe {
        if want_transparency {
            let pict_format = (syms!(XRENDER).XRenderFindVisualFormat)(
                **disp.native_display as *mut _,
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
