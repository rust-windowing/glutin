use super::*;
use winit::event_loop::EventLoopWindowTarget;

pub struct Display {
    display: platform_impl::Display,
}

impl Display {
    pub fn new<TE>(
        el: &EventLoopWindowTarget<TE>,
    ) -> Result<Self, CreationError> {
        platform_impl::Display::new(el)
            .map(|display| Display { display })
    }
}
