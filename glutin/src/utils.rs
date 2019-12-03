use std::fmt;
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone)]
pub(crate) struct NoPrint<T>(pub(crate) T);

impl<T> fmt::Debug for NoPrint<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoPrint(...)")
    }
}

impl<T> Deref for NoPrint<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoPrint<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
