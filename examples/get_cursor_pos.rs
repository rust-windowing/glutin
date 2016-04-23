extern crate glutin;

fn main() {
    let mut window = glutin::WindowBuilder::new().build().unwrap();
    window.set_title("A fantastic window!");
    let _ = unsafe { window.make_current() };

    for event in window.wait_events() {
        match event {
            glutin::Event::KeyboardInput(_, _, _) => {
                match window.get_cursor_position() {
                    Ok((x, y)) => println!("Cursor pos: {}, {}", x, y),
                    Err(())    => println!("Error retrieving cursor position."),
                }
            },
            glutin::Event::Closed => break,
            _ => ()
        }
    }
}