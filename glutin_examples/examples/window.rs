use std::error::Error;

use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn Error>> {
    glutin_examples::main(EventLoop::new().unwrap())
}
