extern crate glutin;

use glutin::winit::{self, MouseCursor};

mod support;

fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let window = winit::WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&events_loop)
        .unwrap();
    let context = glutin::ContextBuilder::new()
        .build(&window)
        .unwrap();

    unsafe { context.make_current().unwrap() };

    let gl = support::load(&context);
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
        use glutin::winit::{Event, WindowEvent, ElementState};
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput {
                    input: winit::KeyboardInput { state: ElementState::Pressed, .. }, ..
                } => {
                    println!("Setting cursor to \"{:?}\"", cursors[cursor_idx]);
                    window.set_cursor(cursors[cursor_idx]);
                    if cursor_idx < cursors.len() - 1 {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                },
                winit::WindowEvent::Closed => return winit::ControlFlow::Break,
                winit::WindowEvent::Resized(w, h) => context.resize(w, h),
                _ => (),
            }
        }

        gl.draw_frame([0.0, 1.0, 0.0, 1.0]);
        context.swap_buffers().unwrap();
        winit::ControlFlow::Continue
    });
}
