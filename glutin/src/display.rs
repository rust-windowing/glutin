use super::*;
use winit::event_loop::EventLoopWindowTarget;

#[derive(Debug)]
pub struct DisplayWrapper<T, TE> {
    pub(crate) display: T,
    pub(crate) el: TE,
}

pub type Display<TE> = DisplayWrapper<platform_impl::Display, TE>;

impl<TE> Display<EventLoopWindowTarget<TE>> {
    pub fn new(
        el: &EventLoopWindowTarget<TE>,
    ) -> Result<Self, CreationError> {
        platform_impl::Display::new(el).map(|display| Display { display, el: el.clone() })
    }
}

impl<T, TE> DisplayWrapper<T, TE> {
    /// Turns the `display` parameter into another type by calling a closure.
    #[inline]
    pub(crate) fn map_display<F, T2>(&self, f: F) -> DisplayWrapper<T2, TE>
    where
        F: FnOnce(T) -> T2,
    {
        DisplayWrapper {
            display: f(self.display),
            el: self.el,
        }
    }

    #[inline]
    pub(crate) fn as_ref<TE2>(self) -> DisplayWrapper<T, TE2>
    {
        DisplayWrapper {
            display: self.display,
            el: &self.el,
        }
    }
}
