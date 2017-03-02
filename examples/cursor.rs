extern crate glutin;

use glutin::MouseCursor;

mod support;

fn main() {
    let events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&events_loop)
        .unwrap();

    unsafe { window.make_current().unwrap() };

    let context = support::load(&window);
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
        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::KeyboardInput(glutin::ElementState::Pressed, _, _) => {
                    println!("Setting cursor to \"{:?}\"", cursors[cursor_idx]);
                    window.set_cursor(cursors[cursor_idx]);
                    if cursor_idx < cursors.len() - 1 {
                        cursor_idx += 1;
                    } else {
                        cursor_idx = 0;
                    }
                },
                glutin::WindowEvent::Closed => events_loop.interrupt(),
                _ => (),
            },
        }

        context.draw_frame((0.0, 1.0, 0.0, 1.0));
        window.swap_buffers().unwrap();
    });
}
