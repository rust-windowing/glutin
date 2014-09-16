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

    /// A key is down.
    KeyDown(KeyboardEvent),

    /// A key is up.
    KeyUp(KeyboardEvent),

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

/// Describes a user interaction with the keyboard.
#[deriving(Show, Hash, PartialEq, Eq, Clone)]
pub struct KeyboardEvent {
    /// The code value of the key represented by the event.
    /// 
    /// Represents a physical key, that is value not changed neither by the modifier state, nor
    ///  by keyboard layout.
    /// If the inputting keyboard isn't a physical keyboard, e.g., using virtual keyboard or
    ///  accessibility tools, web browsers should set proper code value for compatibility as
    ///  far as possible.
    ///
    /// See https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent.code#Code_values
    pub code: Option<&'static str>,
    /// The key value of the key represented by the event.
    ///
    /// See https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent.key#Key_values
    pub key: Option<&'static str>,
    /// The location of the key on the keyboard or other input device.
    pub location: KeyLocation,
    /// A locale string indicating the locale the keyboard is configured for.
    pub local: Option<String>,
    /// True if the key is being held down such that it is automatically repeating.
    pub repeat: bool,
    /// True if the Alt key (or Option, on Mac) was active when the event was generated.
    pub alt_key: bool,
    /// True if the Control key was active when the event was generated.
    pub ctrl_key: bool,
    /// True if the Meta (or Command, on Mac) key was active when the key event was generated.
    pub meta_key: bool,
    /// True if the Shift key was active when the event was generated.
    pub shift_key: bool,
    /// True if the event is fired between after CompositionStart and before CompositionEnd.
    pub is_composing: bool,
}

/// Describes the location on the keyboard of key events.
#[deriving(Show, Hash, PartialEq, Eq, Clone)]
pub enum KeyLocation {
    /// The key must not be distinguished between the left and right versions of the key, and was
    ///  not pressed on the numeric keypad or a key that is considered to be part of the keypad.
    StandardLocation,

    /// The key was the left-hand version of the key; for example, this is the value of the
    ///  location attribute when the left-hand Control key is pressed on a standard 101 key US
    ///  keyboard. This value is only used for keys that have more that one possible location on
    ///  the keyboard.
    LeftLocation,

    /// The key was the right-hand version of the key; for example, this is the value of the
    ///  location attribute when the right-hand Control key is pressed on a standard 101 key US
    ///  keyboard. This value is only used for keys that have more that one possible location on
    ///  the keyboard.
    RightLocation,

    /// The key was on the numeric keypad, or has a virtual key code that corresponds to the
    ///  numeric keypad.
    NumpadLocation,
}

#[deriving(Show, Hash, PartialEq, Eq, Clone)]
pub enum MouseButton {
    LeftMouseButton,
    RightMouseButton,
    MiddleMouseButton,
    OtherMouseButton(u8),
}
