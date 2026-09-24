# Unreleased

- **Breaking:** Update _winit_ to `0.31.0-beta.3`. See [winit's CHANGELOG](https://docs.rs/winit/0.31.0-beta.3/winit/changelog/index.html) for more info.
- **Breaking:** Windows are now returned as `Box<dyn Window>` and `GlWindow` is implemented for `dyn Window`.
- **Breaking:** Removed the `GlutinEventLoop` trait, functions take `&dyn ActiveEventLoop` instead, since winit's `EventLoop` can no longer create windows.
- **Breaking:** `finalize_window` now returns winit's `RequestError` instead of `OsError`.
- On X11, `finalize_window` now preserves existing `WindowAttributesX11` when setting the visual.

# Version 0.5.0

- **Breaking:** Update _winit_ to `0.30`. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/releases/tag/v0.30.0) for more info.
- Add the `GlutinEventLoop` trait to maintain compatibility with the now
  deprecated `EventLoop` but also support the new `ActiveEventLoop`.
- Update `DisplayBuilder` to use `WindowAttributes` instead of `WindowBuilder`.

# Version 0.4.2

- **Breaking:** Update _glutin_ to `0.31.0`. See [glutin's CHANGELOG](https://github.com/rust-windowing/glutin/releases/tag/v0.31.0) for more info.
- **Breaking:** Update _winit_ to `0.29.2`. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/releases/tag/v0.29.2) for more info.
- **Breaking:** Fixed a typo in a type name (`ApiPrefence` -> `ApiPreference`).

# Version 0.3.0

- **Breaking:** Update _winit_ to `0.28`. See [winit's CHANGELOG](https://github.com/rust-windowing/winit/releases/tag/v0.28.0) for more info.

# Version 0.2.2

- Add traits `GlWindow` with helper methods for building and resizing surfaces using a winit `Window`.

# Version 0.2.1

- Fix WGL window initialization.

# Version 0.2.0

- Fix API typo.

# Version 0.1.0

- Implement _glutin-winit_ helpers.
