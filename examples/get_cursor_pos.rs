extern crate glutin;

fn main() {
    let window = glutin::WindowBuilder::new().build().unwrap();
    window.set_title("Press any key to display cursor position");

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