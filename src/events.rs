#[cfg(not(target_os = "emscripten"))]
pub use winit::{Event, TouchPhase, Touch, ScanCode, ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode};

#[cfg(target_os = "emscripten")]
pub use super::platform::winit::{Event, TouchPhase, Touch, ScanCode, ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode};
