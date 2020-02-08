use glutin::config::{Config, ConfigsFinder};
use glutin::context::{Context, ContextBuilder};
use glutin::surface::{Surface, SurfaceTypeTrait};

pub trait FailToCompileIfNotSendSync
where
    Self: Send + Sync,
{
}

impl<T: SurfaceTypeTrait> FailToCompileIfNotSendSync for Surface<T> {}
impl FailToCompileIfNotSendSync for Context {}
impl<'a> FailToCompileIfNotSendSync for ContextBuilder<'a> {}
impl FailToCompileIfNotSendSync for Config {}
impl FailToCompileIfNotSendSync for ConfigsFinder {}

pub trait FailToCompileIfNotClone
where
    Self: Clone,
{
}
impl FailToCompileIfNotClone for Config {}
impl FailToCompileIfNotClone for ConfigsFinder {}
impl<'a> FailToCompileIfNotClone for ContextBuilder<'a> {}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
mod unix {
    use crate::FailToCompileIfNotSendSync;
    use glutin::platform::unix::osmesa::{OsMesaBuffer, OsMesaContext};

    impl FailToCompileIfNotSendSync for OsMesaBuffer {}
    impl FailToCompileIfNotSendSync for OsMesaContext {}
}
