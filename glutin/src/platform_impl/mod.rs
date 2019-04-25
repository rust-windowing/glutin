pub use self::platform_impl::*;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform_impl;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[path = "unix/mod.rs"]
mod platform_impl;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform_impl;
#[cfg(target_os = "android")]
#[path = "android/mod.rs"]
mod platform_impl;
#[cfg(target_os = "ios")]
#[path = "ios/mod.rs"]
mod platform_impl;
#[cfg(target_os = "emscripten")]
#[path = "emscripten/mod.rs"]
mod platform_impl;
