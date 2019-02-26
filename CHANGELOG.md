# Unreleased

- We no longer load `libegl.so` and `libgl.so` multiple times.
- Fixes `Context::is_current` incorrectly returning `false`.
- **Breaking:** Renamed `GlContext{,Ext}` to `ContextTrait{,Ext}`.
- Implemented context sharing support for Windows and Linux.
- Added `SeparatedContext`.
- **Breaking:** Renamed `GlWindow` to `CombinedContext`.
- **Breaking:** Removed `shareable_with_windowed_contexts`. Now you must build
OsMesa contexts via a separate extension.
- Added `ContextBuilder::build` method.
- On X11 and Wayland, you can now use shared contexts, however, one limitation 
of the Wayland backend is that all shared contexts must use the same events
pool as each other.
- Added context sharing support to windows.
- Improved docs.
- Refactored code to be more consistent/cleaner. Ran rustfmt on everything.
- Added NetBSD support.
- **Breaking:** Removed `new_shared` function from `Context` and `GlWindow`, in favor of `new`.
- Added `build` method to `ContextBuilder`.
- Added `get_egl_display` method to `GlContextExt` trait and its implementation for platforms.
- Removed minimum supported Rust version guarantee.
- `NoBackendAvailable` is now `Sync`, as a result `CreationError` is also `Sync`.

# Version 0.19.0 (2018-11-09)

- **Breaking:** The entire API for headless contexts has been removed. Please instead use `Context::new()` when trying to make a context without a visible window. Also removed `headless` feature.
- **Breaking:** Types implementing the `GlContext` trait must now be sized.
- **Breaking:** Added new `CreationErrorPair` enum variant to enum `CreationError`.
- Remove requirement for EGL dev packages on Wayland.
- Update winit dependency to 0.18.0. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/v0.18.0/CHANGELOG.md#version-0180-2018-11-07) for more info.

# Version 0.18.0 (2018-08-03)

- cocoa and core-graphics updates.
- **Breaking:** Added `OsError` variant to `ContextError`.
- Improved glX error reporting.
- The iOS backend no longer fails to compile... again (added iOS testing on CI to prevent further issues).
- Update winit dependency to 0.17.0. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/v0.17.0/CHANGELOG.md#version-0170-2018-08-02) for more info.

# Version 0.17.0 (2018-06-27)

- Fix regression that prevented automatic graphics switching in macOS ([#980](https://github.com/tomaka/glutin/issues/980)).
- Add `ContextBuilder::with_double_buffer` function.
- Add `ContextBuilder::with_hardware_acceleration` function.
- Work around a presumed Android emulator bug
  that would cause context creation to return `CreationError::OpenGlVersionNotSupported`
  in some configurations
  ([#1036](https://github.com/tomaka/glutin/pull/1036)).
- Update winit dependency to 0.16.0. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/v0.16.0/CHANGELOG.md#version-0160-2018-06-25) for more info.
- The iOS backend no longer fails to compile.

# Version 0.16.0 (2018-05-09)

- Update winit dependency to 0.14.0. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/v0.14.0/CHANGELOG.md#version-0140-2018-05-09) for more info.
- Update winit dependency to 0.15.0. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/v0.15.0/CHANGELOG.md#version-0150-2018-05-22) for more info.

# Version 0.15.0 (2018-04-25)

- Update winit dependency to 0.13.0. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/v0.13.0/CHANGELOG.md#version-0130-2018-04-25) for more info.

# Version 0.14.0 (2018-04-06)

- Update winit dependency to 0.12.0. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/master/CHANGELOG.md#version-0120-2018-04-06) for more info.
- Update Wayland backend to not use deprecated `get_inner_size_points` method.

# Version 0.13.1 (2018-03-07)

- Fix Android activity life cycle.
- Update winit dependency to 0.11.2. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/master/CHANGELOG.md#version-0112-2018-03-06) for more info.

# Version 0.13.0 (2018-02-28)

- Update winit dependency to 0.11.1. See [winit's CHANGELOG](https://github.com/tomaka/winit/blob/master/CHANGELOG.md#version-0111-2018-02-19) for more info.

# Version 0.12.2 (2018-02-12)

- Don't use yanked version of winit.

# Version 0.12.1 (2018-02-05)

- Add support for winapi 0.3 ([#975](https://github.com/tomaka/glutin/pull/975)).
- Fix macOS to return compatibility profile if applicable ([#977](https://github.com/tomaka/glutin/pull/977)).
- Update gl_generator and macOS dependencies.
