use {Event, WindowBuilder, KeyModifiers};
use libc;
use std::{mem, ptr};
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use super::ffi;
use sync::one::{Once, ONCE_INIT};

pub use self::monitor::{MonitorID, get_available_monitors, get_primary_monitor};

mod events;
mod monitor;

static THREAD_INIT: Once = ONCE_INIT;

fn ensure_thread_init() {
    THREAD_INIT.doit(|| {
        unsafe {
            ffi::XInitThreads();
        }
    });
}

pub struct Window {
    display: *mut ffi::Display,
    window: ffi::Window,
    im: ffi::XIM,
    ic: ffi::XIC,
    context: ffi::GLXContext,
    is_closed: AtomicBool,
    wm_delete_window: ffi::Atom,
    xf86_desk_mode: *mut ffi::XF86VidModeModeInfo,
    screen_id: libc::c_int,
    is_fullscreen: bool,
    current_modifiers: Cell<KeyModifiers>,
}

impl Window {
    pub fn new(builder: WindowBuilder) -> Result<Window, String> {
        ensure_thread_init();
        let dimensions = builder.dimensions.unwrap_or((800, 600));

        // calling XOpenDisplay
        let display = unsafe {
            let display = ffi::XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(format!("XOpenDisplay failed"));
            }
            display
        };

        let screen_id = match builder.monitor {
            Some(MonitorID(monitor)) => monitor as i32,
            None => unsafe { ffi::XDefaultScreen(display) },
        };

        // getting the FBConfig
        let fb_config = unsafe {
            const VISUAL_ATTRIBUTES: [libc::c_int, ..23] = [
                ffi::GLX_X_RENDERABLE,  1,
                ffi::GLX_DRAWABLE_TYPE, ffi::GLX_WINDOW_BIT,
                ffi::GLX_RENDER_TYPE,   ffi::GLX_RGBA_BIT,
                ffi::GLX_X_VISUAL_TYPE, ffi::GLX_TRUE_COLOR,
                ffi::GLX_RED_SIZE,      8,
                ffi::GLX_GREEN_SIZE,    8,
                ffi::GLX_BLUE_SIZE,     8,
                ffi::GLX_ALPHA_SIZE,    8,
                ffi::GLX_DEPTH_SIZE,    24,
                ffi::GLX_STENCIL_SIZE,  8,
                ffi::GLX_DOUBLEBUFFER,  1,
                0
            ];

            let mut num_fb: libc::c_int = mem::uninitialized();

            let fb = ffi::glx::ChooseFBConfig(display, ffi::XDefaultScreen(display),
                VISUAL_ATTRIBUTES.as_ptr(), &mut num_fb);
            if fb.is_null() {
                return Err(format!("glx::ChooseFBConfig failed"));
            }
            let preferred_fb = *fb;     // TODO: choose more wisely
            ffi::XFree(fb as *const libc::c_void);
            preferred_fb
        };

        let mut best_mode = -1;
        let modes = unsafe {
            let mut mode_num: libc::c_int = mem::uninitialized();
            let mut modes: *mut *mut ffi::XF86VidModeModeInfo = mem::uninitialized();
            if ffi::XF86VidModeGetAllModeLines(display, screen_id, &mut mode_num, &mut modes) == 0 {
                return Err(format!("Could not query the video modes"));
            }

            for i in range(0, mode_num) {
                let mode: ffi::XF86VidModeModeInfo = **modes.offset(i as int);
                if mode.hdisplay == dimensions.val0() as u16 && mode.vdisplay == dimensions.val1() as u16 {
                    best_mode = i;
                }
            };
            if best_mode == -1 && builder.monitor.is_some() {
                return Err(format!("Could not find a suitable graphics mode"));
            }

           modes
        };

        let xf86_desk_mode = unsafe {
            *modes.offset(0)
        };

        // getting the visual infos
        let mut visual_infos = unsafe {
            let vi = ffi::glx::GetVisualFromFBConfig(display, fb_config);
            if vi.is_null() {
                return Err(format!("glx::ChooseVisual failed"));
            }
            let vi_copy = *vi;
            ffi::XFree(vi as *const libc::c_void);
            vi_copy
        };

        // getting the root window
        let root = unsafe { ffi::XDefaultRootWindow(display) };

        // creating the color map
        let cmap = unsafe {
            let cmap = ffi::XCreateColormap(display, root,
                visual_infos.visual, ffi::AllocNone);
            // TODO: error checking?
            cmap
        };

        // creating
        let mut set_win_attr = {
            let mut swa: ffi::XSetWindowAttributes = unsafe { mem::zeroed() };
            swa.colormap = cmap;
            swa.event_mask = ffi::ExposureMask | ffi::ResizeRedirectMask |
                ffi::VisibilityChangeMask | ffi::KeyPressMask | ffi::PointerMotionMask |
                ffi::KeyReleaseMask | ffi::ButtonPressMask |
                ffi::ButtonReleaseMask | ffi::KeymapStateMask;
            swa.border_pixel = 0;
            swa.override_redirect = 0;
            swa
        };

        let mut window_attributes = ffi::CWBorderPixel | ffi::CWColormap | ffi:: CWEventMask;
        if builder.monitor.is_some() {
            window_attributes |= ffi::CWOverrideRedirect;
            unsafe {
                ffi::XF86VidModeSwitchToMode(display, screen_id, *modes.offset(best_mode as int));
                ffi::XF86VidModeSetViewPort(display, screen_id, 0, 0);
                set_win_attr.override_redirect = 1;
            }
        }

        // finally creating the window
        let window = unsafe {
            let win = ffi::XCreateWindow(display, root, 0, 0, dimensions.val0() as libc::c_uint,
                dimensions.val1() as libc::c_uint, 0, visual_infos.depth, ffi::InputOutput,
                visual_infos.visual, window_attributes,
                &mut set_win_attr);
            win
        };

        // creating window, step 2
        let wm_delete_window = unsafe {
            use std::c_str::ToCStr;

            ffi::XMapWindow(display, window);
            let mut wm_delete_window = ffi::XInternAtom(display,
                "WM_DELETE_WINDOW".to_c_str().as_ptr() as *const libc::c_char, 0);
            ffi::XSetWMProtocols(display, window, &mut wm_delete_window, 1);
            let c_title = builder.title.to_c_str();
            ffi::XStoreName(display, window, c_title.as_ptr());
            ffi::XFlush(display);

            wm_delete_window
        };

        // creating IM
        let im = unsafe {
            let im = ffi::XOpenIM(display, ptr::null(), ptr::null_mut(), ptr::null_mut());
            if im.is_null() {
                return Err(format!("XOpenIM failed"));
            }
            im
        };

        // creating input context
        let ic = unsafe {
            use std::c_str::ToCStr;

            let ic = ffi::XCreateIC(im, "inputStyle".to_c_str().as_ptr(),
                ffi::XIMPreeditNothing | ffi::XIMStatusNothing, "clientWindow".to_c_str().as_ptr(),
                window, ptr::null());
            if ic.is_null() {
                return Err(format!("XCreateIC failed"));
            }
            ffi::XSetICFocus(ic);
            ic
        };

        // Attempt to make keyboard input repeat detectable
        unsafe {
            let mut supported_ptr = false;
            ffi::XkbSetDetectableAutoRepeat(display, true, &mut supported_ptr);
            if !supported_ptr {
                return Err(format!("XkbSetDetectableAutoRepeat failed"));
            }
        }


        // creating GL context
        let context = unsafe {
            let mut attributes = Vec::new();

            if builder.gl_version.is_some() {
                let version = builder.gl_version.as_ref().unwrap();
                attributes.push(ffi::GLX_CONTEXT_MAJOR_VERSION);
                attributes.push(version.val0() as libc::c_int);
                attributes.push(ffi::GLX_CONTEXT_MINOR_VERSION);
                attributes.push(version.val1() as libc::c_int);
            }

            attributes.push(0);

            // loading the extra GLX functions
            let extra_functions = ffi::glx_extra::Glx::load_with(|addr| {
                addr.with_c_str(|s| {
                    use libc;
                    ffi::glx::GetProcAddress(s as *const u8) as *const libc::c_void
                })
            });

            let context = if extra_functions.CreateContextAttribsARB.is_loaded() {
                extra_functions.CreateContextAttribsARB(display as *mut ffi::glx_extra::types::Display,
                    fb_config, ptr::null(), 1, attributes.as_ptr())
            } else {
                ffi::glx::CreateContext(display, &mut visual_infos, ptr::null(), 1)
            };

            if context.is_null() {
                return Err(format!("GL context creation failed"));
            }

            context
        };

        // creating the window object
        let window = Window {
            display: display,
            window: window,
            im: im,
            ic: ic,
            context: context,
            is_closed: AtomicBool::new(false),
            wm_delete_window: wm_delete_window,
            xf86_desk_mode: xf86_desk_mode,
            screen_id: screen_id,
            is_fullscreen: builder.monitor.is_some(),
            current_modifiers: Cell::new(KeyModifiers::empty()),
        };

        // calling glViewport
        unsafe {
            let ptr = window.get_proc_address("glViewport");
            assert!(!ptr.is_null());
            let ptr: extern "system" fn(libc::c_int, libc::c_int, libc::c_int, libc::c_int) =
                mem::transmute(ptr);
            let dimensions = window.get_inner_size().unwrap();
            ptr(0, 0, dimensions.val0() as libc::c_int, dimensions.val1() as libc::c_int);
        }

        // returning
        Ok(window)
    }

    pub fn is_closed(&self) -> bool {
        use std::sync::atomic::Relaxed;
        self.is_closed.load(Relaxed)
    }

    pub fn set_title(&self, title: &str) {
        let c_title = title.to_c_str();
        unsafe {
            ffi::XStoreName(self.display, self.window, c_title.as_ptr());
            ffi::XFlush(self.display);
        }
    }

    fn get_geometry(&self) -> Option<(int, int, uint, uint)> {
        unsafe {
            use std::mem;

            let mut root: ffi::Window = mem::uninitialized();
            let mut x: libc::c_int = mem::uninitialized();
            let mut y: libc::c_int = mem::uninitialized();
            let mut width: libc::c_uint = mem::uninitialized();
            let mut height: libc::c_uint = mem::uninitialized();
            let mut border: libc::c_uint = mem::uninitialized();
            let mut depth: libc::c_uint = mem::uninitialized();

            if ffi::XGetGeometry(self.display, self.window,
                &mut root, &mut x, &mut y, &mut width, &mut height,
                &mut border, &mut depth) == 0
            {
                return None;
            }

            Some((x as int, y as int, width as uint, height as uint))
        }
    }

    pub fn get_position(&self) -> Option<(int, int)> {
        self.get_geometry().map(|(x, y, _, _)| (x, y))
    }

    pub fn set_position(&self, x: int, y: int) {
        unsafe { ffi::XMoveWindow(self.display, self.window, x as libc::c_int, y as libc::c_int) }
    }

    pub fn get_inner_size(&self) -> Option<(uint, uint)> {
        self.get_geometry().map(|(_, _, w, h)| (w, h))
    }

    pub fn get_outer_size(&self) -> Option<(uint, uint)> {
        unimplemented!()
    }

    pub fn set_inner_size(&self, _x: uint, _y: uint) {
        unimplemented!()
    }

    pub fn poll_events(&self) -> Vec<Event> {
        use std::mem;

        let mut events = Vec::new();

        loop {
            use std::num::Bounded;

            let mut xev = unsafe { mem::uninitialized() };
            let res = unsafe { ffi::XCheckMaskEvent(self.display, Bounded::max_value(), &mut xev) };

            if res == 0 {
                let res = unsafe { ffi::XCheckTypedEvent(self.display, ffi::ClientMessage, &mut xev) };

                if res == 0 {
                    break
                }
            }

            match xev.type_ {
                ffi::KeymapNotify => {
                    unsafe { ffi::XRefreshKeyboardMapping(&xev) }
                },

                ffi::ClientMessage => {
                    use Closed;
                    use std::sync::atomic::Relaxed;

                    let client_msg: &ffi::XClientMessageEvent = unsafe { mem::transmute(&xev) };

                    if client_msg.l[0] == self.wm_delete_window as libc::c_long {
                        self.is_closed.store(true, Relaxed);
                        events.push(Closed);
                    }
                },

                ffi::ResizeRequest => {
                    use Resized;
                    let rs_event: &ffi::XResizeRequestEvent = unsafe { mem::transmute(&xev) };
                    events.push(Resized(rs_event.width as uint, rs_event.height as uint));
                },

                ffi::MotionNotify => {
                    use MouseMoved;
                    let event: &ffi::XMotionEvent = unsafe { mem::transmute(&xev) };
                    events.push(MouseMoved((event.x as int, event.y as int)));
                },

                ffi::KeyPress | ffi::KeyRelease => {
                    use {KeyboardInput, Pressed, Released, ReceivedCharacter};
                    use {LEFT_CONTROL_MODIFIER, RIGHT_CONTROL_MODIFIER};
                    use {LEFT_SHIFT_MODIFIER, RIGHT_SHIFT_MODIFIER};
                    use {LEFT_ALT_MODIFIER, RIGHT_ALT_MODIFIER, CAPS_LOCK_MODIFIER};
                    let event: &mut ffi::XKeyEvent = unsafe { mem::transmute(&xev) };

                    if event.type_ == ffi::KeyPress {
                        let raw_ev: *mut ffi::XKeyEvent = event;
                        unsafe { ffi::XFilterEvent(mem::transmute(raw_ev), self.window) };
                    }

                    let state = if xev.type_ == ffi::KeyPress { Pressed } else { Released };

                    let written = unsafe {
                        use std::str;

                        let mut buffer: [u8, ..16] = [mem::uninitialized(), ..16];
                        let raw_ev: *mut ffi::XKeyEvent = event;
                        let count = ffi::Xutf8LookupString(self.ic, mem::transmute(raw_ev),
                            mem::transmute(buffer.as_mut_ptr()),
                            buffer.len() as libc::c_int, ptr::null_mut(), ptr::null_mut());

                        str::from_utf8(buffer.as_slice().slice_to(count as uint))
                            .unwrap_or("").to_string()
                    };

                    for chr in written.as_slice().chars() {
                        events.push(ReceivedCharacter(chr));
                    }

                    let keysym = unsafe {
                        ffi::XKeycodeToKeysym(self.display, event.keycode as ffi::KeyCode, 0)
                    };

                    let modifier_flag = match keysym as u32 {
                        ffi::XK_Shift_L => Some(LEFT_SHIFT_MODIFIER),
                        ffi::XK_Shift_R => Some(RIGHT_SHIFT_MODIFIER),
                        ffi::XK_Control_L => Some(LEFT_CONTROL_MODIFIER),
                        ffi::XK_Control_R => Some(RIGHT_CONTROL_MODIFIER),
                        ffi::XK_Caps_Lock => Some(CAPS_LOCK_MODIFIER),
                        ffi::XK_Meta_L => Some(LEFT_ALT_MODIFIER),
                        ffi::XK_Meta_R => Some(RIGHT_ALT_MODIFIER),
                        _ => None,
                    };
                    match modifier_flag {
                        Some(flag) => {
                            let mut current_modifiers = self.current_modifiers.get();
                            match state {
                                Pressed => current_modifiers.insert(flag),
                                Released => current_modifiers.remove(flag),
                            }
                            self.current_modifiers.set(current_modifiers);
                        }
                        None => {}
                    }

                    let vkey =  events::keycode_to_element(keysym as libc::c_uint);

                    events.push(KeyboardInput(state, event.keycode as u8,
                        vkey, self.current_modifiers.get()));
                    //
                },

                ffi::ButtonPress | ffi::ButtonRelease => {
                    use {MouseInput, MouseWheel, Pressed, Released};
                    use {LeftMouseButton, RightMouseButton, MiddleMouseButton};
                    let event: &ffi::XButtonEvent = unsafe { mem::transmute(&xev) };

                    let state = if xev.type_ == ffi::ButtonPress { Pressed } else { Released };

                    let button = match event.button {
                        ffi::Button1 => Some(LeftMouseButton),
                        ffi::Button2 => Some(MiddleMouseButton),
                        ffi::Button3 => Some(RightMouseButton),
                        ffi::Button4 => {
                            events.push(MouseWheel(1, self.current_modifiers.get()));
                            None
                        }
                        ffi::Button5 => {
                            events.push(MouseWheel(-1, self.current_modifiers.get()));
                            None
                        }
                        _ => None
                    };

                    match button {
                        Some(button) =>
                            events.push(MouseInput(state, button)),
                        None => ()
                    };
                },

                _ => ()
            }
        }

        events
    }

    pub fn wait_events(&self) -> Vec<Event> {
        use std::mem;

        loop {
            // this will block until an event arrives, but doesn't remove
            //  it from the queue
            let mut xev = unsafe { mem::uninitialized() };
            unsafe { ffi::XPeekEvent(self.display, &mut xev) };

            // calling poll_events()
            let ev = self.poll_events();
            if ev.len() >= 1 {
                return ev;
            }
        }
    }

    pub unsafe fn make_current(&self) {
        let res = ffi::glx::MakeCurrent(self.display, self.window, self.context);
        if res == 0 {
            fail!("glx::MakeCurrent failed");
        }
    }

    pub fn get_proc_address(&self, addr: &str) -> *const () {
        use std::c_str::ToCStr;
        use std::mem;

        unsafe {
            addr.with_c_str(|s| {
                ffi::glx::GetProcAddress(mem::transmute(s)) as *const ()
            })
        }
    }

    pub fn swap_buffers(&self) {
        unsafe { ffi::glx::SwapBuffers(self.display, self.window) }
    }

    pub fn platform_display(&self) -> *mut libc::c_void {
        self.display as *mut libc::c_void
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { ffi::glx::MakeCurrent(self.display, 0, ptr::null()); }
        unsafe { ffi::glx::DestroyContext(self.display, self.context); }

        if self.is_fullscreen {
            unsafe { ffi::XF86VidModeSwitchToMode(self.display, self.screen_id, self.xf86_desk_mode); }
            unsafe { ffi::XF86VidModeSetViewPort(self.display, self.screen_id, 0, 0); }
        }

        unsafe { ffi::XDestroyIC(self.ic); }
        unsafe { ffi::XCloseIM(self.im); }
        unsafe { ffi::XDestroyWindow(self.display, self.window); }
        unsafe { ffi::XCloseDisplay(self.display); }
    }
}
