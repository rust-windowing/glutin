#[derive(Clone)]
pub struct Joystick;
impl Joystick {
    pub fn still_attached(&self) -> bool { unimplemented!() }
    pub fn poll(&self) -> Option<::joystick::Data> { unimplemented!() }
}

pub struct OneEvents<'a>(pub &'a Joystick);
impl<'a> Iterator for OneEvents<'a> {
    type Item = ::joystick::OneEvent;
    fn next(&mut self) -> Option<::joystick::OneEvent> {
        None
    }
}

#[derive(Clone)]
pub struct Joysticks;
impl Joysticks {
    pub fn scan(&self) -> Vec<::joystick::Joystick> { unimplemented!() }
}

pub struct Events<'a>(pub &'a Joysticks);
impl<'a> Iterator for Events<'a> {
    type Item = ::joystick::Event;
    fn next(&mut self) -> Option<::joystick::Event> {
        None
    }
}
