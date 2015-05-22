#![cfg(target_os = "macos")]

pub use api::cocoa::*;

#[cfg(feature = "joystick")]
pub mod joystick;
