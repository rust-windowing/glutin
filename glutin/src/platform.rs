//! Contains traits with platform-specific methods in them.
//!
//! Contains the following modules:
//!
//!  - `android`
//!  - `ios`
//!  - `macos`
//!  - `unix`
//!  - `windows`
//!

/// Platform-specific methods for android.
pub mod android;
/// Platform-specific methods for iOS.
pub mod ios;
/// Platform-specific methods for macOS.
pub mod macos;
/// Platform-specific methods for unix operating systems.
pub mod unix;
/// Platform-specific methods for Windows.
pub mod windows;
