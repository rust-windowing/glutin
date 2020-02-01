pub use crate::api::osmesa::{OsMesaBuffer, OsMesaContext};
use crate::config::Version;
use crate::context::ContextBuilderWrapper;

use winit_types::error::Error;

impl<'a> OsMesaContextBuilder<'a> {
    /// Builds a [`OsMesaContext`].
    ///
    /// [`OsMesaContext`]: crate::platform::unix::osmesa::OsMesaContext
    #[inline]
    pub fn build(self, version: Version) -> Result<OsMesaContext, Error>
    where
        Self: Sized,
    {
        OsMesaContext::new(self, version)
    }
}

/// A simple type alias for [`ContextBuilderWrapper`]. Glutin clients should use
/// this type in their code, not [`ContextBuilderWrapper`]. If I had a choice,
/// I'd hide [`ContextBuilderWrapper`], but alas, due to limitations in rustdoc,
/// I cannot.
///
/// [`ContextBuilderWrapper`]: crate::context::ContextBuilderWrapper
pub type OsMesaContextBuilder<'a> = ContextBuilderWrapper<&'a OsMesaContext>;
