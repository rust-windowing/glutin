#[cfg(feature = "headless")]
pub use self::headless::HeadlessContext;

#[cfg(feature = "window")]
pub use self::window::{Window, WindowProxy, MonitorID, get_available_monitors, get_primary_monitor};
#[cfg(feature = "window")]
pub use self::window::{WaitEventsIterator, PollEventsIterator};

pub mod ffi;
mod glx_context;

#[cfg(feature = "headless")]
mod headless;

#[cfg(feature = "window")]
mod window;

#[cfg(not(feature = "window"))]
pub type Window = ();       // TODO: hack to make things work
#[cfg(not(feature = "window"))]
pub type MonitorID = ();       // TODO: hack to make things work
