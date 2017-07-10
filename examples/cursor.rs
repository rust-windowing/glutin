extern crate glutin;

mod support;

use glutin::{GlContext, MouseCursor};

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("A fantastic window!");
    let context = glutin::ContextBuilder::new();
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe { gl_window.make_current().unwrap() };

    let gl = support::load(&gl_window);
    let cursors = [
        MouseCursor::Default, MouseCursor::Crosshair, MouseCursor::Hand, MouseCursor::Arrow,
        MouseCursor::Move, MouseCursor::Text, MouseCursor::Wait, MouseCursor::Help,
        MouseCursor::Progress, MouseCursor::NotAllowed, MouseCursor::ContextMenu,
        MouseCursor::NoneCursor, MouseCursor::Cell, MouseCursor::VerticalText, MouseCursor::Alias,
        MouseCursor::Copy, MouseCursor::NoDrop, MouseCursor::Grab, MouseCursor::Grabbing,
        MouseCursor::AllScroll, MouseCursor::ZoomIn, MouseCursor::ZoomOut, MouseCursor::EResize,
        MouseCursor::NResize, MouseCursor::NeResize, MouseCursor::NwResize, MouseCursor::SResize,
        MouseCursor::SeResize, MouseCursor::SwResize, MouseCursor::WResize, MouseCursor::EwResize,
        MouseCursor::NsResize, MouseCursor::NeswResize, MouseCursor::NwseResize,
        MouseCursor::ColResize, MouseCursor::RowResize,
    ];
    let mut cursor_idx = 0;

    events_loop.run_forever(|event| {
        use glutin::{ControlFlow, Event, WindowEvent, ElementState};
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput {
                    input: glutin::KeyboardInput { state: ElementState::Pressed, .. }, ..
                } => {
                    println!("Setting cursor to \"{:?}\"", cursors[cursor_idx]);
                    gl_window.set_cursor(cursors[cursor_idx]);
                    if cursor_idx < cursors.len() - 1 {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                },
                WindowEvent::Closed => return ControlFlow::Break,
                WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                _ => (),
            }
        }

        gl.draw_frame([0.0, 1.0, 0.0, 1.0]);
        gl_window.swap_buffers().unwrap();
        ControlFlow::Continue
    });
}
