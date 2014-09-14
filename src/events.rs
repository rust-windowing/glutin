#[deriving(Clone,Show)]
pub enum Event {
    /// The size of the window has changed.
    Resized(uint, uint),

    /// The position of the window has changed.
    Moved(int, int),

    /// The window has been closed.
    Closed,

    /// The window received a unicode character.
    Input {
        pub data: String,
        pub is_composing: bool
    },

    /// Fired when a  text composition system is enabled and a new composition session is about to
    ///  begin (or has begun, depending on the text composition system) in preparation for
    ///  composing a passage of text.
    CompositionStart {
        pub data: String
    },

    /// Dispatched during a composition session when a text composition system updates its active
    ///  text passage with a new character.
    CompositionUpdate {
        pub data: String
    },

    /// Dispatched when a text composition system completes or cancels the current composition
    ///  session.
    CompositionEnd {
        pub data: String
    },

    /// The window gained or lost focus.
    /// 
    /// The parameter is true if the window has gained focus, and false if it has lost focus.
    Focused(bool),

    /// An event from the keyboard has been received.
    KeyboardInput(ElementState, ScanCode, Option<VirtualKeyCode>, KeyModifiers),

    /// The cursor has moved on the window.
    MouseMove {
        /// X coordinate in pixels relative to the top-left hand corner of the window.
        pub x: int,
        /// Y coordinate in pixels relative to the top-left hand corner of the window.
        pub y: int,
    },

    /// The mouse has entered the window.
    MouseEnter,
    
    /// The mouse has left the window.
    MouseLeave,

    /// A positive value indicates that the wheel was rotated forward, away from the user ;
    ///  a negative value indicates that the wheel was rotated backward, toward the user.
    MouseWheel {
        /// 
        pub delta_x: i32,
        /// 
        pub delta_y: i32,
    },

    /// A mouse button is down.
    MouseDown {
        /// 
        pub button: MouseButton,
        /// 
        pub buttons: Vec<MouseButton>,
        /// X coordinate in pixels relative to the top-left hand corner of the window.
        pub x: int,
        /// Y coordinate in pixels relative to the top-left hand corner of the window.
        pub y: int,
    },

    /// A mouse button is down.
    MouseUp {
        /// 
        pub button: MouseButton,
        /// 
        pub buttons: Vec<MouseButton>,
        /// X coordinate in pixels relative to the top-left hand corner of the window.
        pub x: int,
        /// Y coordinate in pixels relative to the top-left hand corner of the window.
        pub y: int,
    }
}

pub type ScanCode = u8;

bitflags!(
    #[deriving(Show)]
    flags KeyModifiers: u8 {
        static LeftControlModifier = 1,
        static RightControlModifier = 2,
        static LeftShitModifier = 4,
        static RightShitModifier = 8,
        static LeftAltModifier = 16,
        static RightRightModifier = 32,
        static NumLockModifier = 64,
        static CapsLockModifier = 128
    }
)

#[deriving(Show, Hash, PartialEq, Eq, Clone)]
pub enum MouseButton {
    LeftMouseButton,
    RightMouseButton,
    MiddleMouseButton,
    OtherMouseButton(u8),
}

#[deriving(Show, Hash, PartialEq, Eq, Clone)]
pub enum ElementState {
    Pressed,
    Released,
}

#[deriving(Show, Hash, PartialEq, Eq, Clone)]
pub enum VirtualKeyCode {
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    A,
    AbntC1,
    AbntC2,
    Add,
    Apostrophe,
    Apps,
    At,
    Ax,
    B,
    Back,
    Backslash,
    C,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    D,
    Decimal,
    Delete,
    Divide,
    Down,
    E,
    End,
    Equals,
    Escape,
    F,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    G,
    Grave,
    H,
    Home,
    I,
    Insert,
    J,
    K,
    Kana,
    Kanji,
    L,
    LCracket,
    LControl,
    Left,
    LMenu,
    LShift,
    LWin,
    M,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Multiply,
    Mute,
    MyComputer,
    N,
    NextTrack,
    NoConvert,
    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    O,
    OEM102,
    P,
    PageDown,
    PageUp,
    Pause,
    Period,
    Playpause,
    Power,
    Prevtrack,
    Q,
    R,
    RBracket,
    RControl,
    Return,
    Right,
    RMenu,
    RShift,
    RWin,
    S,
    Scroll,
    Semicolon,
    Slash,
    Sleep,
    Snapshot,
    Space,
    Stop,
    Subtract,
    Sysrq,
    T,
    Tab,
    U,
    Underline,
    Unlabeled,
    Up,
    V,
    VolumeDown,
    VolumeUp,
    W,
    Wake,
    Webback,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    X,
    Y,
    Yen,
    Z
}
