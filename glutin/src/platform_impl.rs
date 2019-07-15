pub use self::platform_impl::*;

#[cfg(target_os = "windows")]
#[path = "platform_impl/windows/windows.rs"]
mod platform_impl;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[path = "platform_impl/unix/unix.rs"]
mod platform_impl;
#[cfg(target_os = "macos")]
#[path = "platform_impl/macos/macos.rs"]
mod platform_impl;
#[cfg(target_os = "android")]
#[path = "platform_impl/android/android.rs"]
mod platform_impl;
#[cfg(target_os = "ios")]
#[path = "platform_impl/ios/ios.rs"]
mod platform_impl;
