//! This incomplete example will likely crash at runtime on Windows
//! when the `windows-static-egl` feature is enabled,
//! but it should still compile and link.

extern crate glutin;

use std::os::raw::{c_char, c_void};

#[allow(non_snake_case)]
#[no_mangle]
pub extern fn eglGetProcAddress(_name: *const c_char) -> *const c_void {
    0 as _
}

fn main() {
    let events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("A fantastic window!");
    let context = glutin::ContextBuilder::new();
    let _gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();
}
