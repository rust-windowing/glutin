use std::error::Error;

use winit::event_loop::EventLoopBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    glutin_examples::main(EventLoopBuilder::new().build().unwrap())
}
