use std::fmt;
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone)]
pub(crate) struct NoPrint<T>(pub(crate) T);

impl<T> fmt::Debug for NoPrint<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoPrint(...)")
    }
}

impl<T> Deref for NoPrint<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoPrint<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct NoCmp<T>(pub(crate) T);

impl<T> PartialEq for NoCmp<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl<T> Eq for NoCmp<T> {}

impl<T> Deref for NoCmp<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoCmp<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
