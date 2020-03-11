# Unreleased

# Version 0.24.0 (2020-03-11)

- Updated winit dependency to 0.22.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#0220-2020-03-09) for more info.

# Version 0.23.0 (2020-02-06)

- Updated winit dependency to 0.21.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#0210-2020-02-04) for more info.
- Removed broken CI for the `armv7-apple-ios` target.

# Version 0.22.1 (2020-01-29)

- Fixed incorrectly documented default value for `ContextBuilder::with_srgb`

# Version 0.22.0 (2020-01-07)

- Updated winit dependency to 0.20.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#0200-2020-01-05) for more info.

# Version 0.22.0-alpha6 (2020-01-05)

- Fixed dependencies so wrong winit version is not used.
- On X11, got rid of mistaken `XRenderFindVisualFormat` call so that glutin doesn't ignore configs that lack a `XRenderPictFormat`.
- On iOS, fixed not linking against OpenGLES.framework.
- On X11, fixed VSync not being disabled when requested.

# Version 0.22.0-alpha5 (2019-11-14)

- Fixed build issue.

# Version 0.22.0-alpha4 (2019-11-10)

- Update winit dependency to 0.20.0-alpha4. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#0200-alpha-4) for more info.
- Added an xcode example for building for iOS.
- Made using sRGB the default.
- MacOSX's raw_handle trait method  now returns the CGLContext object.

# Version 0.22.0-alpha3 (2019-8-15)

 - Switched from needing a `EventLoop` to a `EventLoopWindowTarget`

# Version 0.22.0-alpha2 (2019-08-15)

- Fixed attribute handling for sRGB in WGL.
- Fixed VSync being always enabled on EGL.

# Version 0.20.1 (2019-08-08)

 - **Backport:** We now load `libGL.so` instead of `libGLX.so`. 

# Version 0.22.0-alpha1 (2019-06-21)

- Update winit dependency to 0.20.0-alpha1. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#0200-alpha-1) for more info.

# Version 0.21.0 (2019-04-20)

 - Bumped dependencies, fixed docs.

# Version 0.21.0-rc3 (2019-04-13)

 - Bumped dependencies.

# Version 0.21.0-rc2 (2019-04-08)

 - **Breaking**: Removed `DisplayLost` variant to `ContextError`.
 - **Breaking**: Renamed `NotCurrentContext` to `NotCurrent`.
 - **Breaking**: Renamed `PossiblyCurrentContext` to `PossiblyCurrent`.
 - Added `treat_as_current` function.

# Version 0.21.0-rc1 (2019-04-07)

 - **Breaking:** Replaced `CreationErrorPair` enum variant with `CreationErrors`.
 - Added `Clone` to `ContextBuilder`.
 - Added headless example.
 - Removed internal code relating to libcaca.
 - Implemented `Debug` on all public facing types.
 - Dropping contexts on platforms using egl and/or glx no longer resets the
 current context, if the context dropped wasn't the current context.
 - Added context sharing support to MacOS.
 - **Breaking**: Removed `ContextTrait`.
 - **Breaking**: Renamed `OsMesaContextExt` to `HeadlessContextExt`. Added functions
 for using egl-surfaceless.
 - **Breaking**: Changed `WindowedContext` and `RawContext` into typedefs of
 `ContextWrapper`.
 - **Breaking**: Removed `new_windowed` and `new_headless` from `WindowedContext`
 and `Context`, respectively.
 - **Breaking**: Added two new types, `NotCurrentContext` and `PossiblyCurrentContext`,
 which `RawContext`, `WindowedContext`, `ContextBuilder` and `Context` are now
 generic over.
 - Added `{make,treat_as}_not_current` function to `{Raw,Windowed,}Context`.
 - We now load `libGL.so` instead of `libGLX.so`.
 - **Breaking**: Added `DisplayLost` variant to `ContextError`.
 - Fixed bug where we drop the hidden window belonging to a headless context on
 on X11 and/or Wayland before the actual context.
 - "Fixed" bug where we will close `EGLDisplay`s while they are still in use by
 others. Angry and/or salty rant can be found in `glutin/src/api/egl/mod.rs`,
 you can't miss it.
 - **Breaking**: `WindowedContext`s now deref to `Context`, not `Window`. 
 Please use `.window()` to access the window.

# Version 0.20.0 (2019-03-09)

- We no longer load `libEGL.so` and `libGL.so` multiple times.
- Fixes `Context::is_current` incorrectly returning `false`.
- Made `ContextBuilder`'s `pf_reqs` public.
- **Breaking:** Renamed `GlContext{,Ext}` to `ContextTrait{,Ext}`.
- **Breaking:** Renamed `GlWindow` to `WindowedContext`.
- Implemented context sharing support for Windows and Linux.
- Added support for contexts made from raw parts for Windows and Linux.
- **Breaking:** Removed `shareable_with_windowed_contexts`. Now you must build
OsMesa contexts via a separate extension.
- Added `ContextBuilder::build_{windowed,headless}` methods.
- **Breaking:** Renamed `Context::new` to `Context::new_headless`. `new_headless` now accepts dimensions for the off-screen surface backing it.
- **Breaking:** Renamed `GlWindow::new` to `WindowedContext::new_windowed`.
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
- Update winit dependency to 0.19.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#version-0190-2019-03-06) for more info.

# Version 0.19.0 (2018-11-09)

- **Breaking:** The entire API for headless contexts has been removed. Please instead use `Context::new()` when trying to make a context without a visible window. Also removed `headless` feature.
- **Breaking:** Types implementing the `GlContext` trait must now be sized.
- **Breaking:** Added new `CreationErrorPair` enum variant to enum `CreationError`.
- Remove requirement for EGL dev packages on Wayland.
- Update winit dependency to 0.18.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/v0.18.0/CHANGELOG.md#version-0180-2018-11-07) for more info.

# Version 0.18.0 (2018-08-03)

- cocoa and core-graphics updates.
- **Breaking:** Added `OsError` variant to `ContextError`.
- Improved glX error reporting.
- The iOS backend no longer fails to compile... again (added iOS testing on CI to prevent further issues).
- Update winit dependency to 0.17.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/v0.17.0/CHANGELOG.md#version-0170-2018-08-02) for more info.

# Version 0.17.0 (2018-06-27)

- Fix regression that prevented automatic graphics switching in macOS ([#980](https://github.com/rust-windowing/glutin/issues/980)).
- Add `ContextBuilder::with_double_buffer` function.
- Add `ContextBuilder::with_hardware_acceleration` function.
- Work around a presumed Android emulator bug
  that would cause context creation to return `CreationError::OpenGlVersionNotSupported`
  in some configurations
  ([#1036](https://github.com/rust-windowing/glutin/pull/1036)).
- Update winit dependency to 0.16.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/v0.16.0/CHANGELOG.md#version-0160-2018-06-25) for more info.
- The iOS backend no longer fails to compile.

# Version 0.16.0 (2018-05-09)

- Update winit dependency to 0.14.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/v0.14.0/CHANGELOG.md#version-0140-2018-05-09) for more info.
- Update winit dependency to 0.15.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/v0.15.0/CHANGELOG.md#version-0150-2018-05-22) for more info.

# Version 0.15.0 (2018-04-25)

- Update winit dependency to 0.13.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/v0.13.0/CHANGELOG.md#version-0130-2018-04-25) for more info.

# Version 0.14.0 (2018-04-06)

- Update winit dependency to 0.12.0. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#version-0120-2018-04-06) for more info.
- Update Wayland backend to not use deprecated `get_inner_size_points` method.

# Version 0.13.1 (2018-03-07)

- Fix Android activity life cycle.
- Update winit dependency to 0.11.2. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#version-0112-2018-03-06) for more info.

# Version 0.13.0 (2018-02-28)

- Update winit dependency to 0.11.1. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#version-0111-2018-02-19) for more info.

# Version 0.12.2 (2018-02-12)

- Don't use yanked version of winit.

# Version 0.12.1 (2018-02-05)

- Add support for winapi 0.3 ([#975](https://github.com/rust-windowing/glutin/pull/975)).
- Fix macOS to return compatibility profile if applicable ([#977](https://github.com/rust-windowing/glutin/pull/977)).
- Update gl_generator and macOS dependencies.
