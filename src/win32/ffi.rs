#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use libc;

/// WGL bindings
pub mod wgl {
    generate_gl_bindings! {
        api: "wgl",
        profile: "core",
        version: "1.0",
        generator: "static"
    }
}

/// Functions that are not necessarly always available
pub mod wgl_extra {
    generate_gl_bindings! {
        api: "wgl",
        profile: "core",
        version: "1.0",
        generator: "struct",
        extensions: [
            "WGL_ARB_create_context",
            "WGL_EXT_swap_control"
        ]
    }
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/aa383751(v=vs.85).aspx
// we don't define the T types to ensure that A/W functions are used
pub type ATOM = WORD;
pub type BOOL = libc::c_int;
pub type BOOLEAN = BYTE;
pub type BYTE = libc::c_uchar;
pub type DWORD = libc::c_ulong;
pub type HANDLE = PVOID;
pub type HBRUSH = HANDLE;
pub type HCURSOR = HICON;
pub type HDC = HANDLE;
pub type HICON = HANDLE;
pub type HINSTANCE = HANDLE;
pub type HLOCAL = HANDLE;
pub type HMENU = HANDLE;
pub type HMODULE = HINSTANCE;
pub type HWND = HANDLE;
pub type LONG = libc::c_long;
pub type LONG_PTR = libc::intptr_t;
pub type LPARAM = LONG_PTR;
pub type LPCSTR = *const libc::c_char;
pub type LPCWSTR = *const WCHAR;
pub type LPCVOID = *const libc::c_void;
pub type LPSTR = *mut libc::c_char;
pub type LPVOID = *mut libc::c_void;
pub type LPWSTR = *mut WCHAR;
pub type LRESULT = LONG_PTR;
pub type PVOID = *const libc::c_void;
pub type UINT = libc::c_uint;
pub type UINT_PTR = libc::intptr_t;
pub type WCHAR = libc::wchar_t;
pub type WORD = libc::c_ushort;
pub type WPARAM = UINT_PTR;

// macros
pub fn LOWORD(l: DWORD) -> WORD {
    (l & 0xFFFF) as WORD
}

pub fn HIWORD(l: DWORD) -> WORD {
    (l >> 16) as WORD
}

pub fn GET_X_LPARAM(lp: LONG_PTR) -> libc::c_int {
    LOWORD(lp as DWORD) as libc::c_int
}

pub fn GET_Y_LPARAM(lp: LONG_PTR) -> libc::c_int {
    HIWORD(lp as DWORD) as libc::c_int
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/ff485887(v=vs.85).aspx
pub const BN_CLICKED: WORD = 0;
pub const BN_DBLCLK: WORD = 5;
pub const BN_DISABLE: WORD = 4;
pub const BN_DOUBLECLICKED: WORD = 5;
pub const BN_HILITE: WORD = 2;
pub const BN_KILLFOCUS: WORD = 7;
pub const BN_PAINT: WORD = 1;
pub const BN_PUSHED: WORD = 2;
pub const BN_SETFOCUS: WORD = 6;
pub const BN_UNHILITE: WORD = 3;
pub const BN_UNPUSHED: WORD = 3;

// ?
pub const BS_3STATE: DWORD = 5;
pub const BS_AUTO3STATE: DWORD = 6;
pub const BS_AUTOCHECKBOX: DWORD = 3;
pub const BS_AUTORADIOBUTTON: DWORD =  9;
pub const BS_BITMAP: DWORD = 128;
pub const BS_BOTTOM: DWORD = 0x800;
pub const BS_CENTER: DWORD = 0x300;
pub const BS_CHECKBOX: DWORD = 2;
pub const BS_DEFPUSHBUTTON: DWORD = 1;
pub const BS_GROUPBOX: DWORD = 7;
pub const BS_ICON: DWORD = 64;
pub const BS_LEFT: DWORD = 256;
pub const BS_LEFTTEXT: DWORD = 32;
pub const BS_MULTILINE: DWORD = 0x2000;
pub const BS_NOTIFY: DWORD = 0x4000;
pub const BS_OWNERDRAW: DWORD = 0xb;
pub const BS_PUSHBUTTON: DWORD = 0;
pub const BS_PUSHLIKE: DWORD = 4096;
pub const BS_RADIOBUTTON: DWORD = 4;
pub const BS_RIGHT: DWORD = 512;
pub const BS_RIGHTBUTTON: DWORD = 32;
pub const BS_TEXT: DWORD = 0;
pub const BS_TOP: DWORD = 0x400;
pub const BS_USERBUTTON: DWORD = 8;
pub const BS_VCENTER: DWORD =  0xc00;
pub const BS_FLAT: DWORD = 0x8000;

// ?
pub const CDS_UPDATEREGISTRY: DWORD = 0x1;
pub const CDS_TEST: DWORD = 0x2;
pub const CDS_FULLSCREEN: DWORD = 0x4;
pub const CDS_GLOBAL: DWORD = 0x8;
pub const CDS_SET_PRIMARY: DWORD = 0x10;
pub const CDS_VIDEOPARAMETERS: DWORD = 0x20;
pub const CDS_NORESET: DWORD = 0x10000000;
pub const CDS_SETRECT: DWORD = 0x20000000;
pub const CDS_RESET: DWORD = 0x40000000;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ff729176(v=vs.85).aspx
pub const CS_BYTEALIGNCLIENT: DWORD = 0x1000;
pub const CS_BYTEALIGNWINDOW: DWORD = 0x2000;
pub const CS_CLASSDC: DWORD = 0x0040;
pub const CS_DBLCLKS: DWORD = 0x0008;
pub const CS_DROPSHADOW: DWORD = 0x00020000;
pub const CS_GLOBALCLASS: DWORD = 0x4000;
pub const CS_HREDRAW: DWORD = 0x0002;
pub const CS_NOCLOSE: DWORD = 0x0200;
pub const CS_OWNDC: DWORD = 0x0020;
pub const CS_PARENTDC: DWORD = 0x0080;
pub const CS_SAVEBITS: DWORD = 0x0800;
pub const CS_VREDRAW: DWORD = 0x0001;

// ?
#[allow(overflowing_literals)]
pub const CW_USEDEFAULT: libc::c_int = 0x80000000;

// ?
pub const DISP_CHANGE_SUCCESSFUL: LONG = 0;
pub const DISP_CHANGE_RESTART: LONG = 1;
pub const DISP_CHANGE_FAILED: LONG = -1;
pub const DISP_CHANGE_BADMODE: LONG = -2;
pub const DISP_CHANGE_NOTUPDATED: LONG = -3;
pub const DISP_CHANGE_BADFLAGS: LONG = -4;
pub const DISP_CHANGE_BADPARAM: LONG = -5;
pub const DISP_CHANGE_BADDUALVIEW: LONG = -6;

// ?
pub const DISPLAY_DEVICE_ACTIVE: DWORD = 0x00000001;
pub const DISPLAY_DEVICE_MULTI_DRIVER: DWORD = 0x00000002;
pub const DISPLAY_DEVICE_PRIMARY_DEVICE: DWORD = 0x00000004;
pub const DISPLAY_DEVICE_MIRRORING_DRIVER: DWORD = 0x00000008;
pub const DISPLAY_DEVICE_VGA_COMPATIBLE: DWORD = 0x00000010;

// ?
pub const DM_ORIENTATION: DWORD = 0x00000001;
pub const DM_PAPERSIZE: DWORD = 0x00000002;
pub const DM_PAPERLENGTH: DWORD = 0x00000004;
pub const DM_PAPERWIDTH: DWORD = 0x00000008;
pub const DM_SCALE: DWORD = 0x00000010;
pub const DM_POSITION: DWORD = 0x00000020;
pub const DM_NUP: DWORD = 0x00000040;
pub const DM_DISPLAYORIENTATION: DWORD = 0x00000080;
pub const DM_COPIES: DWORD = 0x00000100;
pub const DM_DEFAULTSOURCE: DWORD = 0x00000200;
pub const DM_PRINTQUALITY: DWORD = 0x00000400;
pub const DM_COLOR: DWORD = 0x00000800;
pub const DM_DUPLEX: DWORD = 0x00001000;
pub const DM_YRESOLUTION: DWORD = 0x00002000;
pub const DM_TTOPTION: DWORD = 0x00004000;
pub const DM_COLLATE: DWORD = 0x00008000;
pub const DM_FORMNAME: DWORD = 0x00010000;
pub const DM_LOGPIXELS: DWORD = 0x00020000;
pub const DM_BITSPERPEL: DWORD = 0x00040000;
pub const DM_PELSWIDTH: DWORD = 0x00080000;
pub const DM_PELSHEIGHT: DWORD = 0x00100000;
pub const DM_DISPLAYFLAGS: DWORD = 0x00200000;
pub const DM_DISPLAYFREQUENCY: DWORD = 0x00400000;
pub const DM_ICMMETHOD: DWORD = 0x00800000;
pub const DM_ICMINTENT: DWORD = 0x01000000;
pub const DM_MEDIATYPE: DWORD = 0x02000000;
pub const DM_DITHERTYPE: DWORD = 0x04000000;
pub const DM_PANNINGWIDTH: DWORD = 0x08000000;
pub const DM_PANNINGHEIGHT: DWORD = 0x10000000;
pub const DM_DISPLAYFIXEDOUTPUT: DWORD = 0x20000000;

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd162609(v=vs.85).aspx
pub const EDD_GET_DEVICE_INTERFACE_NAME: DWORD = 0x00000001;

// ?
pub const ENUM_CURRENT_SETTINGS: DWORD = -1;
pub const ENUM_REGISTRY_SETTINGS: DWORD = -2;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms679351(v=vs.85).aspx
pub const FORMAT_MESSAGE_ALLOCATE_BUFFER: DWORD = 0x00000100;
pub const FORMAT_MESSAGE_ARGUMENT_ARRAY: DWORD = 0x00002000;
pub const FORMAT_MESSAGE_FROM_HMODULE: DWORD = 0x00000800;
pub const FORMAT_MESSAGE_FROM_STRING: DWORD = 0x00000400;
pub const FORMAT_MESSAGE_FROM_SYSTEM: DWORD = 0x00001000;
pub const FORMAT_MESSAGE_IGNORE_INSERTS: DWORD = 0x00000200;

// ?
pub const PFD_TYPE_RGBA: BYTE = 0;
pub const PFD_TYPE_COLORINDEX: BYTE = 1;
pub const PFD_MAIN_PLANE: BYTE = 0;
pub const PFD_OVERLAY_PLANE: BYTE = 1;
pub const PFD_UNDERLAY_PLANE: BYTE = (-1);
pub const PFD_DOUBLEBUFFER: DWORD = 0x00000001;
pub const PFD_STEREO: DWORD = 0x00000002;
pub const PFD_DRAW_TO_WINDOW: DWORD = 0x00000004;
pub const PFD_DRAW_TO_BITMAP: DWORD = 0x00000008;
pub const PFD_SUPPORT_GDI: DWORD = 0x00000010;
pub const PFD_SUPPORT_OPENGL: DWORD = 0x00000020;
pub const PFD_GENERIC_FORMAT: DWORD = 0x00000040;
pub const PFD_NEED_PALETTE: DWORD = 0x00000080;
pub const PFD_NEED_SYSTEM_PALETTE: DWORD = 0x00000100;
pub const PFD_SWAP_EXCHANGE: DWORD = 0x00000200;
pub const PFD_SWAP_COPY: DWORD = 0x00000400;
pub const PFD_SWAP_LAYER_BUFFERS: DWORD = 0x00000800;
pub const PFD_GENERIC_ACCELERATED: DWORD = 0x00001000;
pub const PFD_SUPPORT_COMPOSITION: DWORD = 0x00008000;
pub const PFD_DEPTH_DONTCARE: DWORD = 0x20000000;
pub const PFD_DOUBLEBUFFER_DONTCARE: DWORD = 0x40000000;
pub const PFD_STEREO_DONTCARE: DWORD = 0x80000000;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms633548(v=vs.85).aspx
pub const SW_FORCEMINIMIZE: libc::c_int = 11;
pub const SW_HIDE: libc::c_int = 0;
pub const SW_MAXIMIZE: libc::c_int = 3;
pub const SW_MINIMIZE: libc::c_int = 6;
pub const SW_RESTORE: libc::c_int = 9;
pub const SW_SHOW: libc::c_int = 5;
pub const SW_SHOWDEFAULT: libc::c_int = 10;
pub const SW_SHOWMAXIMIZED: libc::c_int = 3;
pub const SW_SHOWMINIMIZED: libc::c_int = 2;
pub const SW_SHOWMINNOACTIVE: libc::c_int = 7;
pub const SW_SHOWNA: libc::c_int = 8;
pub const SW_SHOWNOACTIVATE: libc::c_int = 4;
pub const SW_SHOWNORMAL: libc::c_int = 1;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms633545(v=vs.85).aspx
pub const SWP_ASYNCWINDOWPOS: UINT = 0x4000;
pub const SWP_DEFERERASE: UINT = 0x2000;
pub const SWP_DRAWFRAME: UINT = 0x0020;
pub const SWP_FRAMECHANGED: UINT = 0x0020;
pub const SWP_HIDEWINDOW: UINT = 0x0080;
pub const SWP_NOACTIVATE: UINT = 0x0010;
pub const SWP_NOCOPYBITS: UINT = 0x0100;
pub const SWP_NOMOVE: UINT = 0x0002;
pub const SWP_NOOWNERZORDER: UINT = 0x0200;
pub const SWP_NOREDRAW: UINT = 0x0008;
pub const SWP_NOREPOSITION: UINT = 0x0200;
pub const SWP_NOSENDCHANGING: UINT = 0x0400;
pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOZORDER: UINT = 0x0004;
pub const SWP_SHOWWINDOW: UINT = 0x0040;

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd375731(v=vs.85).aspx
pub const VK_LBUTTON: WPARAM = 0x01;
pub const VK_RBUTTON: WPARAM = 0x02;
pub const VK_CANCEL: WPARAM = 0x03;
pub const VK_MBUTTON: WPARAM = 0x04;
pub const VK_XBUTTON1: WPARAM = 0x05;
pub const VK_XBUTTON2: WPARAM = 0x06;
pub const VK_BACK: WPARAM = 0x08;
pub const VK_TAB: WPARAM = 0x09;
pub const VK_CLEAR: WPARAM = 0x0C;
pub const VK_RETURN: WPARAM = 0x0D;
pub const VK_SHIFT: WPARAM = 0x10;
pub const VK_CONTROL: WPARAM = 0x11;
pub const VK_MENU: WPARAM = 0x12;
pub const VK_PAUSE: WPARAM = 0x13;
pub const VK_CAPITAL: WPARAM = 0x14;
pub const VK_KANA: WPARAM = 0x15;
pub const VK_HANGUEL: WPARAM = 0x15;
pub const VK_HANGUL: WPARAM = 0x15;
pub const VK_JUNJA: WPARAM = 0x17;
pub const VK_FINAL: WPARAM = 0x18;
pub const VK_HANJA: WPARAM = 0x19;
pub const VK_KANJI: WPARAM = 0x19;
pub const VK_ESCAPE: WPARAM = 0x1B;
pub const VK_CONVERT: WPARAM = 0x1C;
pub const VK_NONCONVERT: WPARAM = 0x1D;
pub const VK_ACCEPT: WPARAM = 0x1E;
pub const VK_MODECHANGE: WPARAM = 0x1F;
pub const VK_SPACE: WPARAM = 0x20;
pub const VK_PRIOR: WPARAM = 0x21;
pub const VK_NEXT: WPARAM = 0x22;
pub const VK_END: WPARAM = 0x23;
pub const VK_HOME: WPARAM = 0x24;
pub const VK_LEFT: WPARAM = 0x25;
pub const VK_UP: WPARAM = 0x26;
pub const VK_RIGHT: WPARAM = 0x27;
pub const VK_DOWN: WPARAM = 0x28;
pub const VK_SELECT: WPARAM = 0x29;
pub const VK_PRINT: WPARAM = 0x2A;
pub const VK_EXECUTE: WPARAM = 0x2B;
pub const VK_SNAPSHOT: WPARAM = 0x2C;
pub const VK_INSERT: WPARAM = 0x2D;
pub const VK_DELETE: WPARAM = 0x2E;
pub const VK_HELP: WPARAM = 0x2F;
pub const VK_LWIN: WPARAM = 0x5B;
pub const VK_RWIN: WPARAM = 0x5C;
pub const VK_APPS: WPARAM = 0x5D;
pub const VK_SLEEP: WPARAM = 0x5F;
pub const VK_NUMPAD0: WPARAM = 0x60;
pub const VK_NUMPAD1: WPARAM = 0x61;
pub const VK_NUMPAD2: WPARAM = 0x62;
pub const VK_NUMPAD3: WPARAM = 0x63;
pub const VK_NUMPAD4: WPARAM = 0x64;
pub const VK_NUMPAD5: WPARAM = 0x65;
pub const VK_NUMPAD6: WPARAM = 0x66;
pub const VK_NUMPAD7: WPARAM = 0x67;
pub const VK_NUMPAD8: WPARAM = 0x68;
pub const VK_NUMPAD9: WPARAM = 0x69;
pub const VK_MULTIPLY: WPARAM = 0x6A;
pub const VK_ADD: WPARAM = 0x6B;
pub const VK_SEPARATOR: WPARAM = 0x6C;
pub const VK_SUBTRACT: WPARAM = 0x6D;
pub const VK_DECIMAL: WPARAM = 0x6E;
pub const VK_DIVIDE: WPARAM = 0x6F;
pub const VK_F1: WPARAM = 0x70;
pub const VK_F2: WPARAM = 0x71;
pub const VK_F3: WPARAM = 0x72;
pub const VK_F4: WPARAM = 0x73;
pub const VK_F5: WPARAM = 0x74;
pub const VK_F6: WPARAM = 0x75;
pub const VK_F7: WPARAM = 0x76;
pub const VK_F8: WPARAM = 0x77;
pub const VK_F9: WPARAM = 0x78;
pub const VK_F10: WPARAM = 0x79;
pub const VK_F11: WPARAM = 0x7A;
pub const VK_F12: WPARAM = 0x7B;
pub const VK_F13: WPARAM = 0x7C;
pub const VK_F14: WPARAM = 0x7D;
pub const VK_F15: WPARAM = 0x7E;
pub const VK_F16: WPARAM = 0x7F;
pub const VK_F17: WPARAM = 0x80;
pub const VK_F18: WPARAM = 0x81;
pub const VK_F19: WPARAM = 0x82;
pub const VK_F20: WPARAM = 0x83;
pub const VK_F21: WPARAM = 0x84;
pub const VK_F22: WPARAM = 0x85;
pub const VK_F23: WPARAM = 0x86;
pub const VK_F24: WPARAM = 0x87;
pub const VK_NUMLOCK: WPARAM = 0x90;
pub const VK_SCROLL: WPARAM = 0x91;
pub const VK_LSHIFT: WPARAM = 0xA0;
pub const VK_RSHIFT: WPARAM = 0xA1;
pub const VK_LCONTROL: WPARAM = 0xA2;
pub const VK_RCONTROL: WPARAM = 0xA3;
pub const VK_LMENU: WPARAM = 0xA4;
pub const VK_RMENU: WPARAM = 0xA5;
pub const VK_BROWSER_BACK: WPARAM = 0xA6;
pub const VK_BROWSER_FORWARD: WPARAM = 0xA7;
pub const VK_BROWSER_REFRESH: WPARAM = 0xA8;
pub const VK_BROWSER_STOP: WPARAM = 0xA9;
pub const VK_BROWSER_SEARCH: WPARAM = 0xAA;
pub const VK_BROWSER_FAVORITES: WPARAM = 0xAB;
pub const VK_BROWSER_HOME: WPARAM = 0xAC;
pub const VK_VOLUME_MUTE: WPARAM = 0xAD;
pub const VK_VOLUME_DOWN: WPARAM = 0xAE;
pub const VK_VOLUME_UP: WPARAM = 0xAF;
pub const VK_MEDIA_NEXT_TRACK: WPARAM = 0xB0;
pub const VK_MEDIA_PREV_TRACK: WPARAM = 0xB1;
pub const VK_MEDIA_STOP: WPARAM = 0xB2;
pub const VK_MEDIA_PLAY_PAUSE: WPARAM = 0xB3;
pub const VK_LAUNCH_MAIL: WPARAM = 0xB4;
pub const VK_LAUNCH_MEDIA_SELECT: WPARAM = 0xB5;
pub const VK_LAUNCH_APP1: WPARAM = 0xB6;
pub const VK_LAUNCH_APP2: WPARAM = 0xB7;
pub const VK_OEM_1: WPARAM = 0xBA;
pub const VK_OEM_PLUS: WPARAM = 0xBB;
pub const VK_OEM_COMMA: WPARAM = 0xBC;
pub const VK_OEM_MINUS: WPARAM = 0xBD;
pub const VK_OEM_PERIOD: WPARAM = 0xBE;
pub const VK_OEM_2: WPARAM = 0xBF;
pub const VK_OEM_3: WPARAM = 0xC0;
pub const VK_OEM_4: WPARAM = 0xDB;
pub const VK_OEM_5: WPARAM = 0xDC;
pub const VK_OEM_6: WPARAM = 0xDD;
pub const VK_OEM_7: WPARAM = 0xDE;
pub const VK_OEM_8: WPARAM = 0xDF;
pub const VK_OEM_102: WPARAM = 0xE2;
pub const VK_PROCESSKEY: WPARAM = 0xE5;
pub const VK_PACKET: WPARAM = 0xE7;
pub const VK_ATTN: WPARAM = 0xF6;
pub const VK_CRSEL: WPARAM = 0xF7;
pub const VK_EXSEL: WPARAM = 0xF8;
pub const VK_EREOF: WPARAM = 0xF9;
pub const VK_PLAY: WPARAM = 0xFA;
pub const VK_ZOOM: WPARAM = 0xFB;
pub const VK_NONAME: WPARAM = 0xFC;
pub const VK_PA1: WPARAM = 0xFD;
pub const VK_OEM_CLEAR: WPARAM = 0xFE;

// messages
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_CHAR: UINT = 0x0102;
pub const WM_COMMAND: UINT = 0x0111;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_KEYUP: UINT = 0x0101;
pub const WM_KILLFOCUS: UINT = 0x0008;
pub const WM_MBUTTONDOWN: UINT = 0x0207;
pub const WM_MBUTTONUP: UINT = 0x0208;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_MOVE: UINT = 0x0003;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_RBUTTONDOWN: UINT = 0x0204;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_SETFOCUS: UINT = 0x0007;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_SIZING: UINT = 0x0214;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms632600(v=vs.85).aspx
pub const WS_BORDER: DWORD = 0x00800000;
pub const WS_CAPTION: DWORD = 0x00C00000;
pub const WS_CHILD: DWORD = 0x40000000;
pub const WS_CHILDWINDOW: DWORD = 0x40000000;
pub const WS_CLIPCHILDREN: DWORD = 0x02000000;
pub const WS_CLIPSIBLINGS: DWORD = 0x04000000;
pub const WS_DISABLED: DWORD = 0x08000000;
pub const WS_DLGFRAME: DWORD = 0x00400000;
pub const WS_GROUP: DWORD = 0x00020000;
pub const WS_HSCROLL: DWORD = 0x00100000;
pub const WS_ICONIC: DWORD = 0x20000000;
pub const WS_MAXIMIZE: DWORD = 0x01000000;
pub const WS_MAXIMIZEBOX: DWORD = 0x00010000;
pub const WS_MINIMIZE: DWORD = 0x20000000;
pub const WS_MINIMIZEBOX: DWORD = 0x00020000;
pub const WS_OVERLAPPED: DWORD = 0x00000000;
pub const WS_OVERLAPPEDWINDOW: DWORD = (WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX);
pub const WS_POPUP: DWORD = 0x80000000;
pub const WS_POPUPWINDOW: DWORD = (WS_POPUP | WS_BORDER | WS_SYSMENU);
pub const WS_SIZEBOX: DWORD = 0x00040000;
pub const WS_SYSMENU: DWORD = 0x00080000;
pub const WS_TABSTOP: DWORD = 0x00010000;
pub const WS_THICKFRAME: DWORD = 0x00040000;
pub const WS_TILED: DWORD = 0x00000000;
pub const WS_TILEDWINDOW: DWORD = (WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX);
pub const WS_VISIBLE: DWORD = 0x10000000;
pub const WS_VSCROLL: DWORD = 0x00200000;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ff700543(v=vs.85).aspx
pub const WS_EX_ACCEPTFILES: DWORD = 0x00000010;
pub const WS_EX_APPWINDOW: DWORD = 0x00040000;
pub const WS_EX_CLIENTEDGE: DWORD = 0x00000200;
pub const WS_EX_COMPOSITED: DWORD = 0x02000000;
pub const WS_EX_CONTEXTHELP: DWORD = 0x00000400;
pub const WS_EX_CONTROLPARENT: DWORD = 0x00010000;
pub const WS_EX_DLGMODALFRAME: DWORD = 0x00000001;
pub const WS_EX_LAYERED: DWORD = 0x00080000;
pub const WS_EX_LAYOUTRTL: DWORD = 0x00400000;
pub const WS_EX_LEFT: DWORD = 0x00000000;
pub const WS_EX_LEFTSCROLLBAR: DWORD = 0x00004000;
pub const WS_EX_LTRREADING: DWORD = 0x00000000;
pub const WS_EX_MDICHILD: DWORD = 0x00000040;
pub const WS_EX_NOACTIVATE: DWORD = 0x08000000;
pub const WS_EX_NOINHERITLAYOUT: DWORD = 0x00100000;
pub const WS_EX_NOPARENTNOTIFY: DWORD = 0x00000004;
pub const WS_EX_NOREDIRECTIONBITMAP: DWORD = 0x00200000;
pub const WS_EX_OVERLAPPEDWINDOW: DWORD = (WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE);
pub const WS_EX_PALETTEWINDOW: DWORD = (WS_EX_WINDOWEDGE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST);
pub const WS_EX_RIGHT: DWORD = 0x00001000;
pub const WS_EX_RIGHTSCROLLBAR: DWORD = 0x00000000;
pub const WS_EX_RTLREADING: DWORD = 0x00002000;
pub const WS_EX_STATICEDGE: DWORD = 0x00020000;
pub const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
pub const WS_EX_TOPMOST: DWORD = 0x00000008;
pub const WS_EX_TRANSPARENT: DWORD = 0x00000020;
pub const WS_EX_WINDOWEDGE: DWORD = 0x00000100;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms633573(v=vs.85).aspx
pub type WNDPROC = extern "stdcall" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT;

// ?
pub type HGLRC = HANDLE;

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms633577(v=vs.85).aspx
#[repr(C)]
pub struct WNDCLASSEX {
    pub cbSize: UINT,
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: libc::c_int,
    pub cbWndExtra: libc::c_int,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: LPCWSTR,
    pub lpszClassName: LPCWSTR,
    pub hIconSm: HICON,
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd162805(v=vs.85).aspxtag
#[repr(C)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms644958(v=vs.85).aspx
#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd162768(v=vs.85).aspx
#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [BYTE, ..32],
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd162897(v=vs.85).aspx
#[repr(C)]
pub struct RECT {
    pub left: LONG,
    pub top: LONG,
    pub right: LONG,
    pub bottom: LONG,
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd368826(v=vs.85).aspx
#[repr(C)]
pub struct PIXELFORMATDESCRIPTOR {
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

// http://msdn.microsoft.com/en-us/library/dd162807(v=vs.85).aspx
#[repr(C)]
pub struct POINTL {
    pub x: LONG,
    pub y: LONG,
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd183565(v=vs.85).aspx
#[repr(C)]
pub struct DEVMODE {
    pub dmDeviceName: [WCHAR, ..32],
    pub dmSpecVersion: WORD,
    pub dmDriverVersion: WORD,
    pub dmSize: WORD,
    pub dmDriverExtra: WORD,
    pub dmFields: DWORD,
    pub union1: [u8, ..16],
    pub dmColor: libc::c_short,
    pub dmDuplex: libc::c_short,
    pub dmYResolution: libc::c_short,
    pub dmTTOption: libc::c_short,
    pub dmCollate: libc::c_short,
    pub dmFormName: [WCHAR, ..32],
    pub dmLogPixels: WORD,
    pub dmBitsPerPel: DWORD,
    pub dmPelsWidth: DWORD,
    pub dmPelsHeight: DWORD,
    pub dmDisplayFlags: DWORD,
    pub dmDisplayFrequency: DWORD,
    pub dmICMMethod: DWORD,
    pub dmICMIntent: DWORD,
    pub dmMediaType: DWORD,
    pub dmDitherType: DWORD,
    dmReserved1: DWORD,
    dmReserved2: DWORD,
    pub dmPanningWidth: DWORD,
    pub dmPanningHeight: DWORD,
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/ms632611(v=vs.85).aspx
#[repr(C)]
pub struct WINDOWPLACEMENT {
    pub length: UINT,
    pub flags: UINT,
    pub showCmd: UINT,
    pub ptMinPosition: POINT,
    pub ptMaxPosition: POINT,
    pub rcNormalPosition: RECT,
}

// http://msdn.microsoft.com/en-us/library/windows/desktop/dd183569(v=vs.85).aspx
#[repr(C)]
pub struct DISPLAY_DEVICEW {
    pub cb: DWORD,
    pub DeviceName: [WCHAR, ..32],
    pub DeviceString: [WCHAR, ..128],
    pub StateFlags: DWORD,
    pub DeviceID: [WCHAR, ..128],
    pub DeviceKey: [WCHAR, ..128],
}

pub type LPMSG = *mut MSG;

extern "system" {
    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms632667(v=vs.85).aspx
    pub fn AdjustWindowRectEx(lpRect: *mut RECT, dwStyle: DWORD, bMenu: BOOL,
        dwExStyle: DWORD) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd183362(v=vs.85).aspx
    pub fn BeginPaint(hwnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd183411(v=vs.85).aspx
    pub fn ChangeDisplaySettingsW(lpDevMode: *mut DEVMODE, dwFlags: DWORD) -> LONG;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd183413(v=vs.85).aspx
    pub fn ChangeDisplaySettingsExW(lpszDeviceName: LPCWSTR, lpDevMode: *mut DEVMODE, hwnd: HWND,
        dwFlags: DWORD, lParam: LPVOID) -> LONG;

    // http://msdn.microsoft.com/en-us/library/dd318284(v=vs.85).aspx
    pub fn ChoosePixelFormat(hdc: HDC, ppfd: *const PIXELFORMATDESCRIPTOR) -> libc::c_int;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms632680(v=vs.85).aspx
    pub fn CreateWindowExW(dwExStyle: DWORD, lpClassName: LPCWSTR, lpWindowName: LPCWSTR,
        dwStyle: DWORD, x: libc::c_int, y: libc::c_int, nWidth: libc::c_int, nHeight: libc::c_int,
        hWndParent: HWND, hMenu: HMENU, hInstance: HINSTANCE, lpParam: LPVOID) -> HWND;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633572(v=vs.85).aspx
    pub fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd318302(v=vs.85).aspx
    pub fn DescribePixelFormat(hdc: HDC, iPixelFormat: libc::c_int, nBytes: UINT,
        ppfd: *mut PIXELFORMATDESCRIPTOR) -> libc::c_int;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms632682(v=vs.85).aspx
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms644934(v=vs.85).aspx
    pub fn DispatchMessageW(lpmsg: *const MSG) -> LRESULT;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd162598(v=vs.85).aspx
    pub fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd162609(v=vs.85).aspx
    pub fn EnumDisplayDevicesW(lpDevice: LPCWSTR, iDevNum: DWORD,
        lpDisplayDevice: *mut DISPLAY_DEVICEW, dwFlags: DWORD) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/dd162612(v=vs.85).aspx
    pub fn EnumDisplaySettingsExW(lpszDeviceName: LPCWSTR, iModeNum: DWORD,
        lpDevMode: *mut DEVMODE, dwFlags: DWORD) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd162719(v=vs.85).aspx
    pub fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> libc::c_int;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633503(v=vs.85).aspx
    pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/dd144871(v=vs.85).aspx
    pub fn GetDC(hWnd: HWND) -> HDC;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms679360(v=vs.85).aspx
    pub fn GetLastError() -> DWORD;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms644936(v=vs.85).aspx
    pub fn GetMessageW(lpMsg: LPMSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms683199(v=vs.85).aspx
    pub fn GetModuleHandleW(lpModuleName: LPCWSTR) -> HMODULE;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms683212(v=vs.85).aspx
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: LPCSTR) -> *const libc::c_void;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633518(v=vs.85).aspx
    pub fn GetWindowPlacement(hWnd: HWND, lpwndpl: *mut WINDOWPLACEMENT) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633519(v=vs.85).aspx
    pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms684175(v=vs.85).aspx
    pub fn LoadLibraryW(lpFileName: LPCWSTR) -> HMODULE;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/aa366730(v=vs.85).aspx
    pub fn LocalFree(hMem: HLOCAL) -> HLOCAL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms644943(v=vs.85).aspx
    pub fn PeekMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT,
        wRemoveMsg: UINT) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms644944(v=vs.85).aspx
    pub fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms644945(v=vs.85).aspx
    pub fn PostQuitMessage(nExitCode: libc::c_int);

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633586(v=vs.85).aspx
    pub fn RegisterClassExW(lpWndClass: *const WNDCLASSEX) -> ATOM;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633539(v=vs.85).aspx
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd369049(v=vs.85).aspx
    pub fn SetPixelFormat(hdc: HDC, iPixelFormat: libc::c_int,
        ppfd: *const PIXELFORMATDESCRIPTOR) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633545(v=vs.85).aspx
    pub fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, X: libc::c_int, Y: libc::c_int,
        cx: libc::c_int, cy: libc::c_int, uFlags: UINT) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633546(v=vs.85).aspx
    pub fn SetWindowTextW(hWnd: HWND, lpString: LPCWSTR) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms633548(v=vs.85).aspx
    pub fn ShowWindow(hWnd: HWND, nCmdShow: libc::c_int) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/dd369060(v=vs.85).aspx
    pub fn SwapBuffers(hdc: HDC) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms644934(v=vs.85).aspx
    pub fn TranslateMessage(lpmsg: *const MSG) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/dd145167(v=vs.85).aspx
    pub fn UpdateWindow(hWnd: HWND) -> BOOL;

    // http://msdn.microsoft.com/en-us/library/windows/desktop/ms644956(v=vs.85).aspx
    pub fn WaitMessage() -> BOOL;
}
