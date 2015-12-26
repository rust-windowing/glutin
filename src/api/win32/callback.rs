use std::mem;
use std::ptr;
use std::cell::RefCell;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use WindowAttributes;
use CursorState;
use Event;
use super::event;
use super::WindowState;

use user32;
use shell32;
use winapi;

/// There's no parameters passed to the callback function, so it needs to get
/// its context (the HWND, the Sender for events, etc.) stashed in
/// a thread-local variable.
thread_local!(pub static CONTEXT_STASH: RefCell<Option<ThreadLocalData>> = RefCell::new(None));

pub struct ThreadLocalData {
    pub win: winapi::HWND,
    pub sender: Sender<Event>,
    pub window_state: Arc<Mutex<WindowState>>
}

struct MinMaxInfo {
    reserved: winapi::POINT, // Do not use/change
    max_size: winapi::POINT,
    max_position: winapi::POINT,
    min_track: winapi::POINT,
    max_track: winapi::POINT
}

/// Checks that the window is the good one, and if so send the event to it.
fn send_event(input_window: winapi::HWND, event: Event) {
    CONTEXT_STASH.with(|context_stash| {
        let context_stash = context_stash.borrow();
        let stored = match *context_stash {
            None => return,
            Some(ref v) => v
        };

        let &ThreadLocalData { ref win, ref sender, .. } = stored;

        if win != &input_window {
            return;
        }

        sender.send(event).ok();  // ignoring if closed
    });
}

/// This is the callback that is called by `DispatchMessage` in the events loop.
///
/// Returning 0 tells the Win32 API that the message has been processed.
// FIXME: detect WM_DWMCOMPOSITIONCHANGED and call DwmEnableBlurBehindWindow if necessary
pub unsafe extern "system" fn callback(window: winapi::HWND, msg: winapi::UINT,
                                       wparam: winapi::WPARAM, lparam: winapi::LPARAM)
                                       -> winapi::LRESULT
{
    match msg {
        winapi::WM_DESTROY => {
            use events::Event::Closed;

            CONTEXT_STASH.with(|context_stash| {
                let context_stash = context_stash.borrow();
                let stored = match *context_stash {
                    None => return,
                    Some(ref v) => v
                };

                let &ThreadLocalData { ref win, .. } = stored;

                if win == &window {
                    user32::PostQuitMessage(0);
                }
            });

            send_event(window, Closed);
            0
        },

        winapi::WM_ERASEBKGND => {
            1
        },

        winapi::WM_SIZE => {
            use events::Event::Resized;
            let w = winapi::LOWORD(lparam as winapi::DWORD) as u32;
            let h = winapi::HIWORD(lparam as winapi::DWORD) as u32;
            send_event(window, Resized(w, h));
            0
        },

        winapi::WM_MOVE => {
            use events::Event::Moved;
            let x = winapi::LOWORD(lparam as winapi::DWORD) as i32;
            let y = winapi::HIWORD(lparam as winapi::DWORD) as i32;
            send_event(window, Moved(x, y));
            0
        },

        winapi::WM_CHAR => {
            use std::mem;
            use events::Event::ReceivedCharacter;
            let chr: char = mem::transmute(wparam as u32);
            send_event(window, ReceivedCharacter(chr));
            0
        },

        // Prevents default windows menu hotkeys playing unwanted
        // "ding" sounds. Alternatively could check for WM_SYSCOMMAND
        // with wparam being SC_KEYMENU, but this may prevent some
        // other unwanted default hotkeys as well.
        winapi::WM_SYSCHAR => {
            0
        }

        winapi::WM_MOUSEMOVE => {
            use events::Event::MouseMoved;

            let x = winapi::GET_X_LPARAM(lparam) as i32;
            let y = winapi::GET_Y_LPARAM(lparam) as i32;

            send_event(window, MouseMoved((x, y)));

            0
        },

        winapi::WM_MOUSEWHEEL => {
            use events::Event::MouseWheel;
            use events::MouseScrollDelta::LineDelta;

            let value = (wparam >> 16) as i16;
            let value = value as i32;
            let value = value as f32 / winapi::WHEEL_DELTA as f32;

            send_event(window, MouseWheel(LineDelta(0.0, value)));

            0
        },

        winapi::WM_KEYDOWN | winapi::WM_SYSKEYDOWN => {
            use events::Event::KeyboardInput;
            use events::ElementState::Pressed;
            if msg == winapi::WM_SYSKEYDOWN && wparam as i32 == winapi::VK_F4 {
                user32::DefWindowProcW(window, msg, wparam, lparam)
            } else {
                let (scancode, vkey) = event::vkeycode_to_element(wparam, lparam);
                send_event(window, KeyboardInput(Pressed, scancode, vkey));
                0
            }
        },

        winapi::WM_KEYUP | winapi::WM_SYSKEYUP => {
            use events::Event::KeyboardInput;
            use events::ElementState::Released;
            let (scancode, vkey) = event::vkeycode_to_element(wparam, lparam);
            send_event(window, KeyboardInput(Released, scancode, vkey));
            0
        },

        winapi::WM_LBUTTONDOWN => {
            use events::Event::MouseInput;
            use events::MouseButton::Left;
            use events::ElementState::Pressed;
            send_event(window, MouseInput(Pressed, Left));
            0
        },

        winapi::WM_LBUTTONUP => {
            use events::Event::MouseInput;
            use events::MouseButton::Left;
            use events::ElementState::Released;
            send_event(window, MouseInput(Released, Left));
            0
        },

        winapi::WM_RBUTTONDOWN => {
            use events::Event::MouseInput;
            use events::MouseButton::Right;
            use events::ElementState::Pressed;
            send_event(window, MouseInput(Pressed, Right));
            0
        },

        winapi::WM_RBUTTONUP => {
            use events::Event::MouseInput;
            use events::MouseButton::Right;
            use events::ElementState::Released;
            send_event(window, MouseInput(Released, Right));
            0
        },

        winapi::WM_MBUTTONDOWN => {
            use events::Event::MouseInput;
            use events::MouseButton::Middle;
            use events::ElementState::Pressed;
            send_event(window, MouseInput(Pressed, Middle));
            0
        },

        winapi::WM_MBUTTONUP => {
            use events::Event::MouseInput;
            use events::MouseButton::Middle;
            use events::ElementState::Released;
            send_event(window, MouseInput(Released, Middle));
            0
        },

        winapi::WM_INPUT => {
            let mut data: winapi::RAWINPUT = mem::uninitialized();
            let mut data_size = mem::size_of::<winapi::RAWINPUT>() as winapi::UINT;
            user32::GetRawInputData(mem::transmute(lparam), winapi::RID_INPUT,
                                    mem::transmute(&mut data), &mut data_size,
                                    mem::size_of::<winapi::RAWINPUTHEADER>() as winapi::UINT);

            if data.header.dwType == winapi::RIM_TYPEMOUSE {
                let _x = data.mouse.lLastX;  // FIXME: this is not always the relative movement
                let _y = data.mouse.lLastY;
                // TODO:
                //send_event(window, Event::MouseRawMovement { x: x, y: y });

                0

            } else {
                user32::DefWindowProcW(window, msg, wparam, lparam)
            }
        },

        winapi::WM_SETFOCUS => {
            use events::Event::Focused;
            send_event(window, Focused(true));
            0
        },

        winapi::WM_KILLFOCUS => {
            use events::Event::Focused;
            send_event(window, Focused(false));
            0
        },

        winapi::WM_SETCURSOR => {
            CONTEXT_STASH.with(|context_stash| {
                let cstash = context_stash.borrow();
                let cstash = cstash.as_ref();
                // there's a very bizarre borrow checker bug
                // possibly related to rust-lang/rust/#23338
                let _cursor_state = if let Some(cstash) = cstash {
                    if let Ok(window_state) = cstash.window_state.lock() {
                        match window_state.cursor_state {
                            CursorState::Normal => {
                                user32::SetCursor(user32::LoadCursorW(
                                        ptr::null_mut(),
                                        winapi::IDC_ARROW));
                            },
                            CursorState::Grab | CursorState::Hide => {
                                user32::SetCursor(ptr::null_mut());
                            }
                        }
                    }
                } else {
                    return
                };

//                let &ThreadLocalData { ref cursor_state, .. } = stored;
            });
            0
        },

        winapi::WM_DROPFILES => {
            use events::Event::DroppedFile;

            let hdrop = wparam as winapi::HDROP;
            let mut pathbuf: [u16; winapi::MAX_PATH] = mem::uninitialized();
            let num_drops = shell32::DragQueryFileW(hdrop, 0xFFFFFFFF, ptr::null_mut(), 0);

            for i in 0..num_drops {
                let nch = shell32::DragQueryFileW(hdrop, i, pathbuf.as_mut_ptr(),
                                                  winapi::MAX_PATH as u32) as usize;
                if nch > 0 {
                    send_event(window, DroppedFile(OsString::from_wide(&pathbuf[0..nch]).into()));
                }
            }

            shell32::DragFinish(hdrop);
            0
        },

        winapi::WM_GETMINMAXINFO => {
            let mut mmi = lparam as *mut MinMaxInfo;
            //(*mmi).max_position = winapi::POINT { x: -8, y: -8 }; // The upper left corner of the window if it were maximized on the primary monitor.
            //(*mmi).max_size = winapi::POINT { x: .., y: .. }; // The dimensions of the primary monitor.

            CONTEXT_STASH.with(|context_stash| {
                match context_stash.borrow().as_ref() {
                    Some(cstash) => {
                        let window_state = cstash.window_state.lock().unwrap();

                        match window_state.attributes.min_dimensions {
                            Some((width, height)) => {
                                let mut rc_client: winapi::RECT = mem::uninitialized();
                                let mut rc_wind: winapi::RECT = mem::uninitialized();
                                user32::GetClientRect(window, &mut rc_client);
                                user32::GetWindowRect(window, &mut rc_wind);

                                let border_width : i32 = (rc_wind.right - rc_wind.left) - 
                                    (rc_client.right - rc_client.left);
                                let border_height : i32 = (rc_wind.bottom - rc_wind.top) - 
                                    (rc_client.bottom - rc_client.top);

                                (*mmi).min_track = winapi::POINT { 
                                    x: width as i32 + border_width, 
                                    y: height as i32 + border_height  
                                };
                            },
                            None => { }
                        }

                        match window_state.attributes.max_dimensions {
                            Some((width, height)) => {
                                let mut rc_client: winapi::RECT = mem::uninitialized();
                                let mut rc_wind: winapi::RECT = mem::uninitialized();
                                user32::GetClientRect(window, &mut rc_client);
                                user32::GetWindowRect(window, &mut rc_wind);

                                let border_width : i32 = (rc_wind.right - rc_wind.left) - 
                                    (rc_client.right - rc_client.left);
                                let border_height : i32 = (rc_wind.bottom - rc_wind.top) - 
                                    (rc_client.bottom - rc_client.top);

                                (*mmi).max_track = winapi::POINT { 
                                    x: width as i32 + border_width , 
                                    y: height as i32 + border_height};
                            },
                            None => { }
                        }
                    },
                    None => { }
                }
            });
            0
        },

        x if x == *super::WAKEUP_MSG_ID => {
            use events::Event::Awakened;
            send_event(window, Awakened);
            0
        },

        _ => {
            user32::DefWindowProcW(window, msg, wparam, lparam)
        }
    }
}
