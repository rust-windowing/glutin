use crate::platform_impl;

use winit_types::error::Error;
use glutin_interface::NativeDisplay;

#[derive(Debug)]
pub struct Display(pub(crate) platform_impl::Display);

#[derive(Debug, Clone)]
pub struct DisplayBuilder {
    /// Platform specific attributes
    pub plat_attr: platform_impl::DisplayPlatformAttributes,
}

impl DisplayBuilder {
    pub fn build<ND: NativeDisplay>(self, nd: &ND) -> Result<Display, Error> {
        platform_impl::Display::new(self, nd).map(Display)
    }
}
