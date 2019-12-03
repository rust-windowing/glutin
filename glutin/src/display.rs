use crate::platform_impl;

use glutin_winit_interface::NativeDisplaySource;
use winit_types::error::Error;

#[derive(Debug)]
pub struct Display(pub(crate) platform_impl::Display);

impl Display {
    pub fn new<NDS: NativeDisplaySource>(nds: &NDS) -> Result<Self, Error> {
        platform_impl::Display::new(nds).map(Display)
    }
}
