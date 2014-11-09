extern crate native;

use self::native::NativeTaskBuilder;
use std::task::TaskBuilder;
use std::sync::atomic::AtomicBool;
use std::ptr;
use super::{event, ffi};
use super::Window;
use {CreationError, OsError, Event};

/// Stores the current window and its events dispatcher.
/// 
/// We only have one window per thread. We still store the HWND in case where we
///  receive an event for another window.
local_data_key!(WINDOW: (ffi::HWND, Sender<Event>))

pub fn new_window(builder_dimensions: Option<(uint, uint)>, builder_title: String,
                  builder_monitor: Option<super::MonitorID>,
                  builder_gl_version: Option<(uint, uint)>, builder_debug: bool,
                  builder_vsync: bool, builder_hidden: bool) -> Result<Window, CreationError>
{
    use std::mem;
    use std::os;

    // initializing variables to be sent to the task
    let title = builder_title.as_slice().utf16_units()
        .chain(Some(0).into_iter()).collect::<Vec<u16>>();    // title to utf16
    //let hints = hints.clone();
    let (tx, rx) = channel();

    // GetMessage must be called in the same thread as CreateWindow,
    //  so we create a new thread dedicated to this window.
    // This is the only safe method. Using `nosend` wouldn't work for non-native runtime.
    TaskBuilder::new().native().spawn(proc() {
        // registering the window class
        let class_name = {
            let class_name: Vec<u16> = "Window Class".utf16_units().chain(Some(0).into_iter())
                .collect::<Vec<u16>>();
            
            let class = ffi::WNDCLASSEX {
                cbSize: mem::size_of::<ffi::WNDCLASSEX>() as ffi::UINT,
                style: ffi::CS_HREDRAW | ffi::CS_VREDRAW | ffi::CS_OWNDC,
                lpfnWndProc: callback,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: unsafe { ffi::GetModuleHandleW(ptr::null()) },
                hIcon: ptr::null(),
                hCursor: ptr::null(),
                hbrBackground: ptr::null(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: ptr::null(),
            };

            // We ignore errors because registering the same window class twice would trigger
            //  an error, and because errors here are detected during CreateWindowEx anyway.
            // Also since there is no weird element in the struct, there is no reason for this
            //  call to fail.
            unsafe { ffi::RegisterClassExW(&class) };

            class_name
        };

        // building a RECT object with coordinates
        let mut rect = ffi::RECT {
            left: 0, right: builder_dimensions.unwrap_or((1024, 768)).val0() as ffi::LONG,
            top: 0, bottom: builder_dimensions.unwrap_or((1024, 768)).val1() as ffi::LONG,
        };

        // switching to fullscreen if necessary
        // this means adjusting the window's position so that it overlaps the right monitor,
        //  and change the monitor's resolution if necessary
        if builder_monitor.is_some() {
            let monitor = builder_monitor.as_ref().unwrap();

            // adjusting the rect
            {
                let pos = monitor.get_position();
                rect.left += pos.val0() as ffi::LONG;
                rect.right += pos.val0() as ffi::LONG;
                rect.top += pos.val1() as ffi::LONG;
                rect.bottom += pos.val1() as ffi::LONG;
            }

            // changing device settings
            let mut screen_settings: ffi::DEVMODE = unsafe { mem::zeroed() };
            screen_settings.dmSize = mem::size_of::<ffi::DEVMODE>() as ffi::WORD;
            screen_settings.dmPelsWidth = (rect.right - rect.left) as ffi::DWORD;
            screen_settings.dmPelsHeight = (rect.bottom - rect.top) as ffi::DWORD;
            screen_settings.dmBitsPerPel = 32;      // TODO: ?
            screen_settings.dmFields = ffi::DM_BITSPERPEL | ffi::DM_PELSWIDTH | ffi::DM_PELSHEIGHT;

            let result = unsafe { ffi::ChangeDisplaySettingsExW(monitor.get_system_name().as_ptr(),
                &mut screen_settings, ptr::null(), ffi::CDS_FULLSCREEN, ptr::null_mut()) };
            
            if result != ffi::DISP_CHANGE_SUCCESSFUL {
                tx.send(Err(OsError(format!("ChangeDisplaySettings failed: {}", result))));
                return;
            }
        }

        // computing the style and extended style of the window
        let (ex_style, style) = if builder_monitor.is_some() {
            (ffi::WS_EX_APPWINDOW, ffi::WS_POPUP | ffi::WS_CLIPSIBLINGS | ffi::WS_CLIPCHILDREN)
        } else {
            (ffi::WS_EX_APPWINDOW | ffi::WS_EX_WINDOWEDGE,
                ffi::WS_OVERLAPPEDWINDOW | ffi::WS_CLIPSIBLINGS | ffi::WS_CLIPCHILDREN)
        };

        // adjusting the window coordinates using the style
        unsafe { ffi::AdjustWindowRectEx(&mut rect, style, 0, ex_style) };

        // getting the address of wglCreateContextAttribsARB and the pixel format
        //  that we will use
        let (extra_functions, pixel_format) = {
            // creating a dummy invisible window for GL initialization
            let dummy_window = unsafe {
                let handle = ffi::CreateWindowExW(ex_style, class_name.as_ptr(),
                    title.as_ptr() as ffi::LPCWSTR,
                    style | ffi::WS_CLIPSIBLINGS | ffi::WS_CLIPCHILDREN,
                    ffi::CW_USEDEFAULT, ffi::CW_USEDEFAULT,
                    rect.right - rect.left, rect.bottom - rect.top,
                    ptr::null(), ptr::null(), ffi::GetModuleHandleW(ptr::null()),
                    ptr::null_mut());

                if handle.is_null() {
                    use std::os;
                    tx.send(Err(OsError(format!("CreateWindowEx function failed: {}",
                        os::error_string(os::errno() as uint)))));
                    return;
                }

                handle
            };

            // getting the HDC of the dummy window
            let dummy_hdc = {
                let hdc = unsafe { ffi::GetDC(dummy_window) };
                if hdc.is_null() {
                    tx.send(Err(OsError(format!("GetDC function failed: {}",
                        os::error_string(os::errno() as uint)))));
                    unsafe { ffi::DestroyWindow(dummy_window); }
                    return;
                }
                hdc
            };

            // getting the pixel format that we will use
            let pixel_format = {
                // initializing a PIXELFORMATDESCRIPTOR that indicates what we want
                let mut output: ffi::PIXELFORMATDESCRIPTOR = unsafe { mem::zeroed() };
                output.nSize = mem::size_of::<ffi::PIXELFORMATDESCRIPTOR>() as ffi::WORD;
                output.nVersion = 1;
                output.dwFlags = ffi::PFD_DRAW_TO_WINDOW | ffi::PFD_DOUBLEBUFFER |
                    ffi::PFD_SUPPORT_OPENGL | ffi::PFD_GENERIC_ACCELERATED;
                output.iPixelType = ffi::PFD_TYPE_RGBA;
                output.cColorBits = 24;
                output.cAlphaBits = 8;
                output.cAccumBits = 0;
                output.cDepthBits = 24;
                output.cStencilBits = 8;
                output.cAuxBuffers = 0;
                output.iLayerType = ffi::PFD_MAIN_PLANE;

                let pf_index = unsafe { ffi::ChoosePixelFormat(dummy_hdc, &output) };

                if pf_index == 0 {
                    tx.send(Err(OsError(format!("ChoosePixelFormat function failed: {}",
                        os::error_string(os::errno() as uint)))));
                    unsafe { ffi::DestroyWindow(dummy_window); }
                    return;
                }

                if unsafe { ffi::DescribePixelFormat(dummy_hdc, pf_index,
                    mem::size_of::<ffi::PIXELFORMATDESCRIPTOR>() as ffi::UINT, &mut output) } == 0
                {
                    tx.send(Err(OsError(format!("DescribePixelFormat function failed: {}",
                        os::error_string(os::errno() as uint)))));
                    unsafe { ffi::DestroyWindow(dummy_window); }
                    return;
                }

                output
            };

            // calling SetPixelFormat
            unsafe {
                if ffi::SetPixelFormat(dummy_hdc, 1, &pixel_format) == 0 {
                    tx.send(Err(OsError(format!("SetPixelFormat function failed: {}",
                        os::error_string(os::errno() as uint)))));
                    ffi::DestroyWindow(dummy_window);
                    return;
                }
            }

            // creating the dummy OpenGL context
            let dummy_context = {
                let ctxt = unsafe { ffi::wgl::CreateContext(dummy_hdc) };
                if ctxt.is_null() {
                    tx.send(Err(OsError(format!("wglCreateContext function failed: {}",
                        os::error_string(os::errno() as uint)))));
                    unsafe { ffi::DestroyWindow(dummy_window); }
                    return;
                }
                ctxt
            };

            // making context current
            unsafe { ffi::wgl::MakeCurrent(dummy_hdc, dummy_context); }

            // loading the extra WGL functions
            let extra_functions = ffi::wgl_extra::Wgl::load_with(|addr| {
                unsafe {
                    addr.with_c_str(|s| {
                        use libc;
                        ffi::wgl::GetProcAddress(s) as *const libc::c_void
                    })
                }
            });

            // removing current context
            unsafe { ffi::wgl::MakeCurrent(ptr::null(), ptr::null()); }

            // destroying the context and the window
            unsafe { ffi::wgl::DeleteContext(dummy_context); }
            unsafe { ffi::DestroyWindow(dummy_window); }

            // returning the address
            (extra_functions, pixel_format)
        };

        // creating the real window this time
        let real_window = unsafe {
            let (width, height) = if builder_monitor.is_some() || builder_dimensions.is_some() {
                (Some(rect.right - rect.left), Some(rect.bottom - rect.top))
            } else {
                (None, None)
            };

            let style = if builder_hidden {
                style
            } else {
                style | ffi::WS_VISIBLE
            };

            let handle = ffi::CreateWindowExW(ex_style, class_name.as_ptr(),
                title.as_ptr() as ffi::LPCWSTR,
                style | ffi::WS_CLIPSIBLINGS | ffi::WS_CLIPCHILDREN,
                if builder_monitor.is_some() { 0 } else { ffi::CW_USEDEFAULT },
                if builder_monitor.is_some() { 0 } else { ffi::CW_USEDEFAULT },
                width.unwrap_or(ffi::CW_USEDEFAULT), height.unwrap_or(ffi::CW_USEDEFAULT),
                ptr::null(), ptr::null(), ffi::GetModuleHandleW(ptr::null()),
                ptr::null_mut());

            if handle.is_null() {
                use std::os;
                tx.send(Err(OsError(format!("CreateWindowEx function failed: {}",
                    os::error_string(os::errno() as uint)))));
                return;
            }

            handle
        };

        // getting the HDC of the window
        let hdc = {
            let hdc = unsafe { ffi::GetDC(real_window) };
            if hdc.is_null() {
                tx.send(Err(OsError(format!("GetDC function failed: {}",
                    os::error_string(os::errno() as uint)))));
                unsafe { ffi::DestroyWindow(real_window); }
                return;
            }
            hdc
        };

        // calling SetPixelFormat
        unsafe {
            if ffi::SetPixelFormat(hdc, 1, &pixel_format) == 0 {
                tx.send(Err(OsError(format!("SetPixelFormat function failed: {}",
                    os::error_string(os::errno() as uint)))));
                ffi::DestroyWindow(real_window);
                return;
            }
        }

        // creating the OpenGL context
        let context = {
            use libc;

            let mut attributes = Vec::new();

            if builder_gl_version.is_some() {
                let version = builder_gl_version.as_ref().unwrap();
                attributes.push(ffi::wgl_extra::CONTEXT_MAJOR_VERSION_ARB as libc::c_int);
                attributes.push(version.val0() as libc::c_int);
                attributes.push(ffi::wgl_extra::CONTEXT_MINOR_VERSION_ARB as libc::c_int);
                attributes.push(version.val1() as libc::c_int);
            }

            if builder_debug {
                attributes.push(ffi::wgl_extra::CONTEXT_FLAGS_ARB as libc::c_int);
                attributes.push(ffi::wgl_extra::CONTEXT_DEBUG_BIT_ARB as libc::c_int);
            }

            attributes.push(0);

            let ctxt = unsafe {
                if extra_functions.CreateContextAttribsARB.is_loaded() {
                    extra_functions.CreateContextAttribsARB(hdc, ptr::null(),
                        attributes.as_slice().as_ptr())
                } else {
                    ffi::wgl::CreateContext(hdc)
                }
            };

            if ctxt.is_null() {
                tx.send(Err(OsError(format!("OpenGL context creation failed: {}",
                    os::error_string(os::errno() as uint)))));
                unsafe { ffi::DestroyWindow(real_window); }
                return;
            }

            ctxt
        };

        // calling SetForegroundWindow if fullscreen
        if builder_monitor.is_some() {
            unsafe { ffi::SetForegroundWindow(real_window) };
        }

        // filling the WINDOW task-local storage
        let events_receiver = {
            let (tx, rx) = channel();
            WINDOW.replace(Some((real_window, tx)));
            rx
        };

        // loading the opengl32 module
        let gl_library = {
            let name = "opengl32.dll".utf16_units().chain(Some(0).into_iter())
                .collect::<Vec<u16>>().as_ptr();
            let lib = unsafe { ffi::LoadLibraryW(name) };
            if lib.is_null() {
                tx.send(Err(OsError(format!("LoadLibrary function failed: {}",
                    os::error_string(os::errno() as uint)))));
                unsafe { ffi::wgl::DeleteContext(context); }
                unsafe { ffi::DestroyWindow(real_window); }
                return;
            }
            lib
        };

        // handling vsync
        if builder_vsync {
            if extra_functions.SwapIntervalEXT.is_loaded() {
                unsafe { ffi::wgl::MakeCurrent(hdc, context) };
                if unsafe { extra_functions.SwapIntervalEXT(1) } == 0 {
                    tx.send(Err(OsError(format!("wglSwapIntervalEXT failed"))));
                    unsafe { ffi::wgl::DeleteContext(context); }
                    unsafe { ffi::DestroyWindow(real_window); }
                    return;
                }

                // it is important to remove the current context, otherwise you get very weird
                // errors
                unsafe { ffi::wgl::MakeCurrent(ptr::null(), ptr::null()); }
            }
        }

        // building the struct
        let window = Window{
            window: real_window,
            hdc: hdc,
            context: context,
            gl_library: gl_library,
            events_receiver: events_receiver,
            is_closed: AtomicBool::new(false),
        };

        // sending
        tx.send(Ok(window));

        // now that the `Window` struct is initialized, the main `Window::new()` function will
        //  return and this events loop will run in parallel
        loop {
            let mut msg = unsafe { mem::uninitialized() };

            if unsafe { ffi::GetMessageW(&mut msg, ptr::null(), 0, 0) } == 0 {
                break;
            }

            unsafe { ffi::TranslateMessage(&msg) };
            unsafe { ffi::DispatchMessageW(&msg) };     // calls `callback` (see below)
        }
    });

    rx.recv()
}

/// Checks that the window is the good one, and if so send the event to it.
fn send_event(window: ffi::HWND, event: Event) {
    let stored = match WINDOW.get() {
        None => return,
        Some(v) => v
    };

    let &(ref win, ref sender) = stored.deref();

    if win != &window {
        return;
    }

    sender.send_opt(event).ok();  // ignoring if closed
}

/// This is the callback that is called by `DispatchMessage` in the events loop.
/// 
/// Returning 0 tells the Win32 API that the message has been processed.
extern "stdcall" fn callback(window: ffi::HWND, msg: ffi::UINT,
    wparam: ffi::WPARAM, lparam: ffi::LPARAM) -> ffi::LRESULT
{
    match msg {
        ffi::WM_DESTROY => {
            use Closed;

            match WINDOW.get() {
                None => (),
                Some(v) => {
                    let &(ref win, _) = v.deref();

                    if win == &window {
                        unsafe { ffi::PostQuitMessage(0); }
                    }
                }
            };

            send_event(window, Closed);
            0
        },

        ffi::WM_ERASEBKGND => {
            1
        },

        ffi::WM_SIZE => {
            use Resized;
            let w = ffi::LOWORD(lparam as ffi::DWORD) as uint;
            let h = ffi::HIWORD(lparam as ffi::DWORD) as uint;
            send_event(window, Resized(w, h));
            0
        },

        ffi::WM_MOVE => {
            use events::Moved;
            let x = ffi::LOWORD(lparam as ffi::DWORD) as i16 as int;
            let y = ffi::HIWORD(lparam as ffi::DWORD) as i16 as int;
            send_event(window, Moved(x, y));
            0
        },

        ffi::WM_CHAR => {
            use std::mem;
            use events::ReceivedCharacter;
            let chr: char = unsafe { mem::transmute(wparam as u32) };
            send_event(window, ReceivedCharacter(chr));
            0
        },

        ffi::WM_MOUSEMOVE => {
            use MouseMoved;

            let x = ffi::GET_X_LPARAM(lparam) as int;
            let y = ffi::GET_Y_LPARAM(lparam) as int;

            send_event(window, MouseMoved((x, y)));

            0
        },

        ffi::WM_MOUSEWHEEL => {
            use events::{KeyModifiers, MouseWheel};

            let value = (wparam >> 16) as i16;
            let value = value as i32;

            send_event(window, MouseWheel(value, KeyModifiers::empty()));

            0
        },

        ffi::WM_KEYDOWN => {
            use events::{KeyboardInput, Pressed};
            let scancode = ((lparam >> 16) & 0xff) as u8;
            let vkey = event::vkeycode_to_element(wparam);
            send_event(window, KeyboardInput(Pressed, scancode, vkey));
            0
        },

        ffi::WM_KEYUP => {
            use events::{KeyboardInput, Released};
            let scancode = ((lparam >> 16) & 0xff) as u8;
            let vkey = event::vkeycode_to_element(wparam);
            send_event(window, KeyboardInput(Released, scancode, vkey));
            0
        },

        ffi::WM_LBUTTONDOWN => {
            use events::{Pressed, MouseInput, LeftMouseButton};
            send_event(window, MouseInput(Pressed, LeftMouseButton));
            0
        },

        ffi::WM_LBUTTONUP => {
            use events::{Released, MouseInput, LeftMouseButton};
            send_event(window, MouseInput(Released, LeftMouseButton));
            0
        },

        ffi::WM_RBUTTONDOWN => {
            use events::{Pressed, MouseInput, RightMouseButton};
            send_event(window, MouseInput(Pressed, RightMouseButton));
            0
        },

        ffi::WM_RBUTTONUP => {
            use events::{Released, MouseInput, RightMouseButton};
            send_event(window, MouseInput(Released, RightMouseButton));
            0
        },

        ffi::WM_MBUTTONDOWN => {
            use events::{Pressed, MouseInput, MiddleMouseButton};
            send_event(window, MouseInput(Pressed, MiddleMouseButton));
            0
        },

        ffi::WM_MBUTTONUP => {
            use events::{Released, MouseInput, MiddleMouseButton};
            send_event(window, MouseInput(Released, MiddleMouseButton));
            0
        },

        ffi::WM_SETFOCUS => {
            use events::Focused;
            send_event(window, Focused(true));
            0
        },

        ffi::WM_KILLFOCUS => {
            use events::Focused;
            send_event(window, Focused(false));
            0
        },

        _ => unsafe {
            ffi::DefWindowProcW(window, msg, wparam, lparam)
        }
    }
}

/*fn hints_to_pixelformat(hints: &Hints) -> ffi::PIXELFORMATDESCRIPTOR {
    use std::mem;

    ffi::PIXELFORMATDESCRIPTOR {
        nSize: size_of::<ffi::PIXELFORMATDESCRIPTOR>(),
        nVersion: 1,
        dwFlags:
            if hints.stereo { PFD_STEREO } else { 0 },
        iPixelType: PFD_TYPE_RGBA,
        cColorBits: hints.red_bits + hints.green_bits + hints.blue_bits,
        cRedBits: 

    pub nSize: WORD,
    pub nVersion: WORD,
    pub dwFlags: DWORD,
    pub iPixelType: BYTE,
    pub cColorBits: BYTE,
    pub cRedBits: BYTE,
    pub cRedShift: BYTE,
    pub cGreenBits: BYTE,
    pub cGreenShift: BYTE,
    pub cBlueBits: BYTE,
    pub cBlueShift: BYTE,
    pub cAlphaBits: BYTE,
    pub cAlphaShift: BYTE,
    pub cAccumBits: BYTE,
    pub cAccumRedBits: BYTE,
    pub cAccumGreenBits: BYTE,
    pub cAccumBlueBits: BYTE,
    pub cAccumAlphaBits: BYTE,
    pub cDepthBits: BYTE,
    pub cStencilBits: BYTE,
    pub cAuxBuffers: BYTE,
    pub iLayerType: BYTE,
    pub bReserved: BYTE,
    pub dwLayerMask: DWORD,
    pub dwVisibleMask: DWORD,
    pub dwDamageMask: DWORD,
    }
}*/
