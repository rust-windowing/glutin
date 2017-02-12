use std::collections::VecDeque;
use std::path::PathBuf;

use CreationError;

pub use api::emscripten::{WindowProxy, MonitorId, get_available_monitors, AvailableMonitorsIter};
pub use api::emscripten::{get_primary_monitor, WaitEventsIterator, PollEventsIterator};

pub use native_monitor::NativeMonitorId;

#[derive(Clone, Debug)]
pub enum Event {
    Resized(u32, u32),
    Moved(i32, i32),
    Closed,
    DroppedFile(PathBuf),
    ReceivedCharacter(char),
    Focused(bool),
    KeyboardInput(ElementState, ScanCode, Option<VirtualKeyCode>),
    MouseMoved(i32, i32),
    MouseEntered,
    MouseLeft,
    MouseWheel(MouseScrollDelta, TouchPhase),
    MouseInput(ElementState, MouseButton),
    TouchpadPressure(f32, i64),
    Awakened,
    Refresh,
    Suspended(bool),
    Touch(Touch),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone, Copy)]
pub struct Touch {
    pub phase: TouchPhase,
    pub location: (f64, f64),
    pub id: u64,
}

pub type ScanCode = u8;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ElementState {
    Pressed,
    Released,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseScrollDelta {
    LineDelta(f32, f32),
    PixelDelta(f32, f32),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum VirtualKeyCode {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Escape,
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
    Snapshot,
    Scroll,
    Pause,
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,
    Left,
    Up,
    Right,
    Down,
    Back,
    Return,
    Space,
    Compose,
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
    AbntC1,
    AbntC2,
    Add,
    Apostrophe,
    Apps,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Decimal,
    Divide,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LMenu,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Multiply,
    Mute,
    MyComputer,
    NavigateForward,
    NavigateBackward,
    NextTrack,
    NoConvert,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    OEM102,
    Period,
    PlayPause,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RMenu,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Subtract,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CursorState {
    Normal,
    Hide,
    Grab,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MouseCursor {
    Default,
    Crosshair,
    Hand,
    Arrow,
    Move,
    Text,
    Wait,
    Help,
    Progress,
    NotAllowed,
    ContextMenu,
    NoneCursor,
    Cell,
    VerticalText,
    Alias,
    Copy,
    NoDrop,
    Grab,
    Grabbing,
    AllScroll,
    ZoomIn,
    ZoomOut,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
}


#[derive(Clone)]
pub struct WindowBuilder {
    pub window: WindowAttributes
}

impl WindowBuilder {
    /// Initializes a new `WindowBuilder` with default values.
    #[inline]
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            window: WindowAttributes::default()
        }
    }

    /// Requests the window to be of specific dimensions.
    ///
    /// Width and height are in pixels.
    #[inline]
    pub fn with_dimensions(mut self, width: u32, height: u32) -> WindowBuilder {
        self.window.dimensions = Some((width, height));
        self
    }
    
    /// Sets a minimum dimension size for the window
    ///
    /// Width and height are in pixels.
    #[inline]
    pub fn with_min_dimensions(mut self, width: u32, height: u32) -> WindowBuilder {
        self.window.min_dimensions = Some((width, height));
        self
    }

    /// Sets a maximum dimension size for the window
    ///
    /// Width and height are in pixels.
    #[inline]
    pub fn with_max_dimensions(mut self, width: u32, height: u32) -> WindowBuilder {
        self.window.max_dimensions = Some((width, height));
        self
    }

    /// Requests a specific title for the window.
    #[inline]
    pub fn with_title<T: Into<String>>(mut self, title: T) -> WindowBuilder {
        self.window.title = title.into();
        self
    }

    /// Requests fullscreen mode.
    ///
    /// If you don't specify dimensions for the window, it will match the monitor's.
    #[inline]
    pub fn with_fullscreen(mut self, monitor: MonitorId) -> WindowBuilder {
        self.window.monitor = Some(monitor);
        self
    }

    /// Sets whether the window will be initially hidden or visible.
    #[inline]
    pub fn with_visibility(mut self, visible: bool) -> WindowBuilder {
        self.window.visible = visible;
        self
    }

    /// Sets whether the background of the window should be transparent.
    #[inline]
    pub fn with_transparency(mut self, transparent: bool) -> WindowBuilder {
        self.window.transparent = transparent;
        self
    }

    /// Sets whether the window should have a border, a title bar, etc.
    #[inline]
    pub fn with_decorations(mut self, decorations: bool) -> WindowBuilder {
        self.window.decorations = decorations;
        self
    }

    /// Enables multitouch
    #[inline]
    pub fn with_multitouch(mut self) -> WindowBuilder {
        self.window.multitouch = true;
        self
    }

    /// Builds the window.
    ///
    /// Error should be very rare and only occur in case of permission denied, incompatible system,
    /// out of memory, etc.
    pub fn build(mut self) -> Result<Window, CreationError> {
        // resizing the window to the dimensions of the monitor when fullscreen
        if self.window.dimensions.is_none() && self.window.monitor.is_some() {
            self.window.dimensions = Some(self.window.monitor.as_ref().unwrap().get_dimensions())
        }

        // default dimensions
        if self.window.dimensions.is_none() {
            self.window.dimensions = Some((1024, 768));
        }

        // building
        Ok(Window::new())
    }

    /// Builds the window.
    ///
    /// The context is build in a *strict* way. That means that if the backend couldn't give
    /// you what you requested, an `Err` will be returned.
    #[inline]
    pub fn build_strict(self) -> Result<Window, CreationError> {
        self.build()
    }
}

pub struct Window;

impl Window {
    pub fn new() -> Window {
        Window
    }
}


// Copied from winit
pub mod native_monitor {
    /// Native platform identifier for a monitor. Different platforms use fundamentally different types
    /// to represent a monitor ID.
    #[derive(Clone, PartialEq, Eq)]
    pub enum NativeMonitorId {
        /// Cocoa and X11 use a numeric identifier to represent a monitor.
        Numeric(u32),

        /// Win32 uses a Unicode string to represent a monitor.
        Name(String),

        /// Other platforms (Android) don't support monitor identification.
        Unavailable
    }
}

#[derive(Clone)]
pub struct WindowAttributes {
    pub dimensions: Option<(u32, u32)>,
    pub min_dimensions: Option<(u32, u32)>,
    pub max_dimensions: Option<(u32, u32)>,
    pub monitor: Option<MonitorId>,
    pub title: String,
    pub visible: bool,
    pub transparent: bool,
    pub decorations: bool,
    pub multitouch: bool,
}

impl Default for WindowAttributes {
    fn default() -> WindowAttributes {
        WindowAttributes {
            dimensions: None,
            min_dimensions: None,
            max_dimensions: None,
            monitor: None,
            title: "glutin window".to_owned(),
            visible: true,
            transparent: false,
            decorations: true,
            multitouch: false,
        }
    }
}
